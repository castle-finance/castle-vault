import { assert } from "chai";
import * as anchor from "@project-serum/anchor";
import { TOKEN_PROGRAM_ID, Token, NATIVE_MINT } from "@solana/spl-token";
import { Keypair, PublicKey, TransactionSignature } from "@solana/web3.js";
import { StakingPool, StakeAccount } from "@castlefinance/port-sdk";

import {
    SolendReserveAsset,
    JetReserveAsset,
    PortReserveAsset,
    VaultClient,
    CastleVault,
    ProposedWeightsBps,
    VaultConfig,
    VaultFlags,
} from "../sdk/src/index";
import {
    DeploymentEnvs,
    RebalanceMode,
    RebalanceModes,
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

    let reserveToken: Token;

    let jet: JetReserveAsset;
    let solend: SolendReserveAsset;
    let port: PortReserveAsset;

    let vaultClient: VaultClient;
    let userReserveTokenAccount: PublicKey;

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
        config: VaultConfig,
        solendAvailable: boolean = true,
        portAvailable: boolean = true,
        jetAvailable: boolean = true
    ) {
        vaultClient = await VaultClient.initialize(
            provider,
            provider.wallet as anchor.Wallet,
            DeploymentEnvs.devnetStaging,
            reserveToken.publicKey,
            owner.publicKey,
            referralFeeOwner,
            config,
            program
        );

        await Promise.all([
            solendAvailable
                ? await vaultClient.initializeSolend(
                      provider.wallet as anchor.Wallet,
                      solend,
                      owner
                  )
                : {},
            portAvailable
                ? await vaultClient.initializePort(
                      provider.wallet as anchor.Wallet,
                      port,
                      owner
                  )
                : {},
            jetAvailable
                ? await vaultClient.initializeJet(
                      provider.wallet as anchor.Wallet,
                      jet,
                      owner
                  )
                : {},
        ]);

        if (portAvailable) {
            await vaultClient.initializePortAdditionalState(
                provider.wallet as anchor.Wallet,
                owner
            );
            await vaultClient.initializeRewardAccount(
                provider.wallet as anchor.Wallet,
                owner,
                provider,
                DeploymentEnvs.devnetStaging,
                program
            );
            await vaultClient.loadPortAdditionalAccounts();
        }

        await vaultClient.reload();

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
        return (await vaultClient.getTotalValue()).lamports.toNumber();
    }

    async function getUserLpTokenBalance(): Promise<number> {
        const account = await vaultClient.getUserLpTokenAccount(
            wallet.publicKey
        );
        const info = await vaultClient.getLpTokenAccountInfo(account);
        return info.amount.toNumber();
    }

    async function getLpTokenSupply(): Promise<number> {
        await vaultClient.reload();
        return vaultClient.getVaultState().lpTokenSupply.toNumber();
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
            const preRefresh = vaultClient.getPreRefreshTxs().map((tx) => {
                return { tx: tx, signers: [] };
            });
            const txs = [
                ...preRefresh,
                {
                    tx: await vaultClient.getRebalanceTx(proposedWeights),
                    signers: [],
                },
            ];
            txSigs = await provider.sendAll(txs);
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
                vaultClient.getDepositCap().lamports.toString(),
                "18446744073709551615"
            );
            assert.equal(
                vaultClient.getAllocationCap().asPercent().toNumber(),
                100
            );
            assert.equal(0b111, vaultClient.getYieldSourceFlags());
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
            const oldVaultReserveBalance = await getVaultReserveTokenBalance();
            const oldLpTokenSupply = await getLpTokenSupply();

            const qty = 70;
            await withdrawFromVault(qty);

            const newUserReserveBalance = await getUserReserveTokenBalance();
            const newVaultReserveBalance = await getVaultReserveTokenBalance();
            const newLpTokenSupply = await getLpTokenSupply();

            assert.equal(oldVaultReserveBalance - newVaultReserveBalance, qty);
            assert.equal(oldLpTokenSupply - newLpTokenSupply, qty);
            assert.equal(newUserReserveBalance, qty);
        });
    }

    function testDepositCap() {
        it("Initialize vault correctly", async function () {
            assert.isNotNull(vaultClient.getLpTokenMintInfo());
            assert.isNotNull(vaultClient.getVaultReserveTokenAccount());
            assert.isNotNull(vaultClient.getFeeReceiverAccountInfo());
            assert.isNotNull(vaultClient.getReferralFeeReceiverAccountInfo());
            assert.equal(
                vaultClient.getDepositCap().lamports.toNumber(),
                vaultDepositCap
            );
            assert.equal(
                vaultClient.getAllocationCap().asPercent().toNumber(),
                100
            );
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
            const oldConfig = vaultClient.getVaultConfig();
            const newConfig = {
                ...oldConfig,
                depositCap: oldConfig.depositCap.mul(new anchor.BN(24)),
            };
            const tx = await vaultClient.updateConfig(owner, newConfig);
            await provider.connection.confirmTransaction(tx, "singleGossip");
            await vaultClient.reload();
            assert.equal(
                newConfig.depositCap.toNumber(),
                vaultClient.getDepositCap().lamports.toNumber()
            );
        });

        it("Reject unauthorized config update", async function () {
            const errorCode = "0x8d";
            const noPermissionUser = Keypair.generate();

            const oldConfig = vaultClient.getVaultConfig();
            const newConfig = {
                ...oldConfig,
                depositCap: oldConfig.depositCap.mul(new anchor.BN(24)),
            };

            try {
                const tx = await vaultClient.updateConfig(
                    noPermissionUser,
                    newConfig
                );
                await provider.connection.confirmTransaction(
                    tx,
                    "singleGossip"
                );
                assert.fail("Transaction should be rejected but was not.");
            } catch (err) {
                assert.isTrue(
                    err.message.includes(errorCode),
                    `Error code ${errorCode} not included in error message: ${err}`
                );
            }

            await vaultClient.reload();
            assert.equal(
                oldConfig.depositCap.toNumber(),
                vaultClient.getDepositCap().lamports.toNumber()
            );
        });
    }

    function testVaultFlags() {
        it("Reject deposit transaction if vault is halted", async function () {
            const depositCapErrorCode = program.idl.errors
                .find((e) => e.name == "HaltedVault")
                .code.toString(16);

            await vaultClient.updateHaltFlags(
                owner,
                VaultFlags.HaltDepositsWithdraws
            );

            const qty = 100;
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
        });

        it("Reject rebalance transaction if vault is halted", async function () {
            const errorCode = program.idl.errors
                .find((e) => e.name == "HaltedVault")
                .code.toString(16);

            await vaultClient.updateHaltFlags(owner, VaultFlags.HaltReconciles);

            try {
                await performRebalance();
                assert.fail("Rebalance should be rejected but was not.");
            } catch (err) {
                assert.isTrue(
                    err.message.includes(errorCode),
                    `Error code ${errorCode} not included in error message: ${err}`
                );
            }
        });

        it("Update halt flags", async function () {
            const flags = 0;
            const tx = await vaultClient.updateHaltFlags(owner, flags);
            await provider.connection.confirmTransaction(tx, "singleGossip");
            await vaultClient.reload();
            assert.equal(flags, vaultClient.getHaltFlags());
        });

        it("Update yield source flags", async function () {
            const flags = 1 | (1 << 2);
            const tx = await vaultClient.updateYieldSourceFlags(owner, flags);
            await provider.connection.confirmTransaction(tx, "singleGossip");
            await vaultClient.reload();
            assert.equal(flags, vaultClient.getYieldSourceFlags());
        });

        it("Reject invalid flags update", async function () {
            const errorCode = program.idl.errors
                .find((e) => e.name == "InvalidVaultFlags")
                .code.toString(16);

            const oldFlags = vaultClient.getHaltFlags();

            try {
                const tx = await vaultClient.updateHaltFlags(owner, 23);
                await provider.connection.confirmTransaction(
                    tx,
                    "singleGossip"
                );
                assert.fail("Transaction should be rejected but was not.");
            } catch (err) {
                assert.isTrue(
                    err.message.includes(errorCode),
                    `Error code ${errorCode} not included in error message: ${err}`
                );
            }

            await vaultClient.reload();

            assert.equal(oldFlags, vaultClient.getHaltFlags());
        });

        it("Reject unauthorized flags update", async function () {
            const errorCode = "0x8d";

            const noPermissionUser = Keypair.generate();
            const oldFlags = vaultClient.getHaltFlags();

            try {
                const tx = await vaultClient.updateHaltFlags(
                    noPermissionUser,
                    1
                );
                await provider.connection.confirmTransaction(
                    tx,
                    "singleGossip"
                );
                assert.fail("Transaction should be rejected but was not.");
            } catch (err) {
                assert.isTrue(
                    err.message.includes(errorCode),
                    `Error code ${errorCode} not included in error message: ${err}`
                );
            }

            await vaultClient.reload();

            assert.equal(oldFlags, vaultClient.getHaltFlags());
        });
    }

    function testRebalanceWithdraw(
        expectedSolendRatio: number,
        expectedPortRatio: number,
        expectedJetRatio: number,
        rebalanceMode: RebalanceMode = RebalanceModes.calculator,
        solendAvailable: boolean = true,
        portAvailable: boolean = true,
        jetAvailable: boolean = true
    ) {
        // NOTE: should not be divisible by the number of markets, 3 in this case
        // The TODO to correct this is below
        const depositQty = 1024502;

        before(async () => {
            await mintReserveToken(userReserveTokenAccount, depositQty);
            await depositToVault(depositQty);
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

                if (solendAvailable) {
                    assert.equal(
                        (
                            await vaultClient.getVaultSolendLpTokenAccountValue()
                        ).lamports.toNumber(),
                        0
                    );
                }
                if (portAvailable) {
                    assert.equal(
                        (
                            await vaultClient.getVaultPortLpTokenAccountValue()
                        ).lamports.toNumber(),
                        0
                    );
                }
                if (jetAvailable) {
                    assert.equal(
                        (
                            await vaultClient.getVaultJetLpTokenAccountValue()
                        ).lamports.toNumber(),
                        0
                    );
                }
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

                if (solendAvailable) {
                    assert.equal(
                        (
                            await vaultClient.getVaultSolendLpTokenAccountValue()
                        ).lamports.toNumber(),
                        0
                    );
                }
                if (portAvailable) {
                    assert.equal(
                        (
                            await vaultClient.getVaultPortLpTokenAccountValue()
                        ).lamports.toNumber(),
                        0
                    );
                }
                if (jetAvailable) {
                    assert.equal(
                        (
                            await vaultClient.getVaultJetLpTokenAccountValue()
                        ).lamports.toNumber(),
                        0
                    );
                }
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

                if (solendAvailable) {
                    assert.equal(
                        (
                            await vaultClient.getVaultSolendLpTokenAccountValue()
                        ).lamports.toNumber(),
                        0
                    );
                }
                if (portAvailable) {
                    assert.equal(
                        (
                            await vaultClient.getVaultPortLpTokenAccountValue()
                        ).lamports.toNumber(),
                        0
                    );
                }
                if (jetAvailable) {
                    assert.equal(
                        (
                            await vaultClient.getVaultJetLpTokenAccountValue()
                        ).lamports.toNumber(),
                        0
                    );
                }
            });
        }

        it("Rebalances", async () => {
            await performRebalance({
                solend: expectedSolendRatio * 10000,
                port: expectedPortRatio * 10000,
                jet: expectedJetRatio * 10000,
            });

            // TODO Resolve the difference
            // Use isAtMost because on-chain rust program handles rounding differently than TypeScript.
            // However the difference should not exceet 1 token.
            const maxDiffAllowed = 1;
            const totalValue = await getVaultTotalValue();
            assert.isAtMost(
                Math.abs(totalValue - Math.floor(depositQty)),
                maxDiffAllowed
            );

            if (solendAvailable) {
                const solendValue = (
                    await vaultClient.getVaultSolendLpTokenAccountValue()
                ).lamports.toNumber();
                assert.isAtMost(
                    Math.abs(
                        solendValue -
                            Math.floor(depositQty * expectedSolendRatio)
                    ),
                    maxDiffAllowed
                );
            }
            if (portAvailable) {
                const portValue = (
                    await vaultClient.getVaultPortLpTokenAccountValue()
                ).lamports.toNumber();
                assert.isAtMost(
                    Math.abs(
                        portValue - Math.floor(depositQty * expectedPortRatio)
                    ),
                    maxDiffAllowed
                );
            }
            if (jetAvailable) {
                const jetValue = (
                    await vaultClient.getVaultJetLpTokenAccountValue()
                ).lamports.toNumber();
                assert.isAtMost(
                    Math.abs(
                        jetValue - Math.floor(depositQty * expectedJetRatio)
                    ),
                    maxDiffAllowed
                );
            }
        });

        it("Withdraws", async function () {
            const oldVaultValue = await getVaultTotalValue();
            const oldLpTokenSupply = await getLpTokenSupply();

            const withdrawQty = 922051;
            await withdrawFromVault(withdrawQty);

            const newUserReserveBalance = await getUserReserveTokenBalance();
            const newVaultValue = await getVaultTotalValue();
            const newLpTokenSupply = await getLpTokenSupply();

            const actualWithdrawAmount = oldVaultValue - newVaultValue;

            // Allow max different of 1 token because of rounding error.
            const maxDiffAllowed = 1;
            assert.isAtMost(
                Math.abs(actualWithdrawAmount - withdrawQty),
                maxDiffAllowed
            );
            // Actual should <= requested because we rounds down.
            assert.isAtMost(actualWithdrawAmount, withdrawQty);
            assert.equal(oldLpTokenSupply - newLpTokenSupply, withdrawQty);
            assert.equal(newUserReserveBalance, actualWithdrawAmount);
        });
    }

    async function sleep(t: number) {
        return new Promise((res) => setTimeout(res, t));
    }

    function testFees(
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
            const oldConfig = vaultClient.getVaultConfig();
            const newConfig = {
                ...oldConfig,
                feeCarryBps: oldConfig.feeCarryBps / 2,
                feeMgmtBps: oldConfig.feeMgmtBps / 2,
                referralFeePct: oldConfig.referralFeePct / 2,
            };
            const txSig = await vaultClient.updateConfig(owner, newConfig);
            await provider.connection.confirmTransaction(txSig, "singleGossip");

            await vaultClient.reload();
            assert.equal(
                newConfig.feeCarryBps,
                vaultClient.getCarryFee().asBps().toNumber()
            );
            assert.equal(
                newConfig.feeMgmtBps,
                vaultClient.getManagementFee().asBps().toNumber()
            );
            assert.equal(
                newConfig.referralFeePct,
                vaultClient.getReferralFeeSplit().asPercent().toNumber()
            );
        });

        it("Reject invalid fee rates", async function () {
            const errorCode = program.idl.errors
                .find((e) => e.name == "InvalidReferralFeeConfig")
                .code.toString(16);

            const oldConfig = vaultClient.getVaultConfig();
            const newConfig = {
                ...oldConfig,
                feeCarryBps: oldConfig.feeCarryBps / 2,
                feeMgmtBps: oldConfig.feeMgmtBps / 2,
                referralFeePct: 110,
            };
            try {
                const txSig = await vaultClient.updateConfig(owner, newConfig);
                await provider.connection.confirmTransaction(
                    txSig,
                    "singleGossip"
                );
                assert.fail("Transaction should be rejected but was not.");
            } catch (err) {
                assert.isTrue(
                    err.message.includes(errorCode),
                    `Error code ${errorCode} not included in error message: ${err}`
                );
            }

            await vaultClient.reload();
            assert.equal(
                oldConfig.feeCarryBps,
                vaultClient.getCarryFee().asBps().toNumber()
            );
            assert.equal(
                oldConfig.feeMgmtBps,
                vaultClient.getManagementFee().asBps().toNumber()
            );
            assert.equal(
                oldConfig.referralFeePct,
                vaultClient.getReferralFeeSplit().asPercent().toNumber()
            );
        });
    }

    describe("Equal allocation strategy", () => {
        describe("Deposit and withdrawal", () => {
            before(initLendingMarkets);
            before(async function () {
                await initializeVault({
                    strategyType: { [StrategyTypes.equalAllocation]: {} },
                    rebalanceMode: { [RebalanceModes.calculator]: {} },
                });
            });
            testDepositAndWithdrawal();
        });

        describe("Deposit cap and vault flags", () => {
            before(initLendingMarkets);
            before(async function () {
                await initializeVault({
                    depositCap: new anchor.BN(vaultDepositCap),
                    strategyType: { [StrategyTypes.equalAllocation]: {} },
                    rebalanceMode: { [RebalanceModes.calculator]: {} },
                });
            });
            testDepositCap();
            testVaultFlags();
        });

        describe("Rebalance", () => {
            before(initLendingMarkets);
            before(async function () {
                await initializeVault({
                    strategyType: { [StrategyTypes.equalAllocation]: {} },
                    rebalanceMode: { [RebalanceModes.calculator]: {} },
                });
            });
            testRebalanceWithdraw(1 / 3, 1 / 3, 1 / 3);
        });

        describe("Fees", () => {
            const feeMgmtBps = 10000;
            const feeCarryBps = 10000;
            const referralFeePct = 20;
            before(initLendingMarkets);
            before(async function () {
                await initializeVault({
                    feeCarryBps,
                    feeMgmtBps,
                    referralFeePct,
                    strategyType: { [StrategyTypes.equalAllocation]: {} },
                    rebalanceMode: { [RebalanceModes.calculator]: {} },
                });
            });
            testFees(feeCarryBps, feeMgmtBps, referralFeePct);
        });
    });

    describe("Max yield calculator", () => {
        describe("Rebalance", () => {
            before(initLendingMarkets);
            before(async function () {
                await initializeVault({ allocationCapPct: vaultAllocationCap });
            });

            testRebalanceWithdraw(
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
                await initializeVault({
                    allocationCapPct: vaultAllocationCap,
                    strategyType: { [StrategyTypes.maxYield]: {} },
                    rebalanceMode: { [RebalanceModes.proofChecker]: {} },
                });
            });
            testRebalanceWithdraw(
                1 - vaultAllocationCap / 100,
                0,
                vaultAllocationCap / 100,
                rebalanceMode
            );
        });
    });

    describe("Disabled pools", () => {
        describe("Rebalance with equal allocation strategy missing 1 pool", () => {
            const rebalanceMode = RebalanceModes.calculator;
            before(initLendingMarkets);
            before(async function () {
                await initializeVault(
                    {
                        allocationCapPct: vaultAllocationCap,
                        strategyType: { [StrategyTypes.equalAllocation]: {} },
                        rebalanceMode: { [RebalanceModes.calculator]: {} },
                    },
                    true,
                    false,
                    true
                );
            });

            it("Initialize fewer yield sources", async function () {
                assert.equal(0b101, vaultClient.getYieldSourceFlags());
            });

            it("Update and adjust allocation cap", async function () {
                const oldConfig = vaultClient.getVaultConfig();
                const newConfig = {
                    ...oldConfig,
                    allocationCapPct: 40,
                };
                const tx = await vaultClient.updateConfig(owner, newConfig);
                await provider.connection.confirmTransaction(
                    tx,
                    "singleGossip"
                );
                await vaultClient.reload();
                assert.equal(vaultClient.getVaultConfig().allocationCapPct, 51);
            });

            testRebalanceWithdraw(
                1 / 2,
                0,
                1 / 2,
                rebalanceMode,
                true,
                false,
                true
            );
        });

        describe("Rebalance with equal allocation strategy missing 2 pools", () => {
            const rebalanceMode = RebalanceModes.calculator;
            before(initLendingMarkets);
            before(async function () {
                await initializeVault(
                    {
                        allocationCapPct: 100,
                        strategyType: { [StrategyTypes.equalAllocation]: {} },
                        rebalanceMode: { [RebalanceModes.calculator]: {} },
                    },
                    true,
                    false,
                    false
                );
            });

            it("Initialize fewer yield sources", async function () {
                assert.equal(0b001, vaultClient.getYieldSourceFlags());
            });

            testRebalanceWithdraw(1, 0, 0, rebalanceMode, true, false, false);
        });

        describe("Rebalance with max yield strategy", () => {
            const rebalanceMode = RebalanceModes.calculator;
            before(initLendingMarkets);
            before(async function () {
                await initializeVault(
                    {
                        allocationCapPct: vaultAllocationCap,
                        strategyType: { [StrategyTypes.maxYield]: {} },
                        rebalanceMode: { [RebalanceModes.calculator]: {} },
                    },
                    true,
                    true,
                    false
                );
            });

            it("Initialize fewer yield sources", async function () {
                assert.equal(0b011, vaultClient.getYieldSourceFlags());
            });

            testRebalanceWithdraw(
                vaultAllocationCap / 100,
                1 - vaultAllocationCap / 100,
                0,
                rebalanceMode,
                true,
                true,
                false
            );
        });
    });

    describe("Stake Port LP token and claim reward", () => {
        const rebalanceMode = RebalanceModes.calculator;
        before(initLendingMarkets);
        before(async function () {
            await initializeVault(
                {
                    allocationCapPct: 100,
                    strategyType: { [StrategyTypes.equalAllocation]: {} },
                    rebalanceMode: { [RebalanceModes.calculator]: {} },
                },
                false,
                true,
                false
            );
        });

        const depositQty = 1024502;

        before(async () => {
            await mintReserveToken(userReserveTokenAccount, depositQty);
            await depositToVault(depositQty);
        });

        it("Stake port LP token when rebalancing", async () => {
            await performRebalance({
                solend: 0,
                port: 10000,
                jet: 0,
            });

            const maxDiffAllowed = 1;
            const totalValue = await getVaultTotalValue();
            assert.isAtMost(
                Math.abs(totalValue - Math.floor(depositQty)),
                maxDiffAllowed
            );
            const portValue = (
                await vaultClient.getVaultPortLpTokenAccountValue()
            ).lamports.toNumber();
            assert.isAtMost(
                Math.abs(portValue - Math.floor(depositQty)),
                maxDiffAllowed
            );

            const stakingAccountRaw = await provider.connection.getAccountInfo(
                new PublicKey(port.accounts.vaultPortStakeAccount)
            );
            const stakingAccount = StakeAccount.fromRaw({
                pubkey: port.accounts.vaultPortStakeAccount,
                account: stakingAccountRaw,
            });
            assert.equal(
                stakingAccount.getDepositAmount().toU64().toNumber(),
                depositQty
            );
        });

        it("Withdraws", async function () {
            const oldVaultValue = await getVaultTotalValue();
            const oldLpTokenSupply = await getLpTokenSupply();

            const withdrawQty = 922051;
            await withdrawFromVault(withdrawQty);

            const newUserReserveBalance = await getUserReserveTokenBalance();
            const newVaultValue = await getVaultTotalValue();
            const newLpTokenSupply = await getLpTokenSupply();

            const stakingAccountRaw = await provider.connection.getAccountInfo(
                new PublicKey(port.accounts.vaultPortStakeAccount)
            );
            const stakingAccount = StakeAccount.fromRaw({
                pubkey: port.accounts.vaultPortStakeAccount,
                account: stakingAccountRaw,
            });
            assert.equal(
                stakingAccount.getDepositAmount().toU64().toNumber(),
                depositQty - withdrawQty
            );

            // Allow max different of 1 token because of rounding error.
            const actualWithdrawAmount = oldVaultValue - newVaultValue;
            const maxDiffAllowed = 1;
            assert.isAtMost(
                Math.abs(actualWithdrawAmount - withdrawQty),
                maxDiffAllowed
            );
            // Actual should <= requested because we rounds down.
            assert.isAtMost(actualWithdrawAmount, withdrawQty);
            assert.equal(oldLpTokenSupply - newLpTokenSupply, withdrawQty);
            assert.equal(newUserReserveBalance, actualWithdrawAmount);
        });

        it("Claim reward", async function () {
            const accumulatedRewardAmount =
                await port.getUnclaimedStakingRewards(program);
            assert.isAtLeast(accumulatedRewardAmount, 1);

            await vaultClient.claimPortReward();

            const stakingAccountRaw2 = await provider.connection.getAccountInfo(
                new PublicKey(port.accounts.vaultPortStakeAccount)
            );
            const stakingAccount2 = StakeAccount.fromRaw({
                pubkey: port.accounts.vaultPortStakeAccount,
                account: stakingAccountRaw2,
            });
            const rewardAmountAfterClaiming =
                await port.getUnclaimedStakingRewards(program);
            assert.equal(rewardAmountAfterClaiming, 0);
            
            const mint = port.accounts.stakingRewardTokenMint;
            const rewardToken = new Token(
                program.provider.connection,
                mint,
                TOKEN_PROGRAM_ID,
                Keypair.generate()
            );
            const claimedRewardAmount = (
                await rewardToken.getAccountInfo(
                    port.accounts.vaultPortRewardToken
                )
            ).amount.toNumber();
            assert.isAtLeast(claimedRewardAmount, accumulatedRewardAmount);
        });
    });
});
