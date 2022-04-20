import { assert } from "chai";
import * as anchor from "@project-serum/anchor";
import {
    TOKEN_PROGRAM_ID,
    Token,
    NATIVE_MINT,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    AccountLayout,
} from "@solana/spl-token";
import {
    Keypair,
    LAMPORTS_PER_SOL,
    PublicKey,
    SystemProgram,
    Transaction,
} from "@solana/web3.js";

import {
    SolendReserveAsset,
    JetReserveAsset,
    PortReserveAsset,
    VaultClient,
    CastleLendingAggregator,
    StrategyType,
    createTokenSyncNativeInstruction,
} from "../sdk/src/index";

describe("castle-vault", () => {
    const provider = anchor.Provider.env();
    anchor.setProvider(provider);
    const wallet = provider.wallet as anchor.Wallet;

    const program = anchor.workspace
        .CastleLendingAggregator as anchor.Program<CastleLendingAggregator>;

    const owner = Keypair.generate();

    const pythProduct = new PublicKey(
        "ALP8SdU9oARYVLgLR7LrqMNCYBnhtnQz1cj6bwgwQmgj"
    );
    const pythPrice = new PublicKey(
        "H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG"
    );
    const switchboardFeed = new PublicKey(
        "AdtRGGhmqvom3Jemp5YNrxd9q9unX36BZk1pujkkXijL"
    );

    const slotsPerYear = 63072000;
    const initialReserveAmount = 1_000_000_000;
    const initialCollateralRatio = 1.0;
    const referralFeeOwner = Keypair.generate().publicKey;
    const poolSizeLimit = 100 * LAMPORTS_PER_SOL;
    let isNative = false;

    let reserveToken: Token;

    let jet: JetReserveAsset;
    let solend: SolendReserveAsset;
    let port: PortReserveAsset;

    let vaultClient: VaultClient;
    let userReserveTokenAccount: PublicKey;
    let ownerReserveTokenAccount: PublicKey;

    let expectedWithdrawAmount: anchor.BN;
    let lastUpdatedVaultBalance: anchor.BN;
    let totalFees: { primary: anchor.BN; referral: anchor.BN };
    let lastUpdatedSlot: number;

    async function fetchSlots(txs: string[]): Promise<number[]> {
        const slots = (
            await Promise.all(
                txs.map((tx) => {
                    return provider.connection.getParsedConfirmedTransaction(
                        tx,
                        "confirmed"
                    );
                })
            )
        ).map((res) => res.slot);

        return slots;
    }

    function setNativeMode(mode: boolean = true) {
        isNative = mode;
    }

    function calcReserveToLp(
        amount: anchor.BN,
        lpSupply: anchor.BN,
        vaultValue: anchor.BN
    ): anchor.BN {
        return lpSupply.mul(amount).div(vaultValue);
    }

    function splitFees(
        total: anchor.BN,
        splitPercentage: number
    ): [anchor.BN, anchor.BN] {
        const primFees = total
            .mul(new anchor.BN(100 - splitPercentage))
            .div(new anchor.BN(100));

        const refFees = total
            .mul(new anchor.BN(splitPercentage))
            .div(new anchor.BN(100));

        return [primFees, refFees];
    }

    async function calculateFees(
        vaultBalance: anchor.BN,
        lpMintSupply: anchor.BN,
        currentSlot: number,
        nextSlot: number,
        feeCarryBps: number,
        feeMgmtBps: number,
        referralFeePct: number
    ) {
        const bpsWhole = new anchor.BN(10_000);

        let dt = nextSlot - currentSlot;

        // TODO add carry fee calculation
        //const carryFees = newVaultBalance
        //  .sub(vaultBalance)
        //  .mul(new anchor.BN(feeCarryBps))
        //  .div(bpsWhole);

        const mgmtFees = vaultBalance
            .mul(new anchor.BN(feeMgmtBps))
            .div(bpsWhole)
            .div(new anchor.BN(slotsPerYear))
            .mul(new anchor.BN(dt));

        const tFees = mgmtFees;
        const tLpFees = calcReserveToLp(tFees, lpMintSupply, vaultBalance);

        const [primFee, refFee] = splitFees(tLpFees, referralFeePct);

        return { primary: primFee, referral: refFee };
    }

    async function createAndMintReserveTokens(
        recipient: PublicKey,
        reserveToken: Token,
        mintAmount: number
    ) {
        const ownerReserveTokenAccount = await Token.getAssociatedTokenAddress(
            ASSOCIATED_TOKEN_PROGRAM_ID,
            TOKEN_PROGRAM_ID,
            reserveToken.publicKey,
            recipient
        );

        const createTokenAccountIx =
            Token.createAssociatedTokenAccountInstruction(
                ASSOCIATED_TOKEN_PROGRAM_ID,
                TOKEN_PROGRAM_ID,
                reserveToken.publicKey,
                ownerReserveTokenAccount,
                recipient,
                reserveToken.payer.publicKey
            );

        const mintTokensIx = isNative
            ? [
                  SystemProgram.transfer({
                      fromPubkey: reserveToken.payer.publicKey,
                      toPubkey: ownerReserveTokenAccount,
                      lamports:
                          mintAmount +
                          (await provider.connection.getMinimumBalanceForRentExemption(
                              AccountLayout.span
                          )),
                  }),
                  createTokenAccountIx,
              ]
            : [
                  createTokenAccountIx,
                  Token.createMintToInstruction(
                      TOKEN_PROGRAM_ID,
                      reserveToken.publicKey,
                      ownerReserveTokenAccount,
                      reserveToken.payer.publicKey,
                      [],
                      mintAmount
                  ),
              ];

        const tx = new Transaction({
            feePayer: reserveToken.payer.publicKey,
            recentBlockhash: (await provider.connection.getRecentBlockhash())
                .blockhash,
        }).add(...mintTokensIx);

        const txHash = await provider.connection.sendTransaction(tx, [
            reserveToken.payer,
        ]);

        await provider.connection.confirmTransaction(txHash, "singleGossip");

        return { ownerReserveTokenAccount };
    }

    async function initLendingMarkets() {
        lastUpdatedVaultBalance = new anchor.BN(0);
        expectedWithdrawAmount = new anchor.BN(0);
        totalFees = { primary: new anchor.BN(0), referral: new anchor.BN(0) };
        lastUpdatedSlot = 0;

        const sig = await provider.connection.requestAirdrop(
            owner.publicKey,
            10 * initialReserveAmount
        );

        const supplSig = await provider.connection.requestAirdrop(
            referralFeeOwner,
            1 * initialReserveAmount
        );

        await provider.connection.confirmTransaction(sig, "singleGossip");
        await provider.connection.confirmTransaction(supplSig, "singleGossip");

        reserveToken = isNative
            ? new Token(
                  provider.connection,
                  NATIVE_MINT,
                  TOKEN_PROGRAM_ID,
                  owner
              )
            : await Token.createMint(
                  provider.connection,
                  owner,
                  owner.publicKey,
                  null,
                  2,
                  TOKEN_PROGRAM_ID
              );

        ownerReserveTokenAccount = (
            await createAndMintReserveTokens(
                owner.publicKey,
                reserveToken,
                3 * initialReserveAmount
            )
        ).ownerReserveTokenAccount;

        const pythProgram = new PublicKey(
            "FsJ3A3u2vn5cTVofAjvy6y5kwABJAqYWpe4975bi2epH"
        );
        const switchboardProgram = new PublicKey(
            "DtmE9D2CSB4L5D6A15mraeEjrGMm6auWVzgaD8hK2tZM"
        );

        solend = await SolendReserveAsset.initialize(
            provider,
            owner,
            wallet,
            reserveToken.publicKey,
            pythProgram,
            switchboardProgram,
            pythProduct,
            pythPrice,
            switchboardFeed,
            ownerReserveTokenAccount,
            initialReserveAmount
        );

        port = await PortReserveAsset.initialize(
            provider,
            owner,
            reserveToken.publicKey,
            pythPrice,
            ownerReserveTokenAccount,
            initialReserveAmount
        );

        jet = await JetReserveAsset.initialize(
            provider,
            wallet,
            owner,
            NATIVE_MINT,
            reserveToken,
            pythPrice,
            pythProduct,
            ownerReserveTokenAccount,
            initialReserveAmount
        );
    }

    async function initializeVault(
        strategyType: StrategyType,
        feeCarryBps: number = 0,
        feeMgmtBps: number = 0,
        referralFeePct: number = 0
    ) {
        vaultClient = await VaultClient.initialize(
            program,
            provider.wallet as anchor.Wallet,
            reserveToken.publicKey,
            solend,
            port,
            jet,
            strategyType,
            owner.publicKey,
            { feeCarryBps, feeMgmtBps, referralFeeOwner, referralFeePct },
            poolSizeLimit
        );

        userReserveTokenAccount =
            await reserveToken.createAssociatedTokenAccount(wallet.publicKey);
    }

    async function getReserveTokenBalance(account: PublicKey): Promise<number> {
        const info = await reserveToken.getAccountInfo(account);
        return info.amount.toNumber();
    }

    async function getUserReserveTokenBalance(): Promise<number> {
        const account = await vaultClient.getUserReserveTokenAccount(
            wallet.publicKey
        );
        return await getReserveTokenBalance(account);
    }

    async function getVaultReserveTokenBalance(): Promise<number> {
        return await getReserveTokenBalance(
            vaultClient.vaultState.vaultReserveToken
        );
    }

    async function getVaultTotalValue(): Promise<number> {
        return (await vaultClient.getTotalValue()).toNumber();
    }

    async function getUserLpTokenBalance(): Promise<number> {
        const account = await vaultClient.getUserLpTokenAccount(
            wallet.publicKey
        );
        // Adding this check because the lptoken creation is inside deposit tx and not a standalone tx
        try {
            const info = await vaultClient.getLpTokenAccountInfo(account);
            return info.amount.toNumber();
        } catch (err) {
            return 0;
        }
    }

    async function getLpTokenSupply(): Promise<number> {
        const info = await vaultClient.getLpTokenMintInfo();
        return info.supply.toNumber();
    }

    async function mintReserveTokens(receiver: PublicKey, qty: number) {
        const instructions = isNative
            ? [
                  SystemProgram.transfer({
                      fromPubkey: owner.publicKey,
                      toPubkey: receiver,
                      lamports: qty,
                  }),
                  createTokenSyncNativeInstruction(receiver),
              ]
            : [
                  Token.createMintToInstruction(
                      TOKEN_PROGRAM_ID,
                      reserveToken.publicKey,
                      receiver,
                      owner.publicKey,
                      [],
                      qty
                  ),
              ];

        const tx = new Transaction().add(...instructions);
        const txHash = await provider.connection.sendTransaction(tx, [owner]);

        await provider.connection.confirmTransaction(txHash, "singleGossip");
    }

    async function depositToVault(qty: number): Promise<string[]> {
        const txs = await vaultClient.deposit(
            wallet,
            qty,
            userReserveTokenAccount
        );
        await provider.connection.confirmTransaction(
            txs[txs.length - 1],
            "singleGossip"
        );
        return txs;
    }

    async function withdrawFromVault(qty: number): Promise<string[]> {
        const txs = await vaultClient.withdraw(wallet, qty);
        await provider.connection.confirmTransaction(
            txs[txs.length - 1],
            "singleGossip"
        );
        return txs;
    }

    async function performRebalance(): Promise<string[]> {
        const txs = await vaultClient.rebalance();
        await provider.connection.confirmTransaction(
            txs[txs.length - 1],
            "singleGossip"
        );
        return txs;
    }

    function testDepositAndWithdrawal() {
        it("Vault correctly initialized", async function () {
            assert.notEqual(vaultClient.vaultState, null);
            assert.equal(
                vaultClient.vaultState.poolSizeLimit.toNumber(),
                poolSizeLimit
            );
        });

        it("Deposits to vault reserves", async function () {
            const qty = 1_000_000_000;
            await mintReserveTokens(userReserveTokenAccount, qty);
            await depositToVault(qty);
            const userReserveBalance = await getUserReserveTokenBalance();
            const vaultReserveBalance = await getVaultReserveTokenBalance();
            const userLpBalance = await getUserLpTokenBalance();
            const lpTokenSupply = await getLpTokenSupply();

            assert.equal(userReserveBalance, 0);
            assert.equal(vaultReserveBalance, qty);
            assert.equal(userLpBalance, qty * initialCollateralRatio);
            assert.equal(lpTokenSupply, qty * initialCollateralRatio);
        });

        it("Withdraw funds from vault", async function () {
            const vaultReserveBalance0 = await getVaultReserveTokenBalance();
            const loTokenSupply0 = await getLpTokenSupply();

            const qty1 = 700_000_000;
            await withdrawFromVault(qty1);

            const userReserveBalance1 = await getUserReserveTokenBalance();
            const vaultReserveBalance1 = await getVaultReserveTokenBalance();
            const loTokenSupply1 = await getLpTokenSupply();

            assert.equal(vaultReserveBalance0 - vaultReserveBalance1, qty1);
            assert.equal(loTokenSupply0 - loTokenSupply1, qty1);
            assert.equal(userReserveBalance1, qty1);

            const qty2 = 200_000_000;
            await withdrawFromVault(qty2);

            const userReserveBalance2 = await getUserReserveTokenBalance();
            const vaultReserveBalance2 = await getVaultReserveTokenBalance();
            const loTokenSupply2 = await getLpTokenSupply();

            assert.equal(vaultReserveBalance1 - vaultReserveBalance2, qty2);
            assert.equal(loTokenSupply1 - loTokenSupply2, qty2);
            assert.equal(userReserveBalance2, qty1 + qty2);

            if (isNative) {
                await reserveToken.closeAccount(
                    userReserveTokenAccount,
                    wallet.publicKey,
                    wallet.payer,
                    []
                );

                await reserveToken.closeAccount(
                    ownerReserveTokenAccount,
                    owner.publicKey,
                    owner,
                    []
                );
                setNativeMode(false);
            }
        });
    }

    function testDepositCap() {
        it("Initialize vault correctly", async function () {
            assert.notEqual(vaultClient.vaultState, null);
            assert.equal(
                vaultClient.vaultState.poolSizeLimit.toNumber(),
                poolSizeLimit
            );
        });

        it("Reject transaction if deposit cap is reached", async function () {
            const qty = poolSizeLimit + 100;

            await mintReserveTokens(userReserveTokenAccount, qty);
            try {
                await depositToVault(qty);
                assert.fail("Deposit should be rejected but was not.");
            } catch (err) {
                // TODO check err
            }

            const userReserveBalance = await getReserveTokenBalance(
                userReserveTokenAccount
            );

            const vaultReserveBalance = await getVaultReserveTokenBalance();
            const userLpBalance = await getUserLpTokenBalance();
            const lpTokenSupply = await getLpTokenSupply();

            assert.equal(userReserveBalance, qty);
            assert.equal(vaultReserveBalance, 0);
            assert.equal(userLpBalance, 0);
            assert.equal(lpTokenSupply, 0);
        });

        it("Update deposit cap", async function () {
            const newDepositCap = poolSizeLimit * 0.24;
            const txs = await vaultClient.updateDepositCap(
                owner,
                newDepositCap
            );
            await provider.connection.confirmTransaction(
                txs[txs.length - 1],
                "singleGossip"
            );
            await vaultClient.reload();
            assert.equal(
                newDepositCap,
                vaultClient.vaultState.poolSizeLimit.toNumber()
            );
        });

        it("Reject unauthorized deposit cap update", async function () {
            const noPermissionUser = Keypair.generate();

            const prevDepositCap =
                vaultClient.vaultState.poolSizeLimit.toNumber();
            const newDepositCap = prevDepositCap * 0.24;

            try {
                const txs = await vaultClient.updateDepositCap(
                    noPermissionUser,
                    newDepositCap
                );
                await provider.connection.confirmTransaction(
                    txs[txs.length - 1],
                    "singleGossip"
                );
                assert.fail("Transaction should be rejected but was not.");
            } catch (err) {
                // TODO check err
            }

            await vaultClient.reload();
            assert.equal(
                prevDepositCap,
                vaultClient.vaultState.poolSizeLimit.toNumber()
            );
        });
    }

    function testRebalance(
        expectedSolendRatio: number = 1 / 3,
        expectedPortRatio: number = 1 / 3,
        expectedJetRatio: number = 1 / 3
    ) {
        it("Perform rebalance", async function () {
            const qty = 1024503;
            await mintReserveTokens(userReserveTokenAccount, qty);
            await depositToVault(qty);

            await performRebalance();

            const totalValue = await getVaultTotalValue();
            const solendValue = (
                await vaultClient.solend.getLpTokenAccountValue(
                    vaultClient.vaultState.vaultSolendLpToken
                )
            ).toNumber();
            const jetValue = (
                await vaultClient.jet.getLpTokenAccountValue(
                    vaultClient.vaultState.vaultJetLpToken
                )
            ).toNumber();
            const portValue = (
                await vaultClient.port.getLpTokenAccountValue(
                    vaultClient.vaultState.vaultPortLpToken
                )
            ).toNumber();

            assert.equal(totalValue, qty);

            // Use isAtMost because on-chain rust program handles rounding differently than TypeScript.
            // However the difference should not exceet 1 token.
            assert.isAtMost(
                Math.abs(solendValue - Math.floor(qty * expectedSolendRatio)),
                1
            );
            assert.isAtMost(
                Math.abs(portValue - Math.floor(qty * expectedPortRatio)),
                1
            );
            assert.isAtMost(
                Math.abs(jetValue - Math.floor(qty * expectedJetRatio)),
                1
            );

            const solendReserveBalance = await getReserveTokenBalance(
                vaultClient.solend.accounts.liquiditySupply
            );
            const portReserveBalance = await getReserveTokenBalance(
                vaultClient.port.accounts.liquiditySupply
            );
            const jetReserveBalance = await getReserveTokenBalance(
                vaultClient.jet.accounts.liquiditySupply
            );

            assert.equal(
                solendReserveBalance,
                solendValue + initialReserveAmount
            );
            assert.equal(portReserveBalance, portValue + initialReserveAmount);
            assert.equal(jetReserveBalance, jetValue + initialReserveAmount);
        });
    }

    async function sleep(t: number) {
        return new Promise((res) => setTimeout(res, t));
    }

    function testFeeComputation(
        feeCarryBps: number = 0,
        feeMgmtBps: number = 0,
        referalPct: number = 0
    ) {
        it("Collect fees", async function () {
            const qty1 = 5.47 * 10 ** 9;
            await mintReserveTokens(userReserveTokenAccount, qty1);
            let txs = await depositToVault(qty1);
            const slots0 = await fetchSlots(txs);

            const vaultTotalValue = new anchor.BN(await getVaultTotalValue());
            const lpTokenSupply = new anchor.BN(await getLpTokenSupply());

            await sleep(1000);

            // This is needed to trigger refresh and fee collection
            // Consider adding an API to client library to trigger refresh
            const qty2 = 10;
            await mintReserveTokens(userReserveTokenAccount, qty2);
            txs = await depositToVault(qty2);
            const slots1 = await fetchSlots(txs);

            const expectedFees = await calculateFees(
                vaultTotalValue,
                lpTokenSupply,
                slots0[slots0.length - 1],
                slots1[slots1.length - 1],
                feeCarryBps,
                feeMgmtBps,
                referalPct
            );

            const referralAccountInfo =
                await vaultClient.getReferralFeeReceiverAccountInfo();
            const feeAccountInfo =
                await vaultClient.getFeeReceiverAccountInfo();

            const actualReferralFees = referralAccountInfo.amount.toNumber();
            const actualMgmtFees = feeAccountInfo.amount.toNumber();

            assert.isAtMost(
                Math.abs(actualMgmtFees - expectedFees.primary.toNumber()),
                1
            );
            assert.isAtMost(
                Math.abs(actualReferralFees - expectedFees.referral.toNumber()),
                1
            );
        });

        it("Update fee rates", async function () {
            const prevFees = vaultClient.getFees();
            const newFeeCarryBps = prevFees.feeCarryBps / 2;
            const newFeeMgmtBps = prevFees.feeMgmtBps / 2;
            const newReferralFeePct = prevFees.referralFeePct / 2;

            const txs = await vaultClient.updateFees(
                owner,
                newFeeCarryBps,
                newFeeMgmtBps,
                newReferralFeePct
            );
            await provider.connection.confirmTransaction(
                txs[txs.length - 1],
                "singleGossip"
            );
            await vaultClient.reload();
            const actualFees = vaultClient.getFees();
            assert.equal(newFeeCarryBps, actualFees.feeCarryBps);
            assert.equal(newFeeMgmtBps, actualFees.feeMgmtBps);
            assert.equal(newReferralFeePct, actualFees.referralFeePct);
        });

        it("Reject unauthorized fee rate update", async function () {
            const prevFees = vaultClient.getFees();
            const newFeeCarryBps = prevFees.feeCarryBps / 2;
            const newFeeMgmtBps = prevFees.feeMgmtBps / 2;
            const newReferralFeePct = prevFees.referralFeePct / 2;

            const noPermissionUser = Keypair.generate();

            try {
                const txs = await vaultClient.updateFees(
                    noPermissionUser,
                    newFeeCarryBps,
                    newFeeMgmtBps,
                    newReferralFeePct
                );
                await provider.connection.confirmTransaction(
                    txs[txs.length - 1],
                    "singleGossip"
                );
                assert.fail("Transaction should be rejected but was not.");
            } catch (err) {
                // TODO check err
            }

            await vaultClient.reload();
            const actualFees = vaultClient.getFees();
            assert.equal(prevFees.feeCarryBps, actualFees.feeCarryBps);
            assert.equal(prevFees.feeMgmtBps, actualFees.feeMgmtBps);
            assert.equal(prevFees.referralFeePct, actualFees.referralFeePct);
        });

        it("Reject invalid fee rates", async function () {
            const prevFees = vaultClient.getFees();
            const newFeeCarryBps = prevFees.feeCarryBps / 2;
            const newFeeMgmtBps = prevFees.feeMgmtBps / 2;
            const newReferralFeePct = 80;
            try {
                const txs = await vaultClient.updateFees(
                    owner,
                    newFeeCarryBps,
                    newFeeMgmtBps,
                    newReferralFeePct
                );
                await provider.connection.confirmTransaction(
                    txs[txs.length - 1],
                    "singleGossip"
                );
                assert.fail("Transaction should be rejected but was not.");
            } catch (err) {
                // TODO check err
            }

            await vaultClient.reload();
            const actualFees = vaultClient.getFees();
            assert.equal(prevFees.feeCarryBps, actualFees.feeCarryBps);
            assert.equal(prevFees.feeMgmtBps, actualFees.feeMgmtBps);
            assert.equal(prevFees.referralFeePct, actualFees.referralFeePct);
        });
    }

    describe("Equal allocation strategy", () => {
        describe("Deposit and withdrawal for WSOL", () => {
            before(setNativeMode.bind(this));
            before(initLendingMarkets.bind(this));
            before(async function () {
                await initializeVault({ equalAllocation: {} });
            });
            testDepositAndWithdrawal();
        });

        describe("Deposit and withdrawal", () => {
            before(initLendingMarkets);
            before(async function () {
                await initializeVault({ equalAllocation: {} });
            });
            testDepositAndWithdrawal();
        });

        describe("Deposit cap", () => {
            before(initLendingMarkets);
            before(async function () {
                await initializeVault({ equalAllocation: {} });
            });
            testDepositCap();
        });

        describe("Rebalance", () => {
            before(initLendingMarkets);
            before(async function () {
                await initializeVault({ equalAllocation: {} });
            });
            testRebalance();
        });

        describe("Fees", () => {
            const feeMgmtBps = 10000;
            const feeCarryBps = 10000;
            const referralFeePct = 20;

            before(initLendingMarkets);
            before(async function () {
                await initializeVault(
                    { equalAllocation: {} },
                    feeCarryBps,
                    feeMgmtBps,
                    referralFeePct
                );
            });
            testFeeComputation(feeCarryBps, feeMgmtBps, referralFeePct);
        });
        // TODO borrow from solend to increase apy and ensure it switches to that
        // TODO borrow from port to increase apy and ensure it switches to that
    });

    describe("Max yield strategy", () => {
        describe("Deposit and withdrawal", () => {
            before(initLendingMarkets);
            before(async function () {
                await initializeVault({ maxYield: {} });
            });
            testDepositAndWithdrawal();
        });

        describe("Deposit and withdrawal for WSOL", () => {
            before(setNativeMode.bind(this));
            before(initLendingMarkets.bind(this));
            before(async function () {
                await initializeVault({ maxYield: {} });
            });
            testDepositAndWithdrawal();
        });

        describe("Deposit cap", () => {
            before(initLendingMarkets);
            before(async function () {
                await initializeVault({ maxYield: {} });
            });
            testDepositCap();
        });

        describe("Rebalance", () => {
            before(initLendingMarkets);
            before(async function () {
                await initializeVault({ maxYield: {} });
            });
            testRebalance(0, 0, 1);
        });

        describe("Fees", () => {
            const feeMgmtBps = 10000;
            const feeCarryBps = 10000;
            const referralFeePct = 20;

            before(initLendingMarkets);
            before(async function () {
                await initializeVault(
                    { maxYield: {} },
                    feeCarryBps,
                    feeMgmtBps,
                    referralFeePct
                );
            });
            testFeeComputation(feeCarryBps, feeMgmtBps, referralFeePct);
        });
    });
});
