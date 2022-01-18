import { clusterApiUrl, Connection, Keypair, PublicKey, SystemProgram, SYSVAR_CLOCK_PUBKEY, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import { Program, Provider, Wallet, utils } from "@project-serum/anchor"
import { CastleLendingAggregator } from "../target/types/castle_lending_aggregator";
import { JetClient, JetReserve } from "@jet-lab/jet-engine";
import { SolendMarket } from "@solendprotocol/solend-sdk";
import { Token, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { Port, Environment } from "@port.finance/port-sdk";
import vaultIdl from "../target/idl/castle_lending_aggregator.json";

const VAULT_PROGRAM_ID = new PublicKey("6hSKFKsZvksTb4M7828LqWsquWnyatoRwgZbcpeyfWRb")

const main = async () => {
    const connection = new Connection(clusterApiUrl('devnet'));
    const wallet = Wallet.local();
    const provider = new Provider(connection, wallet, Provider.defaultOptions());
    const program = new Program<CastleLendingAggregator>(vaultIdl as CastleLendingAggregator, VAULT_PROGRAM_ID, provider);

    const solendMarket = await SolendMarket.initialize(provider.connection, "devnet");
    await solendMarket.loadReserves();
    const solendReserve = solendMarket.reserves.find(res => res.config.symbol === 'SOL');
    const solendCollateralMint = new PublicKey(solendReserve.config.collateralMintAddress);

    const port = new Port(connection, Environment.forMainNet(), new PublicKey("H27Quk3DSbu55T4dCr1NddTTSAezXwHU67FPCZVKLhSW"))
    const portReserve = await port.getReserve(new PublicKey("6FeVStQAGPWvfWijDHF7cTWRCi7He6vTT3ubfNhe9SPt"));

    const jetClient = await JetClient.connect(provider, true);
    const jetReserve = await JetReserve.load(jetClient, new PublicKey("BrXRUKDaSnxHwL46J4LSPiHnundhLzTfKRToh4s9jFbK"));

    const vaultStateAccount = Keypair.generate();
    const reserveTokenMint = new Token(
        connection,
        new PublicKey("So11111111111111111111111111111111111111112"),
        TOKEN_PROGRAM_ID,
        wallet.payer,
    );

    const [vaultAuthority, authorityBump] = await PublicKey.findProgramAddress(
        [vaultStateAccount.publicKey.toBuffer(), utils.bytes.utf8.encode("authority")],
        program.programId,
    );

    const [vaultReserveTokenAccount, reserveBump] = await PublicKey.findProgramAddress(
        [vaultStateAccount.publicKey.toBuffer(), reserveTokenMint.publicKey.toBuffer()],
        program.programId,
    );

    const [vaultSolendLpTokenAccount, solendLpBump] = await PublicKey.findProgramAddress(
        [vaultStateAccount.publicKey.toBuffer(), solendCollateralMint.toBuffer()],
        program.programId,
    );

    const [vaultPortLpTokenAccount, portLpBump] = await PublicKey.findProgramAddress(
        [vaultStateAccount.publicKey.toBuffer(), portReserve.getShareMintId().toBuffer()],
        program.programId,
    );

    const [vaultJetLpTokenAccount, jetLpBump] = await PublicKey.findProgramAddress(
        [vaultStateAccount.publicKey.toBuffer(), jetReserve.data.depositNoteMint.toBuffer()],
        program.programId,
    );

    const [lpTokenMint, lpTokenMintBump] = await PublicKey.findProgramAddress(
        [vaultStateAccount.publicKey.toBuffer(), utils.bytes.utf8.encode("lp_mint")],
        program.programId,
    );

    const txSig = await program.rpc.initialize(
        {
            authority: authorityBump,
            reserve: reserveBump,
            lpMint: lpTokenMintBump,
            solendLp: solendLpBump,
            portLp: portLpBump,
            jetLp: jetLpBump,
        },
        {
            accounts: {
                vault: vaultStateAccount.publicKey,
                vaultAuthority: vaultAuthority,
                lpTokenMint: lpTokenMint,
                vaultReserveToken: vaultReserveTokenAccount,
                vaultSolendLpToken: vaultSolendLpTokenAccount,
                vaultPortLpToken: vaultPortLpTokenAccount,
                vaultJetLpToken: vaultJetLpTokenAccount,
                reserveTokenMint: reserveTokenMint.publicKey,
                solendLpTokenMint: solendCollateralMint,
                portLpTokenMint: portReserve.getShareMintId(),
                jetLpTokenMint: jetReserve.data.depositNoteMint,
                payer: wallet.publicKey,
                systemProgram: SystemProgram.programId,
                tokenProgram: TOKEN_PROGRAM_ID,
                rent: SYSVAR_RENT_PUBKEY,
                clock: SYSVAR_CLOCK_PUBKEY,
            },
            signers: [vaultStateAccount],
            instructions: [await program.account.vault.createInstruction(vaultStateAccount)]
        }
    );
    console.log(txSig);
}

main();