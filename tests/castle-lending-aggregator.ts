import assert from "assert";
import { Program, utils} from '@project-serum/anchor';
import { TOKEN_PROGRAM_ID, Token } from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram, SYSVAR_CLOCK_PUBKEY, SYSVAR_RENT_PUBKEY, TransactionInstruction } from "@solana/web3.js";

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

    let reserveTokenMint: Token;

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

    let vaultAuthority: PublicKey;
    let authorityBump: number;
    let lpTokenMint: PublicKey;
    let lpTokenMintBump: number;
    let vaultReserveTokenAccount: PublicKey;
    let reserveBump: number;
    let lpToken: Token;

    it("Creates vault", async () => {
        [vaultAuthority, authorityBump] = await PublicKey.findProgramAddress(
            [vaultStateAccount.publicKey.toBuffer(), utils.bytes.utf8.encode("authority")],
            program.programId,
        );

        [vaultReserveTokenAccount, reserveBump] = await PublicKey.findProgramAddress(
            [vaultStateAccount.publicKey.toBuffer(), reserveTokenMint.publicKey.toBuffer()],
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
            },
            {
                accounts: {
                    vaultAuthority: vaultAuthority,
                    payer: payer.publicKey,
                    vault: vaultStateAccount.publicKey,
                    reserveTokenMint: reserveTokenMint.publicKey,
                    lpTokenMint: lpTokenMint,
                    vaultReserveToken: vaultReserveTokenAccount,
                    systemProgram: SystemProgram.programId,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    rent: SYSVAR_RENT_PUBKEY,
                    clock: SYSVAR_CLOCK_PUBKEY,
                },
                signers: [vaultStateAccount, payer],
                instructions: [await program.account.vault.createInstruction(vaultStateAccount)]
            }
        );

        const actualPoolAccount = await program.account.vault.fetch(vaultStateAccount.publicKey);
        assert(actualPoolAccount.tokenProgram.equals(TOKEN_PROGRAM_ID));
        assert(actualPoolAccount.reserveTokenAccount.equals(vaultReserveTokenAccount));
        assert(actualPoolAccount.reserveTokenMint.equals(reserveTokenMint.publicKey));
        assert(actualPoolAccount.lpTokenMint.equals(lpTokenMint));
    });

    let userLpTokenAccount: PublicKey;
    let vaultLpTokenAccount: PublicKey;
    let refreshInstruction: TransactionInstruction;

    const solendCollateralMintToken = new Token(
        provider.connection, 
        solendCollateralMint.publicKey, 
        TOKEN_PROGRAM_ID, 
        payer
    );

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
        userLpTokenAccount = await lpToken.createAccount(owner.publicKey);
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
                    lpTokenMint: lpTokenMint,
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

        const userPoolTokenAccountInfo = await lpToken.getAccountInfo(userLpTokenAccount);
        assert.equal(userPoolTokenAccountInfo.amount.toNumber(), depositAmount * initialCollateralRatio);

        const lpTokenMintInfo = await lpToken.getMintInfo();
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

        const userLpTokenAccountInfo = await lpToken.getAccountInfo(userLpTokenAccount);
        assert.equal(
            userLpTokenAccountInfo.amount.toNumber(), 
            (depositAmount * initialCollateralRatio) - withdrawAmount
        );
    });
});