import assert from "assert";
import { Program } from '@project-serum/anchor';
import { AccountLayout, MintLayout, TOKEN_PROGRAM_ID, Token } from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram, SYSVAR_CLOCK_PUBKEY } from "@solana/web3.js";

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
            10,
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
        
        // TODO change to require initial deposit
        vaultReserveTokenAccount = await reserveTokenMint.createAccount(vaultAuthority);
        await reserveTokenMint.mintTo(vaultReserveTokenAccount, owner, [], 1000);

        await program.rpc.initializePool(
            {
                accounts: {
                    authority: vaultAuthority,
                    reservePool: vaultStateAccount.publicKey,
                    poolMint: lpTokenMint.publicKey,
                    token: vaultReserveTokenAccount,
                    destination: ownerLpTokenAccount,
                    tokenProgram: TOKEN_PROGRAM_ID,
                },
                signers: [vaultStateAccount],
                instructions: [await program.account.reservePool.createInstruction(vaultStateAccount)]
            }
        );

        const actualPoolAccount = await program.account.reservePool.fetch(vaultStateAccount.publicKey);
        assert(actualPoolAccount.tokenProgramId.equals(TOKEN_PROGRAM_ID));
        assert(actualPoolAccount.tokenAccount.equals(vaultReserveTokenAccount));
        assert(actualPoolAccount.tokenMint.equals(reserveTokenMint.publicKey));
        assert(actualPoolAccount.poolMint.equals(lpTokenMint.publicKey));

        const lpTokenMintInfo = await lpTokenMint.getMintInfo();
        assert.equal(lpTokenMintInfo.supply.toNumber(), 1000000);
    });

    let userLpTokenAccount: PublicKey;
    const depositAmount = 1000;
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

        await program.rpc.deposit(
            new anchor.BN(depositAmount),
            {
                accounts: {
                    reservePool: vaultStateAccount.publicKey,
                    authority: vaultAuthority,
                    userAuthority: userAuthority.publicKey,
                    source: userReserveTokenAccount,
                    destination: userLpTokenAccount,
                    token: vaultReserveTokenAccount,
                    poolMint: lpTokenMint.publicKey,
                    tokenProgram: TOKEN_PROGRAM_ID,
                },
                signers: [userAuthority],
            }
        );

        const userTokenAccountInfo = await reserveTokenMint.getAccountInfo(userReserveTokenAccount);
        assert.equal(userTokenAccountInfo.amount.toNumber(), 0);

        const tokenAccountInfo = await reserveTokenMint.getAccountInfo(vaultReserveTokenAccount);
        assert.equal(tokenAccountInfo.amount.toNumber(), 2000);

        const userPoolTokenAccountInfo = await lpTokenMint.getAccountInfo(userLpTokenAccount);
        assert.equal(userPoolTokenAccountInfo.amount.toNumber(), 1000000);

        const lpTokenMintInfo = await lpTokenMint.getMintInfo();
        assert.equal(lpTokenMintInfo.supply.toNumber(), 2000000);
    });

    it("Forwards deposits to lending program", async () => {
        const solendCollateralMintToken = new Token(provider.connection, solendCollateralMint.publicKey, TOKEN_PROGRAM_ID, payer);
        const vaultLpTokenAccount = await solendCollateralMintToken.createAccount(vaultAuthority);

        await program.rpc.rebalance(
            {
                accounts: {
                    vaultState: vaultStateAccount.publicKey,
                    vaultAuthority: vaultAuthority,
                    vaultReserveToken: vaultReserveTokenAccount,
                    vaultLpToken: vaultLpTokenAccount,
                    solendProgram: solendProgramId,
                    solendMarketAuthority: solendMarketAuthority,
                    solendMarket: solendMarket.publicKey,
                    solendReserveStateAccount: solendReserve.publicKey,
                    solendLpMintAccount: solendCollateralMint.publicKey,
                    solendDepositTokenAccount: solendLiquiditySupply.publicKey,
                    solendPyth: pythPrice,
                    solendSwitchboard: switchboardFeed,
                    clock: SYSVAR_CLOCK_PUBKEY,
                    tokenProgram: TOKEN_PROGRAM_ID,
                },
            }
        );
        const vaultReserveTokenAccountInfo = await reserveTokenMint.getAccountInfo(vaultReserveTokenAccount);
        assert.equal(vaultReserveTokenAccountInfo.amount.toNumber(), 0);

        const vaultLpTokenAccountInfo = await solendCollateralMintToken.getAccountInfo(vaultLpTokenAccount);
        assert.notEqual(vaultLpTokenAccountInfo.amount.toNumber(), 0);

        const liquiditySupplyAccountInfo = await reserveTokenMint.getAccountInfo(solendLiquiditySupply.publicKey);
        assert.equal(liquiditySupplyAccountInfo.amount.toNumber(), 2010);
    });

    it("Rebalances", async () => {
    });

    it("Withdraws from vault", async () => {
        // Pool tokens to withdraw from
        const withdrawAmount = 500000;

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
                    reservePool: vaultStateAccount.publicKey,
                    authority: vaultAuthority,
                    userAuthority: userAuthority.publicKey,
                    source: userLpTokenAccount,
                    token: vaultReserveTokenAccount,
                    destination: userReserveTokenAccount,
                    poolMint: lpTokenMint.publicKey,
                    tokenProgram: TOKEN_PROGRAM_ID,
                },
                signers: [userAuthority],
            }
        );

        const userReserveTokenAccountInfo = await reserveTokenMint.getAccountInfo(userReserveTokenAccount);
        assert.equal(userReserveTokenAccountInfo.amount.toNumber(), 500);

        const vaultReserveTokenAccountInfo = await reserveTokenMint.getAccountInfo(vaultReserveTokenAccount);
        assert.equal(vaultReserveTokenAccountInfo.amount.toNumber(), 1500);

        const userLpTokenAccountInfo = await lpTokenMint.getAccountInfo(userLpTokenAccount);
        assert.equal(userLpTokenAccountInfo.amount.toNumber(), withdrawAmount);

        const lpTokenMintInfo = await lpTokenMint.getMintInfo();
        assert.equal(lpTokenMintInfo.supply.toNumber(), 1500000);
    });
});