import { assert } from "chai";
import * as anchor from "@project-serum/anchor";
import { TOKEN_PROGRAM_ID, Token, NATIVE_MINT } from "@solana/spl-token";
import { Keypair, PublicKey, TransactionSignature } from "@solana/web3.js";

import {
    SolendReserveAsset,
    JetReserveAsset,
    PortReserveAsset,
    VaultClient,
    CastleVault,
    ProposedWeightsBps,
} from "../sdk/src/index";
import {
    DeploymentEnvs,
    RebalanceMode,
    RebalanceModes,
    StrategyType,
    StrategyTypes,
} from "@castlefinance/vault-core";

describe("castle-vault", () => {
    const provider = anchor.Provider.env();
    anchor.setProvider(provider);
    const wallet = provider.wallet as anchor.Wallet;

    const program = anchor.workspace.CastleVault as anchor.Program<CastleVault>;

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
    const initialReserveAmount = 10000000;
    const initialCollateralRatio = 1.0;
    const referralFeeOwner = Keypair.generate().publicKey;
    const vaultDepositCap = 10 * 10 ** 9;
    const vaultAllocationCap = 76;

    let jetReserveAmount: number;

    let reserveToken: Token;

    let jet: JetReserveAsset;
    let solend: SolendReserveAsset;
    let port: PortReserveAsset;

    let vaultClient: VaultClient;
    let userReserveTokenAccount: PublicKey;

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

    async function initLendingMarkets() {
        lastUpdatedVaultBalance = new anchor.BN(0);
        expectedWithdrawAmount = new anchor.BN(0);
        totalFees = { primary: new anchor.BN(0), referral: new anchor.BN(0) };
        lastUpdatedSlot = 0;

        const sig = await provider.connection.requestAirdrop(
            owner.publicKey,
            1000000000
        );

        const supplSig = await provider.connection.requestAirdrop(
            referralFeeOwner,
            1000000000
        );

        await provider.connection.confirmTransaction(sig, "singleGossip");
        await provider.connection.confirmTransaction(supplSig, "singleGossip");

        reserveToken = await Token.createMint(
            provider.connection,
            owner,
            owner.publicKey,
            null,
            2,
            TOKEN_PROGRAM_ID
        );

        const ownerReserveTokenAccount = await reserveToken.createAccount(
            owner.publicKey
        );

        await reserveToken.mintTo(
            ownerReserveTokenAccount,
            owner,
            [],
            3 * initialReserveAmount
        );

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

        const jetBorrowedAmt = initialReserveAmount / 2;
        jetReserveAmount = initialReserveAmount - jetBorrowedAmt;
        const jetBorrowTxs = await jet.borrow(
            owner,
            ownerReserveTokenAccount,
            jetBorrowedAmt
        );
        await provider.connection.confirmTransaction(
            jetBorrowTxs[jetBorrowTxs.length - 1],
            "finalized"
        );
    }

    async function initializeVault(
        strategyType: StrategyType,
        rebalanceMode: RebalanceMode,
        feeCarryBps: number = 0,
        feeMgmtBps: number = 0,
        referralFeePct: number = 0
    ) {
        vaultClient = await VaultClient.initialize(
            provider,
            provider.wallet as anchor.Wallet,
            DeploymentEnvs.devnetStaging,
            reserveToken.publicKey,
            solend,
            port,
            jet,
            strategyType,
            rebalanceMode,
            owner.publicKey,
            { feeCarryBps, feeMgmtBps, referralFeeOwner, referralFeePct },
            vaultDepositCap,
            vaultAllocationCap,
            program
        );

        userReserveTokenAccount = await reserveToken.createAccount(
            wallet.publicKey
        );
    }

    async function getReserveTokenBalance(account: PublicKey): Promise<number> {
        const info = await vaultClient.getReserveTokenAccountInfo(account);
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
            vaultClient.getVaultReserveTokenAccount()
        );
    }

    async function getVaultTotalValue(): Promise<number> {
        return (await vaultClient.getTotalValue()).toNumber();
    }

    async function getUserLpTokenBalance(): Promise<number> {
        const account = await vaultClient.getUserLpTokenAccount(
            wallet.publicKey
        );
        const info = await vaultClient.getLpTokenAccountInfo(account);
        return info.amount.toNumber();
    }

    async function getLpTokenSupply(): Promise<number> {
        const info = await vaultClient.getLpTokenMintInfo();
        return info.supply.toNumber();
    }

    async function mintReserveToken(receiver: PublicKey, qty: number) {
        await reserveToken.mintTo(receiver, owner, [], qty);
    }

    async function depositToVault(
        qty: number
    ): Promise<TransactionSignature[]> {
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

    async function withdrawFromVault(
        qty: number
    ): Promise<TransactionSignature[]> {
        const txs = await vaultClient.withdraw(wallet, qty);
        await provider.connection.confirmTransaction(
            txs[txs.length - 1],
            "singleGossip"
        );
        return txs;
    }

    /**
     *
     * @param proposedWeights
     * @param rebalanceOnly if true, skips initial simulation and reconciles
     *                      used for testing errors since sim doesn't give a msg
     * @returns
     */
    async function performRebalance(
        proposedWeights?: ProposedWeightsBps,
        rebalanceOnly: boolean = false
    ): Promise<TransactionSignature[]> {
        let txSigs = null;

        if (rebalanceOnly) {
            const tx = vaultClient.getRebalanceTx(proposedWeights);
            txSigs = [await provider.send(tx)];
        } else {
            txSigs = await vaultClient.rebalance(proposedWeights);
        }
        await provider.connection.confirmTransaction(
            txSigs[txSigs.length - 1],
            "singleGossip"
        );
        return txSigs;
    }

    function testDepositAndWithdrawal() {
        it("Initializes a vault", async function () {
            assert.isNotNull(vaultClient.getLpTokenMintInfo());
            assert.isNotNull(vaultClient.getVaultReserveTokenAccount());
            assert.isNotNull(vaultClient.getFeeReceiverAccountInfo());
            assert.isNotNull(vaultClient.getReferralFeeReceiverAccountInfo());
            assert.equal(
                vaultClient.getDepositCap().toNumber(),
                vaultDepositCap
            );
            assert.equal(vaultClient.getAllocationCap(), vaultAllocationCap);
        });

        it("Deposits to vault reserves", async function () {
            const qty = 100;

            await mintReserveToken(userReserveTokenAccount, qty);
            await depositToVault(qty);

            const userReserveBalance = await getReserveTokenBalance(
                userReserveTokenAccount
            );
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

            const qty1 = 70;
            await withdrawFromVault(qty1);

            const userReserveBalance1 = await getUserReserveTokenBalance();
            const vaultReserveBalance1 = await getVaultReserveTokenBalance();
            const loTokenSupply1 = await getLpTokenSupply();

            assert.equal(vaultReserveBalance0 - vaultReserveBalance1, qty1);
            assert.equal(loTokenSupply0 - loTokenSupply1, qty1);
            assert.equal(userReserveBalance1, qty1);

            const qty2 = 20;
            await withdrawFromVault(qty2);

            const userReserveBalance2 = await getUserReserveTokenBalance();
            const vaultReserveBalance2 = await getVaultReserveTokenBalance();
            const loTokenSupply2 = await getLpTokenSupply();

            assert.equal(vaultReserveBalance1 - vaultReserveBalance2, qty2);
            assert.equal(loTokenSupply1 - loTokenSupply2, qty2);
            assert.equal(userReserveBalance2, qty1 + qty2);
        });
    }

    function testDepositCap() {
        it("Initialize vault correctly", async function () {
            assert.isNotNull(vaultClient.getLpTokenMintInfo());
            assert.isNotNull(vaultClient.getVaultReserveTokenAccount());
            assert.isNotNull(vaultClient.getFeeReceiverAccountInfo());
            assert.isNotNull(vaultClient.getReferralFeeReceiverAccountInfo());
            assert.equal(
                vaultClient.getDepositCap().toNumber(),
                vaultDepositCap
            );
            assert.equal(vaultClient.getAllocationCap(), vaultAllocationCap);
        });

        it("Reject transaction if deposit cap is reached", async function () {
            const depositCapErrorCode = program.idl.errors
                .find((e) => e.name == "DepositCapError")
                .code.toString(16);

            const qty = vaultDepositCap + 100;
            await mintReserveToken(userReserveTokenAccount, qty);

            try {
                await depositToVault(qty);
                assert.fail("Deposit should be rejected but was not.");
            } catch (err) {
                assert.isTrue(
                    err.message.includes(depositCapErrorCode),
                    `Error code ${depositCapErrorCode} not included in error message: ${err}`
                );
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
            const newDepositCap = vaultDepositCap * 0.24;
            const txs = await vaultClient.updateDepositCap(
                owner,
                newDepositCap
            );
            await provider.connection.confirmTransaction(
                txs[txs.length - 1],
                "singleGossip"
            );
            await vaultClient.reload();
            assert.equal(newDepositCap, vaultClient.getDepositCap().toNumber());
        });

        it("Reject unauthorized deposit cap update", async function () {
            const noPermissionUser = Keypair.generate();

            const prevDepositCap = vaultClient.getDepositCap().toNumber();
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
                vaultClient.getDepositCap().toNumber()
            );
        });
    }

    function testRebalance(
        expectedSolendRatio: number,
        expectedPortRatio: number,
        expectedJetRatio: number,
        rebalanceMode: RebalanceMode = RebalanceModes.calculator
    ) {
        // NOTE: should not be divisible by the number of markets, 3 in this case
        // The TODO to correct this is below
        const qty = 1024502;

        before(async () => {
            await mintReserveToken(userReserveTokenAccount, qty);
            await depositToVault(qty);
        });

        if (rebalanceMode == RebalanceModes.proofChecker) {
            it("Rejects tx if weights don't equal 100%", async () => {
                const errorCode = program.idl.errors
                    .find((e) => e.name == "InvalidProposedWeights")
                    .code.toString(16);
                try {
                    await performRebalance(
                        {
                            solend: 0,
                            port: 0,
                            jet: 0,
                        },
                        true
                    );
                    assert.fail("Rebalance did not fail");
                } catch (err) {
                    assert.isTrue(
                        err.message.includes(errorCode),
                        `Error code ${errorCode} not included in error message: ${err}`
                    );
                }

                assert.equal(
                    (
                        await vaultClient.getVaultSolendLpTokenAccountValue()
                    ).toNumber(),
                    0
                );
                assert.equal(
                    (
                        await vaultClient.getVaultPortLpTokenAccountValue()
                    ).toNumber(),
                    0
                );
                assert.equal(
                    (
                        await vaultClient.getVaultJetLpTokenAccountValue()
                    ).toNumber(),
                    0
                );
            });

            it("Rejects tx if weights are suboptimal", async () => {
                const errorCode = program.idl.errors
                    .find((e) => e.name == "RebalanceProofCheckFailed")
                    .code.toString(16);
                try {
                    await performRebalance(
                        {
                            solend: 3333,
                            port: 3333,
                            jet: 3334,
                        },
                        true
                    );
                    assert.fail("Rebalance did not fail");
                } catch (err) {
                    assert.isTrue(
                        err.message.includes(errorCode),
                        `Error code ${errorCode} not included in error message: ${err}`
                    );
                }

                assert.equal(
                    (
                        await vaultClient.getVaultSolendLpTokenAccountValue()
                    ).toNumber(),
                    0
                );
                assert.equal(
                    (
                        await vaultClient.getVaultPortLpTokenAccountValue()
                    ).toNumber(),
                    0
                );
                assert.equal(
                    (
                        await vaultClient.getVaultJetLpTokenAccountValue()
                    ).toNumber(),
                    0
                );
            });

            it("Rejects tx if weights exceeds the cap", async () => {
                const errorCode = program.idl.errors
                    .find((e) => e.name == "InvalidProposedWeights")
                    .code.toString(16);
                try {
                    await performRebalance(
                        {
                            solend: 10000,
                            port: 0,
                            jet: 0,
                        },
                        true
                    );
                    assert.fail("Rebalance did not fail");
                } catch (err) {
                    assert.isTrue(
                        err.message.includes(errorCode),
                        `Error code ${errorCode} not included in error message: ${err}`
                    );
                }

                assert.equal(
                    (
                        await vaultClient.getVaultSolendLpTokenAccountValue()
                    ).toNumber(),
                    0
                );
                assert.equal(
                    (
                        await vaultClient.getVaultPortLpTokenAccountValue()
                    ).toNumber(),
                    0
                );
                assert.equal(
                    (
                        await vaultClient.getVaultJetLpTokenAccountValue()
                    ).toNumber(),
                    0
                );
            });
        }

        it("Rebalances", async () => {
            await performRebalance({
                solend: expectedSolendRatio * 10000,
                port: expectedPortRatio * 10000,
                jet: expectedJetRatio * 10000,
            });

            const totalValue = await getVaultTotalValue();
            const solendValue = (
                await vaultClient.getVaultSolendLpTokenAccountValue()
            ).toNumber();
            const portValue = (
                await vaultClient.getVaultPortLpTokenAccountValue()
            ).toNumber();
            const jetValue = (
                await vaultClient.getVaultJetLpTokenAccountValue()
            ).toNumber();

            // TODO Resolve the difference
            // Use isAtMost because on-chain rust program handles rounding differently than TypeScript.
            // However the difference should not exceet 1 token.
            const maxDiffAllowed = 1;
            assert.isAtMost(
                Math.abs(totalValue - Math.floor(qty)),
                maxDiffAllowed
            );
            assert.isAtMost(
                Math.abs(solendValue - Math.floor(qty * expectedSolendRatio)),
                maxDiffAllowed
            );
            assert.isAtMost(
                Math.abs(portValue - Math.floor(qty * expectedPortRatio)),
                maxDiffAllowed
            );
            assert.isAtMost(
                Math.abs(jetValue - Math.floor(qty * expectedJetRatio)),
                maxDiffAllowed
            );
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
            await mintReserveToken(userReserveTokenAccount, qty1);
            const txs1 = await depositToVault(qty1);
            const slots0 = await fetchSlots(txs1);

            const vaultTotalValue = new anchor.BN(await getVaultTotalValue());
            const lpTokenSupply = new anchor.BN(await getLpTokenSupply());

            await sleep(1000);

            // This is needed to trigger refresh and fee collection
            // Consider adding an API to client library to trigger refresh
            const qty2 = 10;
            await mintReserveToken(userReserveTokenAccount, qty2);
            const txs2 = await depositToVault(qty2);
            const slots1 = await fetchSlots(txs2);

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
        describe("Deposit and withdrawal", () => {
            before(initLendingMarkets);
            before(async function () {
                await initializeVault(
                    StrategyTypes.equalAllocation,
                    RebalanceModes.calculator
                );
            });
            testDepositAndWithdrawal();
        });

        describe("Deposit cap", () => {
            before(initLendingMarkets);
            before(async function () {
                await initializeVault(
                    StrategyTypes.equalAllocation,
                    RebalanceModes.calculator
                );
            });
            testDepositCap();
        });

        describe("Rebalance", () => {
            before(initLendingMarkets);
            before(async function () {
                await initializeVault(
                    StrategyTypes.equalAllocation,
                    RebalanceModes.calculator
                );
            });
            testRebalance(1 / 3, 1 / 3, 1 / 3);
        });

        describe("Fees", () => {
            const feeMgmtBps = 10000;
            const feeCarryBps = 10000;
            const referralFeePct = 20;

            before(initLendingMarkets);
            before(async function () {
                await initializeVault(
                    StrategyTypes.equalAllocation,
                    RebalanceModes.calculator,
                    feeCarryBps,
                    feeMgmtBps,
                    referralFeePct
                );
            });
            testFeeComputation(feeCarryBps, feeMgmtBps, referralFeePct);
        });
    });

    describe("Max yield calculator", () => {
        describe("Rebalance", () => {
            before(initLendingMarkets);
            before(async function () {
                await initializeVault(
                    StrategyTypes.maxYield,
                    RebalanceModes.calculator
                );
            });

            testRebalance(
                1 - vaultAllocationCap / 100,
                0,
                vaultAllocationCap / 100
            );

            // TODO borrow from solend to increase apy and ensure it switches to that
            // TODO borrow from port to increase apy and ensure it switches to that
        });
    });

    describe("Max yield proof checker", () => {
        describe("Rebalance", () => {
            const rebalanceMode = RebalanceModes.proofChecker;
            before(initLendingMarkets);
            before(async function () {
                await initializeVault(StrategyTypes.maxYield, rebalanceMode);
            });
            testRebalance(
                1 - vaultAllocationCap / 100,
                0,
                vaultAllocationCap / 100,
                rebalanceMode
            );
        });
    });
});
