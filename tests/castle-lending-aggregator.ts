import assert from "assert";
import { Program, utils } from '@project-serum/anchor';
import { TOKEN_PROGRAM_ID, Token } from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram, SYSVAR_CLOCK_PUBKEY, SYSVAR_RENT_PUBKEY, TransactionInstruction } from "@solana/web3.js";

import * as port from './helpers/port';
import { Solend } from './helpers/solend';
import { CastleLendingAggregator } from "../target/types/castle_lending_aggregator";

// Change to import after https://github.com/project-serum/anchor/issues/1153 is resolved
const anchor = require("@project-serum/anchor");

// TODO use SDK instead of raw code
// TODO use provider.wallet instead of owner
describe("castle-vault", () => {
    // Configure the client to use the local cluster.
    const provider = anchor.Provider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.CastleLendingAggregator as Program<CastleLendingAggregator>;

    const owner = anchor.web3.Keypair.generate();
    const vaultStateAccount = anchor.web3.Keypair.generate();
    const payer = anchor.web3.Keypair.generate();

    const solendCollateralMint = anchor.web3.Keypair.generate();
    const solendReserve = anchor.web3.Keypair.generate();
    const solendLiquiditySupply = anchor.web3.Keypair.generate();

    const solendProgramId = new PublicKey("So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo")
    const solendProgram = new Solend(provider, solendProgramId);

    const pythProduct = new PublicKey("3Mnn2fX6rQyUsyELYms1sBJyChWofzSNRoqYzvgMVz5E");
    const pythPrice = new PublicKey("J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix");
    const switchboardFeed = new PublicKey("GvDMxPzN1sCj7L26YDK2HnMRXEQmQ2aemov8YBtPS7vR");

    const initialReserveAmount = 100;

    let portReserveState: port.ReserveState;

    let reserveTokenMint: Token;
    let solendMarket: Keypair;
    let solendMarketAuthority: PublicKey;
    let portMarket: Keypair;
    let portMarketAuthority: PublicKey;

    before("Initialize lending markets", async () => {
        const sig = await provider.connection.requestAirdrop(payer.publicKey, 1000000000);
        await provider.connection.confirmTransaction(sig, "singleGossip");

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
            owner,
            port.DEFAULT_RESERVE_CONFIG,
        );
    });

    let vaultAuthority: PublicKey;
    let authorityBump: number;
    let vaultSolendLpTokenAccount: PublicKey;
    let solendLpBump: number;
    let vaultReserveTokenAccount: PublicKey;
    let reserveBump: number;
    let lpTokenMint: PublicKey;
    let lpTokenMintBump: number;

    it("Creates vault", async () => {
        [vaultAuthority, authorityBump] = await PublicKey.findProgramAddress(
            [vaultStateAccount.publicKey.toBuffer(), utils.bytes.utf8.encode("authority")],
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

        [lpTokenMint, lpTokenMintBump] = await PublicKey.findProgramAddress(
            [vaultStateAccount.publicKey.toBuffer(), utils.bytes.utf8.encode("lp_mint")],
            program.programId,
        );

        await program.rpc.initialize(
            {
                authority: authorityBump,
                reserve: reserveBump,
                lpMint: lpTokenMintBump,
                solendLp: solendLpBump,
            },
            {
                accounts: {
                    vault: vaultStateAccount.publicKey,
                    vaultAuthority: vaultAuthority,
                    lpTokenMint: lpTokenMint,
                    vaultReserveToken: vaultReserveTokenAccount,
                    vaultSolendLpToken: vaultSolendLpTokenAccount,
                    reserveTokenMint: reserveTokenMint.publicKey,
                    solendLpTokenMint: solendCollateralMint.publicKey,
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
        assert(actualVaultAccount.solendLpTokenMint.equals(solendCollateralMint.publicKey));
        assert(actualVaultAccount.lpTokenMint.equals(lpTokenMint));
        assert(actualVaultAccount.reserveTokenMint.equals(reserveTokenMint.publicKey));
    });

    let lpToken: Token;
    let userLpTokenAccount: PublicKey;
    let refreshInstruction: TransactionInstruction;

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

        refreshInstruction = program.instruction.refresh({
            accounts: {
                vault: vaultStateAccount.publicKey,
                vaultReserveToken: vaultReserveTokenAccount,
                vaultSolendLpToken: vaultSolendLpTokenAccount,
                solendProgram: solendProgramId,
                solendReserveState: solendReserve.publicKey,
                solendPyth: pythPrice,
                solendSwitchboard: switchboardFeed,
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
                instructions: [refreshInstruction],
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
                instructions: [refreshInstruction],
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

    it("Forwards deposits to lending program", async () => {
        const rebalanceInstruction = program.instruction.rebalance(
            new anchor.BN(0),
            {
                accounts: {
                    vault: vaultStateAccount.publicKey,
                    vaultReserveToken: vaultReserveTokenAccount,
                    solendProgram: solendProgramId,
                    solendReserveState: solendReserve.publicKey,
                }
            }
        );
        await program.rpc.reconcileSolend(
            {
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
                },
                instructions: [refreshInstruction, rebalanceInstruction],
            }
        );
        const vaultReserveTokenAccountInfo = await reserveTokenMint.getAccountInfo(vaultReserveTokenAccount);
        assert.equal(vaultReserveTokenAccountInfo.amount.toNumber(), 0);

        const solendCollateralToken = new Token(
            provider.connection,
            solendCollateralMint.publicKey,
            TOKEN_PROGRAM_ID,
            payer,
        );
        const vaultLpTokenAccountInfo = await solendCollateralToken.getAccountInfo(vaultSolendLpTokenAccount);
        assert.notEqual(vaultLpTokenAccountInfo.amount.toNumber(), 0);

        const liquiditySupplyAccountInfo = await reserveTokenMint.getAccountInfo(solendLiquiditySupply.publicKey);
        assert.equal(liquiditySupplyAccountInfo.amount.toNumber(), depositAmount - withdrawAmount + initialReserveAmount);
    });

    it("Rebalances", async () => {
    });

    it("Withdraws from lending programs", async () => {
    });
});