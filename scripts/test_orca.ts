import { Provider, Wallet } from "@project-serum/anchor";
import {
    Cluster,
    Keypair,
    PublicKey,
    SystemProgram,
    Transaction,
    TransactionInstruction,
    Connection,
    LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, Token, AccountLayout } from "@solana/spl-token";

import { OrcaLegacySwap } from "../sdk/src/dex";
import { OrcaPoolConfig } from "@orca-so/sdk";
import { OrcaPoolParams } from "@orca-so/sdk/dist/model/orca/pool/pool-types";
import { orcaPoolConfigs } from "@orca-so/sdk/dist/constants/pools";

// TODO make this a CLI
const main = async () => {

    const tokenA = new PublicKey(
        "PoRTjZMPXb9T7dyU7tpLEZRQj7e6ssfAE62j2oQuc6y" // Port token
    );
    const tokenB = new PublicKey(
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" // USDC
    );

    const tokenPairToOrcaLegacyPool = Object.fromEntries(
        Object.values(OrcaPoolConfig).map((v) => {
            const params = orcaPoolConfigs[v];
            const tokens = Object.keys(params.tokens);
            return [tokens[0].toString() + tokens[1].toString(), params];
        })
    );

    const tokenPairSig = tokenA.toString() + tokenB.toString();
    const params: OrcaPoolParams = tokenPairToOrcaLegacyPool[tokenPairSig];
    if (params == undefined) {
        throw new Error("Token pair not supported");
    }
    console.log(params);

    //
    //// Create a pool on devnet
    //
    const connection = new Connection("https://api.devnet.solana.com");

    const wallet = Keypair.generate();
    const sig0 = await connection.requestAirdrop(
        wallet.publicKey,
        LAMPORTS_PER_SOL
    );
    await connection.confirmTransaction(sig0, "finalized");

    const tokenMintA = await Token.createMint(
        connection,
        wallet,
        wallet.publicKey,
        null,
        6,
        TOKEN_PROGRAM_ID
    );
    const tokenMintB = await Token.createMint(
        connection,
        wallet,
        wallet.publicKey,
        null,
        6,
        TOKEN_PROGRAM_ID
    );

    const provider = new Provider(connection, new Wallet(wallet), {
        commitment: "confirmed",
    });

    const orcaAccounts = await OrcaLegacySwap.initialize(
        provider,
        wallet,
        tokenMintA,
        tokenMintB,
        wallet,
        wallet
    );
};

main();
