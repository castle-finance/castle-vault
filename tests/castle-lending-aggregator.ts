import assert from "assert";
import * as anchor from '@project-serum/anchor';
import { TOKEN_PROGRAM_ID, Token } from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram, SYSVAR_CLOCK_PUBKEY, SYSVAR_RENT_PUBKEY, TransactionInstruction } from "@solana/web3.js";

import * as jet from './helpers/jet';
import * as port from './helpers/port';
import { Solend } from './helpers/solend';
import { CastleLendingAggregator } from "../target/types/castle_lending_aggregator";
import { JetMarket } from "@jet-lab/jet-engine";

// TODO use SDK instead of raw code
// TODO use provider.wallet instead of owner
describe("castle-vault", () => {
    // Configure the client to use the local cluster.
    const provider = anchor.Provider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.CastleLendingAggregator as anchor.Program<CastleLendingAggregator>;

    const owner = anchor.web3.Keypair.generate();
    const vaultStateAccount = anchor.web3.Keypair.generate();
    const payer = anchor.web3.Keypair.generate();

    const solendCollateralMint = anchor.web3.Keypair.generate();
    const solendReserve = anchor.web3.Keypair.generate();
    const solendLiquiditySupply = anchor.web3.Keypair.generate();

    // TODO change to devnet version
    const solendProgramId = new PublicKey("ALend7Ketfx5bxh6ghsCDXAoDrhvEmsXT3cynB6aPLgx")
    const solendProgram = new Solend(provider, solendProgramId);

    const pythProduct = new PublicKey("3Mnn2fX6rQyUsyELYms1sBJyChWofzSNRoqYzvgMVz5E");
    const pythPrice = new PublicKey("J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix");
    const switchboardFeed = new PublicKey("GvDMxPzN1sCj7L26YDK2HnMRXEQmQ2aemov8YBtPS7vR");

    const jetProgram = new PublicKey("JPv1rCqrhagNNmJVM5J1he7msQ5ybtvE1nNuHpDHMNU");

    const initialReserveAmount = 100;

    let portReserveState: port.ReserveState;
    let jetReserveAccounts: jet.ReserveAccounts;

    let reserveTokenMint: Token;
    let quoteTokenMint: Token;

    let solendMarket: Keypair;
    let solendMarketAuthority: PublicKey;
    let portMarket: Keypair;
    let portMarketAuthority: PublicKey;
    let jetMarket: JetMarket;
    let jetMarketAuthority: PublicKey;

    before("Initialize lending markets", async () => {
        const sig = await provider.connection.requestAirdrop(payer.publicKey, 1000000000);
        await provider.connection.confirmTransaction(sig, "singleGossip");

        quoteTokenMint = await Token.createMint(
            provider.connection,
            payer,
            owner.publicKey,
            null,
            2,
            TOKEN_PROGRAM_ID
        );


        reserveTokenMint = await Token.createMint(
            provider.connection,
            payer,
            owner.publicKey,
            null,
            2,
            TOKEN_PROGRAM_ID
        );

        const ownerReserveTokenAccount = await reserveTokenMint.createAccount(owner.publicKey);
        await reserveTokenMint.mintTo(ownerReserveTokenAccount, owner, [], 3 * initialReserveAmount);

        solendMarket = await solendProgram.initLendingMarket(
            owner.publicKey,
            payer,
            new PublicKey("gSbePebfvPy7tRqimPoVecS2UsBvYv46ynrzWocc92s"),
            new PublicKey("2TfB33aLaneQb5TNVwyDz3jSZXS6jdW2ARw1Dgf84XCG"),
        );

        [solendMarketAuthority,] = await PublicKey.findProgramAddress(
            [solendMarket.publicKey.toBuffer()],
            solendProgramId,
        );

        await solendProgram.addReserve(
            initialReserveAmount,
            ownerReserveTokenAccount,
            owner,
            payer,
            reserveTokenMint,
            solendReserve,
            solendCollateralMint,
            solendLiquiditySupply,
            pythProduct,
            pythPrice,
            switchboardFeed,
            solendMarket.publicKey,
            solendMarketAuthority,
        );

        console.log("Initialized Solend");

        portMarket = await port.createLendingMarket(provider);

        [portMarketAuthority,] = await PublicKey.findProgramAddress(
            [portMarket.publicKey.toBuffer()],
            port.PORT_LENDING,
        );

        portReserveState = await port.createDefaultReserve(
            provider,
            initialReserveAmount,
            ownerReserveTokenAccount,
            portMarket.publicKey,
            pythPrice,
            owner,
        );

        console.log("Initialized Port");

        jetMarket = await jet.createLendingMarket(provider, quoteTokenMint.publicKey);
        jetMarketAuthority = await jet.getMarketAuthority(jetMarket.address);
        jetReserveAccounts = await jet.initReserve(
            provider,
            jetMarket.address,
            provider.wallet.publicKey,
            quoteTokenMint.publicKey,
            reserveTokenMint,
            TOKEN_PROGRAM_ID, // dummy dex market addr
            pythPrice,
            pythProduct,
        )

        console.log("Initialized Jet");
    });

    let vaultAuthority: PublicKey;
    let authorityBump: number;
    let vaultSolendLpTokenAccount: PublicKey;
    let solendLpBump: number;
    let vaultPortLpTokenAccount: PublicKey;
    let portLpBump: number;
    let vaultJetLpTokenAccount: PublicKey;
    let jetLpBump: number;
    let vaultReserveTokenAccount: PublicKey;
    let reserveBump: number;
    let lpTokenMint: PublicKey;
    let lpTokenMintBump: number;

    it("Creates vault", async () => {
        [vaultAuthority, authorityBump] = await PublicKey.findProgramAddress(
            [vaultStateAccount.publicKey.toBuffer(), anchor.utils.bytes.utf8.encode("authority")],
            program.programId,
        );

        [vaultReserveTokenAccount, reserveBump] = await PublicKey.findProgramAddress(
            [vaultStateAccount.publicKey.toBuffer(), reserveTokenMint.publicKey.toBuffer()],
            program.programId,
        );

        [vaultSolendLpTokenAccount, solendLpBump] = await PublicKey.findProgramAddress(
            [vaultStateAccount.publicKey.toBuffer(), solendCollateralMint.publicKey.toBuffer()],
            program.programId,
        );

        [vaultPortLpTokenAccount, portLpBump] = await PublicKey.findProgramAddress(
            [vaultStateAccount.publicKey.toBuffer(), portReserveState.collateralMintAccount.toBuffer()],
            program.programId,
        );

        [vaultJetLpTokenAccount, jetLpBump] = await PublicKey.findProgramAddress(
            [vaultStateAccount.publicKey.toBuffer(), jetReserveAccounts.accounts.depositNoteMint.toBuffer()],
            program.programId,
        );

        [lpTokenMint, lpTokenMintBump] = await PublicKey.findProgramAddress(
            [vaultStateAccount.publicKey.toBuffer(), anchor.utils.bytes.utf8.encode("lp_mint")],
            program.programId,
        );

        await program.rpc.initialize(
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
                    solendLpTokenMint: solendCollateralMint.publicKey,
                    portLpTokenMint: portReserveState.collateralMintAccount,
                    jetLpTokenMint: jetReserveAccounts.accounts.depositNoteMint,
                    payer: payer.publicKey,
                    systemProgram: SystemProgram.programId,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    rent: SYSVAR_RENT_PUBKEY,
                    clock: SYSVAR_CLOCK_PUBKEY,
                },
                signers: [vaultStateAccount, payer],
                instructions: [await program.account.vault.createInstruction(vaultStateAccount)]
            }
        );

        const actualVaultAccount = await program.account.vault.fetch(vaultStateAccount.publicKey);
        assert(actualVaultAccount.vaultAuthority.equals(vaultAuthority));
        assert(actualVaultAccount.vaultReserveToken.equals(vaultReserveTokenAccount));
        assert(actualVaultAccount.vaultSolendLpToken.equals(vaultSolendLpTokenAccount));
        assert(actualVaultAccount.lpTokenMint.equals(lpTokenMint));
        assert(actualVaultAccount.reserveTokenMint.equals(reserveTokenMint.publicKey));
    });

    let lpToken: Token;
    let userLpTokenAccount: PublicKey;
    let refreshIx: anchor.web3.TransactionInstruction;

    const depositAmount = 1000;
    const initialCollateralRatio = 1.0;

    it("Deposits to vault reserves", async () => {
        // Create depositor token account
        const userAuthority = anchor.web3.Keypair.generate();
        const userReserveTokenAccount = await reserveTokenMint.createAccount(owner.publicKey);
        await reserveTokenMint.mintTo(userReserveTokenAccount, owner, [], depositAmount);
        await reserveTokenMint.approve(
            userReserveTokenAccount,
            userAuthority.publicKey,
            owner,
            [],
            depositAmount,
        );

        lpToken = new Token(
            provider.connection,
            lpTokenMint,
            TOKEN_PROGRAM_ID,
            payer,
        );
        userLpTokenAccount = await lpToken.createAccount(owner.publicKey);

        refreshIx = program.instruction.refresh({
            accounts: {
                vault: vaultStateAccount.publicKey,
                vaultReserveToken: vaultReserveTokenAccount,
                vaultSolendLpToken: vaultSolendLpTokenAccount,
                vaultPortLpToken: vaultPortLpTokenAccount,
                vaultJetLpToken: vaultJetLpTokenAccount,
                solendProgram: solendProgramId,
                solendReserveState: solendReserve.publicKey,
                solendPyth: pythPrice,
                solendSwitchboard: switchboardFeed,
                portProgram: port.PORT_LENDING,
                portReserveState: portReserveState.address,
                portOracle: portReserveState.oracle,
                jetProgram: jetProgram,
                jetMarket: jetMarket.address,
                jetMarketAuthority: jetMarket.marketAuthority,
                jetReserveState: jetReserveAccounts.accounts.reserve.publicKey,
                jetFeeNoteVault: jetReserveAccounts.accounts.feeNoteVault,
                jetDepositNoteMint: jetReserveAccounts.accounts.depositNoteMint,
                jetPyth: jetReserveAccounts.accounts.pythPrice,
                tokenProgram: TOKEN_PROGRAM_ID,
                clock: SYSVAR_CLOCK_PUBKEY,
            }
        });

        await program.rpc.deposit(
            new anchor.BN(depositAmount),
            {
                accounts: {
                    vault: vaultStateAccount.publicKey,
                    vaultAuthority: vaultAuthority,
                    vaultReserveToken: vaultReserveTokenAccount,
                    lpTokenMint: lpTokenMint,
                    userReserveToken: userReserveTokenAccount,
                    userLpToken: userLpTokenAccount,
                    userAuthority: userAuthority.publicKey,
                    tokenProgram: TOKEN_PROGRAM_ID,
                },
                signers: [userAuthority],
                instructions: [refreshIx],
            }
        );

        const userTokenAccountInfo = await reserveTokenMint.getAccountInfo(userReserveTokenAccount);
        assert.equal(userTokenAccountInfo.amount.toNumber(), 0);

        const tokenAccountInfo = await reserveTokenMint.getAccountInfo(vaultReserveTokenAccount);
        assert.equal(tokenAccountInfo.amount.toNumber(), depositAmount);

        const userPoolTokenAccountInfo = await lpToken.getAccountInfo(userLpTokenAccount);
        assert.equal(userPoolTokenAccountInfo.amount.toNumber(), depositAmount * initialCollateralRatio);

        const lpTokenMintInfo = await lpToken.getMintInfo();
        assert.equal(lpTokenMintInfo.supply.toNumber(), depositAmount * initialCollateralRatio);
    });

    const withdrawAmount = 500;
    it("Withdraws from vault reserves", async () => {
        // Create token account to withdraw into
        const userReserveTokenAccount = await reserveTokenMint.createAccount(owner.publicKey);

        // Delegate authority to transfer pool tokens
        const userAuthority = anchor.web3.Keypair.generate();
        await lpToken.approve(
            userLpTokenAccount,
            userAuthority.publicKey,
            owner,
            [],
            withdrawAmount,
        );

        await program.rpc.withdraw(
            new anchor.BN(withdrawAmount),
            {
                accounts: {
                    vault: vaultStateAccount.publicKey,
                    vaultAuthority: vaultAuthority,
                    userAuthority: userAuthority.publicKey,
                    userLpToken: userLpTokenAccount,
                    userReserveToken: userReserveTokenAccount,
                    vaultReserveToken: vaultReserveTokenAccount,
                    vaultLpMint: lpTokenMint,
                    tokenProgram: TOKEN_PROGRAM_ID,
                },
                signers: [userAuthority],
                instructions: [refreshIx],
            }
        );

        const userReserveTokenAccountInfo = await reserveTokenMint.getAccountInfo(userReserveTokenAccount);
        assert.equal(userReserveTokenAccountInfo.amount.toNumber(), withdrawAmount);

        const userLpTokenAccountInfo = await lpToken.getAccountInfo(userLpTokenAccount);
        assert.equal(
            userLpTokenAccountInfo.amount.toNumber(),
            (depositAmount * initialCollateralRatio) - withdrawAmount
        );
    });

    let reconcileSolendIx: TransactionInstruction;
    let reconcilePortIx: TransactionInstruction;
    let reconcileJetIx: TransactionInstruction;
    it("Forwards deposits to lending markets", async () => {
        const tx = new anchor.web3.Transaction();
        tx.add(refreshIx);
        tx.add(program.instruction.rebalance(new anchor.BN(0), {
            accounts: {
                vault: vaultStateAccount.publicKey,
                vaultReserveToken: vaultReserveTokenAccount,
                vaultSolendLpToken: vaultSolendLpTokenAccount,
                vaultPortLpToken: vaultPortLpTokenAccount,
                vaultJetLpToken: vaultJetLpTokenAccount,
                solendReserveState: solendReserve.publicKey,
                portReserveState: portReserveState.address,
                jetReserveState: jetReserveAccounts.accounts.reserve.publicKey,
            }
        }));
        reconcileSolendIx = program.instruction.reconcileSolend({
            accounts: {
                vault: vaultStateAccount.publicKey,
                vaultAuthority: vaultAuthority,
                vaultReserveToken: vaultReserveTokenAccount,
                vaultSolendLpToken: vaultSolendLpTokenAccount,
                solendProgram: solendProgramId,
                solendMarketAuthority: solendMarketAuthority,
                solendMarket: solendMarket.publicKey,
                solendReserveState: solendReserve.publicKey,
                solendLpMint: solendCollateralMint.publicKey,
                solendReserveToken: solendLiquiditySupply.publicKey,
                clock: SYSVAR_CLOCK_PUBKEY,
                tokenProgram: TOKEN_PROGRAM_ID,
            }
        });
        tx.add(reconcileSolendIx);

        reconcilePortIx = program.instruction.reconcilePort({
            accounts: {
                vault: vaultStateAccount.publicKey,
                vaultAuthority: vaultAuthority,
                vaultReserveToken: vaultReserveTokenAccount,
                vaultPortLpToken: vaultPortLpTokenAccount,
                portProgram: port.PORT_LENDING,
                portMarketAuthority: portMarketAuthority,
                portMarket: portMarket.publicKey,
                portReserveState: portReserveState.address,
                portLpMint: portReserveState.collateralMintAccount,
                portReserveToken: portReserveState.liquiditySupplyPubkey,
                clock: SYSVAR_CLOCK_PUBKEY,
                tokenProgram: TOKEN_PROGRAM_ID,
            }
        });
        tx.add(reconcilePortIx);

        reconcileJetIx = program.instruction.reconcileJet({
            accounts: {
                vault: vaultStateAccount.publicKey,
                vaultAuthority: vaultAuthority,
                vaultReserveToken: vaultReserveTokenAccount,
                vaultJetLpToken: vaultJetLpTokenAccount,
                jetProgram: jet.PROGRAM_ID,
                jetMarket: jetMarket.address,
                jetMarketAuthority: jetMarketAuthority,
                jetReserveState: jetReserveAccounts.accounts.reserve.publicKey,
                jetReserveToken: jetReserveAccounts.accounts.vault,
                jetLpMint: jetReserveAccounts.accounts.depositNoteMint,
                tokenProgram: TOKEN_PROGRAM_ID,
            }
        });
        tx.add(reconcileJetIx);

        await provider.send(tx);

        const vaultReserveTokenAccountInfo = await reserveTokenMint.getAccountInfo(vaultReserveTokenAccount);
        assert.equal(vaultReserveTokenAccountInfo.amount.toNumber(), 2);

        const solendCollateralRatio = 1;
        const solendAllocation = 0.332;
        const solendCollateralToken = new Token(
            provider.connection,
            solendCollateralMint.publicKey,
            TOKEN_PROGRAM_ID,
            payer,
        );
        const vaultSolendLpTokenAccountInfo = await solendCollateralToken.getAccountInfo(vaultSolendLpTokenAccount);
        assert.equal(
            vaultSolendLpTokenAccountInfo.amount.toNumber(),
            ((depositAmount - withdrawAmount) * solendAllocation) * solendCollateralRatio
        );
        const solendLiquiditySupplyAccountInfo = await reserveTokenMint.getAccountInfo(solendLiquiditySupply.publicKey);
        assert.equal(
            solendLiquiditySupplyAccountInfo.amount.toNumber(),
            ((depositAmount - withdrawAmount) * solendAllocation) + initialReserveAmount
        );

        const portCollateralRatio = 1;
        const portAllocation = 0.332;
        const portCollateralToken = new Token(
            provider.connection,
            portReserveState.collateralMintAccount,
            TOKEN_PROGRAM_ID,
            payer,
        );
        const vaultPortLpTokenAccountInfo = await portCollateralToken.getAccountInfo(vaultPortLpTokenAccount);
        assert.equal(
            vaultPortLpTokenAccountInfo.amount.toNumber(),
            ((depositAmount - withdrawAmount) * portAllocation) * portCollateralRatio
        );
        const portLiquiditySupplyAccountInfo = await reserveTokenMint.getAccountInfo(portReserveState.liquiditySupplyPubkey);
        assert.equal(
            portLiquiditySupplyAccountInfo.amount.toNumber(),
            ((depositAmount - withdrawAmount) * portAllocation) + initialReserveAmount
        );

        const jetCollateralRatio = 1;
        const jetAllocation = 0.332;
        const jetCollateralToken = new Token(
            provider.connection,
            jetReserveAccounts.accounts.depositNoteMint,
            TOKEN_PROGRAM_ID,
            payer,
        );
        const vaultJetLpTokenAccountInfo = await jetCollateralToken.getAccountInfo(vaultJetLpTokenAccount);
        assert.equal(
            vaultJetLpTokenAccountInfo.amount.toNumber(),
            ((depositAmount - withdrawAmount) * jetAllocation) * jetCollateralRatio
        );

        const jetLiquiditySupplyAccountInfo = await reserveTokenMint.getAccountInfo(jetReserveAccounts.accounts.vault);
        assert.equal(
            jetLiquiditySupplyAccountInfo.amount.toNumber(),
            ((depositAmount - withdrawAmount) * jetAllocation)
        );
    });

    it("Rebalances", async () => {
        // TODO
    });

    it("Withdraws from lending programs", async () => {
        // TODO
    });
});