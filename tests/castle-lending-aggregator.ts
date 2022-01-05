import assert from "assert";
import { Program } from '@project-serum/anchor';
import { TOKEN_PROGRAM_ID, Token } from "@solana/spl-token";
import { Keypair, PublicKey, SYSVAR_CLOCK_PUBKEY, TransactionInstruction } from "@solana/web3.js";

import { Solend } from './helpers/solend';
import { CastleLendingAggregator } from "../target/types/castle_lending_aggregator";

// Change to import after https://github.com/project-serum/anchor/issues/1153 is resolved
const anchor = require("@project-serum/anchor");

/// TODO use SDK instead of raw code
describe("castle-vault", () => {
    // Configure the client to use the local cluster.
    const provider = anchor.Provider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.CastleLendingAggregator as Program<CastleLendingAggregator>;

    let vaultAuthority: PublicKey;
    let lpTokenMint: Token;
    let reserveTokenMint: Token;
    let vaultReserveTokenAccount: PublicKey;

    const owner = anchor.web3.Keypair.generate();
    const vaultStateAccount = anchor.web3.Keypair.generate();
    const payer = anchor.web3.Keypair.generate();

    let solendMarket: Keypair;
    let solendMarketAuthority: PublicKey;

    const solendProgramId = new PublicKey("BwTGCAdzPncEFqP5JBAeCLRWKE8MDVvbGDVMD7XX2fvu") 
    const solendProgram = new Solend(provider, solendProgramId);

    const solendCollateralMint = anchor.web3.Keypair.generate();
    const solendReserve = anchor.web3.Keypair.generate();
    const solendLiquiditySupply = anchor.web3.Keypair.generate();

    const pythProduct = new PublicKey("3Mnn2fX6rQyUsyELYms1sBJyChWofzSNRoqYzvgMVz5E");
    const pythPrice = new PublicKey("J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix");
    const switchboardFeed = new PublicKey("GvDMxPzN1sCj7L26YDK2HnMRXEQmQ2aemov8YBtPS7vR");

    const solendInitialReserveAmount = 100;
    before(async () => {
        const sig  = await provider.connection.requestAirdrop(payer.publicKey, 1000000000);
        await provider.connection.confirmTransaction(sig, "singleGossip");

        solendMarket = await solendProgram.initLendingMarket(
            owner.publicKey,
            payer,
            new PublicKey("gSbePebfvPy7tRqimPoVecS2UsBvYv46ynrzWocc92s"),
            new PublicKey("2TfB33aLaneQb5TNVwyDz3jSZXS6jdW2ARw1Dgf84XCG"),
        );

        reserveTokenMint = await Token.createMint(
            provider.connection,
            payer,
            owner.publicKey,
            null,
            2,
            TOKEN_PROGRAM_ID
        );

        [solendMarketAuthority, ] = await PublicKey.findProgramAddress(
            [solendMarket.publicKey.toBuffer()],
            solendProgramId,
        );

        await solendProgram.addReserve(
            solendInitialReserveAmount,
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
    });

    const initialReserveAmount = 1000;
    it("Creates vault", async () => {
        [vaultAuthority, ] = await PublicKey.findProgramAddress(
            [vaultStateAccount.publicKey.toBuffer()],
            program.programId,
        )
        lpTokenMint = await Token.createMint(
            provider.connection,
            payer,
            vaultAuthority,
            null,
            2,
            TOKEN_PROGRAM_ID
        );
        const ownerLpTokenAccount = await lpTokenMint.createAccount(owner.publicKey);
        const ownerReserveTokenAccount = await reserveTokenMint.createAccount(owner.publicKey);
        await reserveTokenMint.mintTo(ownerReserveTokenAccount, owner, [], initialReserveAmount);

        vaultReserveTokenAccount = await reserveTokenMint.createAccount(vaultAuthority);

        await program.rpc.initialize(
            new anchor.BN(initialReserveAmount),
            {
                accounts: {
                    vaultAuthority: vaultAuthority,
                    userAuthority: owner.publicKey,
                    vault: vaultStateAccount.publicKey,
                    lpTokenMint: lpTokenMint.publicKey,
                    vaultReserveToken: vaultReserveTokenAccount,
                    userReserveToken: ownerReserveTokenAccount,
                    userLpToken: ownerLpTokenAccount,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    clock: SYSVAR_CLOCK_PUBKEY,
                },
                signers: [vaultStateAccount, owner],
                instructions: [await program.account.vault.createInstruction(vaultStateAccount)]
            }
        );

        const actualPoolAccount = await program.account.vault.fetch(vaultStateAccount.publicKey);
        assert(actualPoolAccount.tokenProgram.equals(TOKEN_PROGRAM_ID));
        assert(actualPoolAccount.reserveTokenAccount.equals(vaultReserveTokenAccount));
        assert(actualPoolAccount.reserveTokenMint.equals(reserveTokenMint.publicKey));
        assert(actualPoolAccount.lpTokenMint.equals(lpTokenMint.publicKey));

        const lpTokenMintInfo = await lpTokenMint.getMintInfo();
        assert.equal(lpTokenMintInfo.supply.toNumber(), initialReserveAmount);
    });

    let userLpTokenAccount: PublicKey;
    let vaultLpTokenAccount: PublicKey;
    let refreshInstruction: TransactionInstruction;

    const solendCollateralMintToken = new Token(provider.connection, solendCollateralMint.publicKey, TOKEN_PROGRAM_ID, payer);

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

        // Create depositor pool LP token account
        userLpTokenAccount = await lpTokenMint.createAccount(owner.publicKey);
        vaultLpTokenAccount = await solendCollateralMintToken.createAccount(vaultAuthority);

        refreshInstruction = program.instruction.refresh({
            accounts: {
                vault: vaultStateAccount.publicKey,
                vaultReserveToken: vaultReserveTokenAccount,
                vaultSolendLpToken: vaultLpTokenAccount,
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
                    userAuthority: userAuthority.publicKey,
                    userReserveToken: userReserveTokenAccount,
                    userLpToken: userLpTokenAccount,
                    vaultReserveToken: vaultReserveTokenAccount,
                    lpTokenMint: lpTokenMint.publicKey,
                    tokenProgram: TOKEN_PROGRAM_ID,
                },
                signers: [userAuthority],
                instructions: [refreshInstruction],
            }
        );

        const userTokenAccountInfo = await reserveTokenMint.getAccountInfo(userReserveTokenAccount);
        assert.equal(userTokenAccountInfo.amount.toNumber(), 0);

        const tokenAccountInfo = await reserveTokenMint.getAccountInfo(vaultReserveTokenAccount);
        assert.equal(tokenAccountInfo.amount.toNumber(), initialReserveAmount + depositAmount);

        const userPoolTokenAccountInfo = await lpTokenMint.getAccountInfo(userLpTokenAccount);
        assert.equal(userPoolTokenAccountInfo.amount.toNumber(), depositAmount * initialCollateralRatio);

        const lpTokenMintInfo = await lpTokenMint.getMintInfo();
        assert.equal(lpTokenMintInfo.supply.toNumber(), (initialReserveAmount + depositAmount) * initialCollateralRatio);
    });

    it("Forwards deposits to lending program", async () => {
        await program.rpc.rebalance(
            {
                accounts: {
                    vault: vaultStateAccount.publicKey,
                    vaultAuthority: vaultAuthority,
                    vaultReserveToken: vaultReserveTokenAccount,
                    vaultSolendLpToken: vaultLpTokenAccount,
                    solendProgram: solendProgramId,
                    solendMarketAuthority: solendMarketAuthority,
                    solendMarket: solendMarket.publicKey,
                    solendReserveState: solendReserve.publicKey,
                    solendLpMint: solendCollateralMint.publicKey,
                    solendReserveToken: solendLiquiditySupply.publicKey,
                    clock: SYSVAR_CLOCK_PUBKEY,
                    tokenProgram: TOKEN_PROGRAM_ID,
                },
                instructions: [refreshInstruction],
            }
        );
        const vaultReserveTokenAccountInfo = await reserveTokenMint.getAccountInfo(vaultReserveTokenAccount);
        assert.equal(vaultReserveTokenAccountInfo.amount.toNumber(), 0);

        const vaultLpTokenAccountInfo = await solendCollateralMintToken.getAccountInfo(vaultLpTokenAccount);
        assert.notEqual(vaultLpTokenAccountInfo.amount.toNumber(), 0);

        const liquiditySupplyAccountInfo = await reserveTokenMint.getAccountInfo(solendLiquiditySupply.publicKey);
        assert.equal(liquiditySupplyAccountInfo.amount.toNumber(), initialReserveAmount + depositAmount + solendInitialReserveAmount);
    });

    it("Rebalances", async () => {
    });

    it("Withdraws from vault", async () => {
        // Pool tokens to withdraw from
        const withdrawAmount = 500;

        // Create token account to withdraw into
        const userReserveTokenAccount = await reserveTokenMint.createAccount(owner.publicKey);

        // Delegate authority to transfer pool tokens
        const userAuthority = anchor.web3.Keypair.generate();
        await lpTokenMint.approve(
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
                    vaultLpMint: lpTokenMint.publicKey,
                    vaultSolendLpToken: vaultLpTokenAccount,
                    solendProgram: solendProgramId,
                    solendMarketAuthority: solendMarketAuthority,
                    solendMarket: solendMarket.publicKey,
                    solendReserveState: solendReserve.publicKey,
                    solendLpMint: solendCollateralMint.publicKey,
                    solendReserveToken: solendLiquiditySupply.publicKey,
                    clock: SYSVAR_CLOCK_PUBKEY,
                    tokenProgram: TOKEN_PROGRAM_ID,
                },
                signers: [userAuthority],
                instructions: [refreshInstruction],
            }
        );

        const userReserveTokenAccountInfo = await reserveTokenMint.getAccountInfo(userReserveTokenAccount);
        assert.equal(userReserveTokenAccountInfo.amount.toNumber(), withdrawAmount);

        const userLpTokenAccountInfo = await lpTokenMint.getAccountInfo(userLpTokenAccount);
        assert.equal(
            userLpTokenAccountInfo.amount.toNumber(), 
            (depositAmount * initialCollateralRatio) - withdrawAmount
        );
    });
});