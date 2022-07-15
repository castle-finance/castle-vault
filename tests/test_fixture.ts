import { assert } from "chai";
import * as anchor from "@project-serum/anchor";
import { TOKEN_PROGRAM_ID, Token, NATIVE_MINT } from "@solana/spl-token";
import {
    Keypair,
    PublicKey,
    TransactionSignature,
    Transaction,
} from "@solana/web3.js";
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

export interface YieldSrcSelector {
    solend: boolean;
    port: boolean;
    jet: boolean;
}

export class TestFixture {
    provider = anchor.Provider.env();
    wallet = this.provider.wallet as anchor.Wallet;
    program = anchor.workspace.CastleVault as anchor.Program<CastleVault>;
    owner = Keypair.generate();
    pythProduct = new PublicKey("ALP8SdU9oARYVLgLR7LrqMNCYBnhtnQz1cj6bwgwQmgj");
    pythPrice = new PublicKey("H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG");
    switchboardFeed = new PublicKey(
        "AdtRGGhmqvom3Jemp5YNrxd9q9unX36BZk1pujkkXijL"
    );

    // suppress noisy tx failure logs for "reject __" tests
    enableSuppressLogs = true;
    oldConsoleError;

    slotsPerYear = 63072000;
    initialReserveAmount = 10000000;
    initialCollateralRatio = 1.0;
    referralFeeOwner = Keypair.generate().publicKey;
    vaultDepositCap = 10 * 10 ** 9;
    vaultAllocationCap = 76;

    reserveToken: Token;

    jet: JetReserveAsset;
    solend: SolendReserveAsset;
    port: PortReserveAsset;

    vaultClient: VaultClient;
    userReserveTokenAccount: PublicKey;

    constructor() {}

    suppressLogs() {
        if (this.enableSuppressLogs) {
            this.oldConsoleError = console.error;
            console.error = function () {
                const _noop = "";
            };
        }
    }

    restoreLogs() {
        if (this.enableSuppressLogs) {
            console.error = this.oldConsoleError;
        }
    }

    async fetchSlots(txs: string[]): Promise<number[]> {
        const slots = (
            await Promise.all(
                txs.map((tx) => {
                    return this.provider.connection.getParsedConfirmedTransaction(
                        tx,
                        "confirmed"
                    );
                })
            )
        ).map((res) => res.slot);

        return slots;
    }

    calcReserveToLp(
        amount: anchor.BN,
        lpSupply: anchor.BN,
        vaultValue: anchor.BN
    ): anchor.BN {
        return lpSupply.mul(amount).div(vaultValue);
    }

    splitFees(
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

    async calculateFees(
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
            .div(new anchor.BN(this.slotsPerYear))
            .mul(new anchor.BN(dt));

        const tFees = mgmtFees;
        const tLpFees = this.calcReserveToLp(tFees, lpMintSupply, vaultBalance);

        const [primFee, refFee] = this.splitFees(tLpFees, referralFeePct);

        return { primary: primFee, referral: refFee };
    }

    async initLendingMarkets(
        yieldSrcOptions: YieldSrcSelector = {
            solend: true,
            port: true,
            jet: true,
        },
        createPortSubReward: boolean = false
    ) {
        const sig = await this.provider.connection.requestAirdrop(
            this.owner.publicKey,
            1000000000
        );

        const supplSig = await this.provider.connection.requestAirdrop(
            this.referralFeeOwner,
            1000000000
        );

        await this.provider.connection.confirmTransaction(sig, "singleGossip");
        await this.provider.connection.confirmTransaction(
            supplSig,
            "singleGossip"
        );

        this.reserveToken = await Token.createMint(
            this.provider.connection,
            this.owner,
            this.owner.publicKey,
            null,
            2,
            TOKEN_PROGRAM_ID
        );

        const ownerReserveTokenAccount = await this.reserveToken.createAccount(
            this.owner.publicKey
        );

        await this.reserveToken.mintTo(
            ownerReserveTokenAccount,
            this.owner,
            [],
            3 * this.initialReserveAmount
        );

        const pythProgram = new PublicKey(
            "FsJ3A3u2vn5cTVofAjvy6y5kwABJAqYWpe4975bi2epH"
        );
        const switchboardProgram = new PublicKey(
            "DtmE9D2CSB4L5D6A15mraeEjrGMm6auWVzgaD8hK2tZM"
        );

        if (yieldSrcOptions.solend) {
            this.solend = await SolendReserveAsset.initialize(
                this.provider,
                this.owner,
                this.wallet,
                this.reserveToken.publicKey,
                pythProgram,
                switchboardProgram,
                this.pythProduct,
                this.pythPrice,
                this.switchboardFeed,
                ownerReserveTokenAccount,
                this.initialReserveAmount
            );
        }

        if (yieldSrcOptions.port) {
            this.port = await PortReserveAsset.initialize(
                this.provider,
                this.owner,
                this.reserveToken.publicKey,
                this.pythPrice,
                ownerReserveTokenAccount,
                this.initialReserveAmount,
                createPortSubReward
            );
        }

        if (yieldSrcOptions.jet) {
            this.jet = await JetReserveAsset.initialize(
                this.provider,
                this.wallet,
                this.owner,
                NATIVE_MINT,
                this.reserveToken,
                this.pythPrice,
                this.pythProduct,
                ownerReserveTokenAccount,
                this.initialReserveAmount
            );

            const jetBorrowedAmt = this.initialReserveAmount / 2;
            const jetBorrowTxs = await this.jet.borrow(
                this.owner,
                ownerReserveTokenAccount,
                jetBorrowedAmt
            );
            await this.provider.connection.confirmTransaction(
                jetBorrowTxs[jetBorrowTxs.length - 1],
                "finalized"
            );
        }
    }

    async initializeVault(
        config: VaultConfig,
        yieldSrcOptions: YieldSrcSelector = {
            solend: true,
            port: true,
            jet: true,
        },
    ) {
        this.vaultClient = await VaultClient.initialize(
            this.provider,
            this.provider.wallet as anchor.Wallet,
            DeploymentEnvs.devnetStaging,
            this.reserveToken.publicKey,
            this.owner.publicKey,
            this.referralFeeOwner,
            config,
            this.program
        );

        await Promise.all([
            yieldSrcOptions.solend
                ? await this.vaultClient.initializeSolend(
                      this.provider.wallet as anchor.Wallet,
                      this.solend,
                      this.owner
                  )
                : {},
            yieldSrcOptions.port
                ? await this.vaultClient.initializePort(
                      this.provider.wallet as anchor.Wallet,
                      this.port,
                      this.owner
                  )
                : {},
            yieldSrcOptions.jet
                ? await this.vaultClient.initializeJet(
                      this.provider.wallet as anchor.Wallet,
                      this.jet,
                      this.owner
                  )
                : {},
        ]);

        if (yieldSrcOptions.port) {
            await this.vaultClient.initializePortAdditionalState(
                this.provider.wallet as anchor.Wallet,
                this.owner
            );
            await this.vaultClient.initializePortRewardAccounts(
                this.provider.wallet as anchor.Wallet,
                this.owner,
                this.provider,
                DeploymentEnvs.devnetStaging,
                this.program
            );
            await this.vaultClient.loadPortAdditionalAccounts();
        }

        await this.vaultClient.reload();

        this.userReserveTokenAccount = await this.reserveToken.createAccount(
            this.wallet.publicKey
        );
    }

    async getReserveTokenBalance(account: PublicKey): Promise<number> {
        const info = await this.vaultClient.getReserveTokenAccountInfo(account);
        return info.amount.toNumber();
    }

    async getUserReserveTokenBalance(): Promise<number> {
        const account = await this.vaultClient.getUserReserveTokenAccount(
            this.wallet.publicKey
        );
        return await this.getReserveTokenBalance(account);
    }

    async getVaultReserveTokenBalance(): Promise<number> {
        return await this.getReserveTokenBalance(
            this.vaultClient.getVaultReserveTokenAccount()
        );
    }

    async getVaultTotalValue(): Promise<number> {
        return (await this.vaultClient.getTotalValue()).lamports.toNumber();
    }

    async getUserLpTokenBalance(): Promise<number> {
        const account = await this.vaultClient.getUserLpTokenAccount(
            this.wallet.publicKey
        );
        const info = await this.vaultClient.getLpTokenAccountInfo(account);
        return info.amount.toNumber();
    }

    async getLpTokenSupply(): Promise<number> {
        await this.vaultClient.reload();
        return this.vaultClient.getVaultState().lpTokenSupply.toNumber();
    }

    async mintReserveToken(receiver: PublicKey, qty: number) {
        await this.reserveToken.mintTo(receiver, this.owner, [], qty);
    }

    async depositToVault(qty: number): Promise<TransactionSignature[]> {
        await this.mintReserveToken(this.userReserveTokenAccount, qty);
        const txs = await this.vaultClient.deposit(
            this.wallet,
            qty,
            this.userReserveTokenAccount
        );
        await this.provider.connection.confirmTransaction(
            txs[txs.length - 1],
            "singleGossip"
        );
        return txs;
    }

    async withdrawFromVault(qty: number): Promise<TransactionSignature[]> {
        const txs = await this.vaultClient.withdraw(this.wallet, qty);
        await this.provider.connection.confirmTransaction(
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
    async performRebalance(
        proposedWeights?: ProposedWeightsBps,
        rebalanceOnly: boolean = false
    ): Promise<TransactionSignature[]> {
        let txSigs = null;

        if (rebalanceOnly) {
            const preRefresh = this.vaultClient.getPreRefreshTxs().map((tx) => {
                return { tx: tx, signers: [] };
            });
            const txs = [
                ...preRefresh,
                {
                    tx: await this.vaultClient.getRebalanceTx(proposedWeights),
                    signers: [],
                },
            ];
            txSigs = await this.provider.sendAll(txs);
        } else {
            txSigs = await this.vaultClient.rebalance(proposedWeights);
        }
        await this.provider.connection.confirmTransaction(
            txSigs[txSigs.length - 1],
            "singleGossip"
        );
        return txSigs;
    }

    async sleep(t: number) {
        return new Promise((res) => setTimeout(res, t));
    }

    testDepositAndWithdrawal() {
        it("Initializes a vault", async () => {
            assert.isNotNull(this.vaultClient.getLpTokenMintInfo());
            assert.isNotNull(this.vaultClient.getVaultReserveTokenAccount());
            assert.isNotNull(this.vaultClient.getFeeReceiverAccountInfo());
            assert.isNotNull(this.vaultClient.getReferralFeeReceiverAccountInfo());
            assert.equal(
                this.vaultClient.getDepositCap().lamports.toString(),
                "18446744073709551615"
            );
            assert.equal(
                this.vaultClient.getAllocationCap().asPercent().toNumber(),
                100
            );
            assert.equal(0b111, this.vaultClient.getYieldSourceFlags());
        });

        it("Deposits to vault reserves", async () => {
            const qty = 100;

            await this.mintReserveToken(this.userReserveTokenAccount, qty);
            await this.depositToVault(qty);

            const userReserveBalance = await this.getReserveTokenBalance(
                this.userReserveTokenAccount
            );
            const vaultReserveBalance = await this.getVaultReserveTokenBalance();
            const userLpBalance = await this.getUserLpTokenBalance();
            const lpTokenSupply = await this.getLpTokenSupply();

            assert.equal(userReserveBalance, 0);
            assert.equal(vaultReserveBalance, qty);
            assert.equal(userLpBalance, qty * this.initialCollateralRatio);
            assert.equal(lpTokenSupply, qty * this.initialCollateralRatio);
        });

        it("Withdraw funds from vault", async () => {
            const oldVaultReserveBalance = await this.getVaultReserveTokenBalance();
            const oldLpTokenSupply = await this.getLpTokenSupply();

            const qty = 70;
            await this.withdrawFromVault(qty);

            const newUserReserveBalance = await this.getUserReserveTokenBalance();
            const newVaultReserveBalance = await this.getVaultReserveTokenBalance();
            const newLpTokenSupply = await this.getLpTokenSupply();

            assert.equal(oldVaultReserveBalance - newVaultReserveBalance, qty);
            assert.equal(oldLpTokenSupply - newLpTokenSupply, qty);
            assert.equal(newUserReserveBalance, qty);
        });
    }

    testDepositCap() {
        it("Initialize vault correctly", async () => {
            assert.isNotNull(this.vaultClient.getLpTokenMintInfo());
            assert.isNotNull(this.vaultClient.getVaultReserveTokenAccount());
            assert.isNotNull(this.vaultClient.getFeeReceiverAccountInfo());
            assert.isNotNull(this.vaultClient.getReferralFeeReceiverAccountInfo());
            assert.equal(
                this.vaultClient.getDepositCap().lamports.toNumber(),
                this.vaultDepositCap
            );
            assert.equal(
                this.vaultClient.getAllocationCap().asPercent().toNumber(),
                100
            );
        });

        it("Reject transaction if deposit cap is reached", async () => {
            this.suppressLogs()

            const depositCapErrorCode = this.program.idl.errors
                .find((e) => e.name == "DepositCapError")
                .code.toString(16);

            const qty = this.vaultDepositCap + 100;
            await this.mintReserveToken(this.userReserveTokenAccount, qty);

            try {
                await this.depositToVault(qty);
                assert.fail("Deposit should be rejected but was not.");
            } catch (err) {
                assert.isTrue(
                    err.message.includes(depositCapErrorCode),
                    `Error code ${depositCapErrorCode} not included in error message: ${err}`
                );
            }

            const userReserveBalance = await this.getReserveTokenBalance(
                this.userReserveTokenAccount
            );
            const vaultReserveBalance = await this.getVaultReserveTokenBalance();
            const userLpBalance = await this.getUserLpTokenBalance();
            const lpTokenSupply = await this.getLpTokenSupply();

            assert.equal(userReserveBalance, qty);
            assert.equal(vaultReserveBalance, 0);
            assert.equal(userLpBalance, 0);
            assert.equal(lpTokenSupply, 0);

            this.restoreLogs()
        });

        it("Update deposit cap", async () => {
            const oldConfig = this.vaultClient.getVaultConfig();
            const newConfig = {
                ...oldConfig,
                depositCap: oldConfig.depositCap.mul(new anchor.BN(24)),
            };
            const tx = await this.vaultClient.updateConfig(this.owner, newConfig);
            await this.provider.connection.confirmTransaction(tx, "singleGossip");
            await this.vaultClient.reload();
            assert.equal(
                newConfig.depositCap.toNumber(),
                this.vaultClient.getDepositCap().lamports.toNumber()
            );
        });

        it("Reject unauthorized config update", async () => {
            this.suppressLogs()

            const errorCode = "0x8d";
            const noPermissionUser = Keypair.generate();

            const oldConfig = this.vaultClient.getVaultConfig();
            const newConfig = {
                ...oldConfig,
                depositCap: oldConfig.depositCap.mul(new anchor.BN(24)),
            };

            try {
                const tx = await this.vaultClient.updateConfig(
                    noPermissionUser,
                    newConfig
                );
                await this.provider.connection.confirmTransaction(
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

            await this.vaultClient.reload();
            assert.equal(
                oldConfig.depositCap.toNumber(),
                this.vaultClient.getDepositCap().lamports.toNumber()
            );

            this.restoreLogs()
        });
    }

    testVaultFlags() {
        it("Reject deposit transaction if vault is halted", async () => {
            this.suppressLogs()

            const depositCapErrorCode = this.program.idl.errors
                .find((e) => e.name == "HaltedVault")
                .code.toString(16);

            await this.vaultClient.updateHaltFlags(
                this.owner,
                VaultFlags.HaltDepositsWithdraws
            );

            const qty = 100;
            await this.mintReserveToken(this.userReserveTokenAccount, qty);

            try {
                await this.depositToVault(qty);
                assert.fail("Deposit should be rejected but was not.");
            } catch (err) {
                assert.isTrue(
                    err.message.includes(depositCapErrorCode),
                    `Error code ${depositCapErrorCode} not included in error message: ${err}`
                );
            }

            this.restoreLogs()
        });

        it("Reject rebalance transaction if vault is halted", async () => {
            this.suppressLogs()

            const errorCode = this.program.idl.errors
                .find((e) => e.name == "HaltedVault")
                .code.toString(16);

            await this.vaultClient.updateHaltFlags(this.owner, VaultFlags.HaltReconciles);

            try {
                await this.performRebalance();
                assert.fail("Rebalance should be rejected but was not.");
            } catch (err) {
                assert.isTrue(
                    err.message.includes(errorCode),
                    `Error code ${errorCode} not included in error message: ${err}`
                );
            }

            this.restoreLogs()
        });

        it("Update halt flags", async () => {
            const flags = 0;
            const tx = await this.vaultClient.updateHaltFlags(this.owner, flags);
            await this.provider.connection.confirmTransaction(tx, "singleGossip");
            await this.vaultClient.reload();
            assert.equal(flags, this.vaultClient.getHaltFlags());
        });

        it("Update yield source flags", async () => {
            const flags = 1 | (1 << 2);
            const tx = await this.vaultClient.updateYieldSourceFlags(this.owner, flags);
            await this.provider.connection.confirmTransaction(tx, "singleGossip");
            await this.vaultClient.reload();
            assert.equal(flags, this.vaultClient.getYieldSourceFlags());
        });

        it("Reject invalid flags update", async () => {
            this.suppressLogs()

            const errorCode = this.program.idl.errors
                .find((e) => e.name == "InvalidVaultFlags")
                .code.toString(16);

            const oldFlags = this.vaultClient.getHaltFlags();

            try {
                const tx = await this.vaultClient.updateHaltFlags(this.owner, 23);
                await this.provider.connection.confirmTransaction(
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

            await this.vaultClient.reload();

            assert.equal(oldFlags, this.vaultClient.getHaltFlags());

            this.restoreLogs()
        });

        it("Reject unauthorized flags update", async () => {
            this.suppressLogs()

            const errorCode = "0x8d";

            const noPermissionUser = Keypair.generate();
            const oldFlags = this.vaultClient.getHaltFlags();

            try {
                const tx = await this.vaultClient.updateHaltFlags(
                    noPermissionUser,
                    1
                );
                await this.provider.connection.confirmTransaction(
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

            await this.vaultClient.reload();

            assert.equal(oldFlags, this.vaultClient.getHaltFlags());

            this.restoreLogs()
        });
    }

    testRebalanceWithdraw(
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
            await this.mintReserveToken(this.userReserveTokenAccount, depositQty);
            await this.depositToVault(depositQty);
        });

        if (rebalanceMode == RebalanceModes.proofChecker) {
            it("Rejects tx if weights don't equal 100%", async () => {
                this.suppressLogs()

                const errorCode = this.program.idl.errors
                    .find((e) => e.name == "InvalidProposedWeights")
                    .code.toString(16);
                try {
                    await this.performRebalance(
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
                            await this.vaultClient.getVaultSolendLpTokenAccountValue()
                        ).lamports.toNumber(),
                        0
                    );
                }
                if (portAvailable) {
                    assert.equal(
                        (
                            await this.vaultClient.getVaultPortLpTokenAccountValue()
                        ).lamports.toNumber(),
                        0
                    );
                }
                if (jetAvailable) {
                    assert.equal(
                        (
                            await this.vaultClient.getVaultJetLpTokenAccountValue()
                        ).lamports.toNumber(),
                        0
                    );
                }

                this.restoreLogs()
            });

            it("Rejects tx if weights are suboptimal", async () => {
                this.suppressLogs()

                const errorCode = this.program.idl.errors
                    .find((e) => e.name == "RebalanceProofCheckFailed")
                    .code.toString(16);
                try {
                    await this.performRebalance(
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
                            await this.vaultClient.getVaultSolendLpTokenAccountValue()
                        ).lamports.toNumber(),
                        0
                    );
                }
                if (portAvailable) {
                    assert.equal(
                        (
                            await this.vaultClient.getVaultPortLpTokenAccountValue()
                        ).lamports.toNumber(),
                        0
                    );
                }
                if (jetAvailable) {
                    assert.equal(
                        (
                            await this.vaultClient.getVaultJetLpTokenAccountValue()
                        ).lamports.toNumber(),
                        0
                    );
                }

                this.restoreLogs()
            });

            it("Rejects tx if weights exceeds the cap", async () => {
                this.suppressLogs()

                const errorCode = this.program.idl.errors
                    .find((e) => e.name == "InvalidProposedWeights")
                    .code.toString(16);
                try {
                    await this.performRebalance(
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
                            await this.vaultClient.getVaultSolendLpTokenAccountValue()
                        ).lamports.toNumber(),
                        0
                    );
                }
                if (portAvailable) {
                    assert.equal(
                        (
                            await this.vaultClient.getVaultPortLpTokenAccountValue()
                        ).lamports.toNumber(),
                        0
                    );
                }
                if (jetAvailable) {
                    assert.equal(
                        (
                            await this.vaultClient.getVaultJetLpTokenAccountValue()
                        ).lamports.toNumber(),
                        0
                    );
                }

                this.restoreLogs()
            });
        }

        it("Rebalances", async () => {
            await this.performRebalance({
                solend: expectedSolendRatio * 10000,
                port: expectedPortRatio * 10000,
                jet: expectedJetRatio * 10000,
            });

            // TODO Resolve the difference
            // Use isAtMost because on-chain rust program handles rounding differently than TypeScript.
            // However the difference should not exceet 1 token.
            const maxDiffAllowed = 1;
            const totalValue = await this.getVaultTotalValue();
            assert.isAtMost(
                Math.abs(totalValue - Math.floor(depositQty)),
                maxDiffAllowed
            );

            if (solendAvailable) {
                const solendValue = (
                    await this.vaultClient.getVaultSolendLpTokenAccountValue()
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
                    await this.vaultClient.getVaultPortLpTokenAccountValue()
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
                    await this.vaultClient.getVaultJetLpTokenAccountValue()
                ).lamports.toNumber();
                assert.isAtMost(
                    Math.abs(
                        jetValue - Math.floor(depositQty * expectedJetRatio)
                    ),
                    maxDiffAllowed
                );
            }
        });

        it("Withdraws", async () => {
            const oldVaultValue = await this.getVaultTotalValue();
            const oldLpTokenSupply = await this.getLpTokenSupply();

            const withdrawQty = 922051;
            await this.withdrawFromVault(withdrawQty);

            const newUserReserveBalance = await this.getUserReserveTokenBalance();
            const newVaultValue = await this.getVaultTotalValue();
            const newLpTokenSupply = await this.getLpTokenSupply();

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

    testFees(
        feeCarryBps: number = 0,
        feeMgmtBps: number = 0,
        referalPct: number = 0
    ) {
        it("Collect fees", async () => {
            const qty1 = 5.47 * 10 ** 9;
            await this.mintReserveToken(this.userReserveTokenAccount, qty1);
            const txs1 = await this.depositToVault(qty1);
            const slots0 = await this.fetchSlots(txs1);

            const vaultTotalValue = new anchor.BN(await this.getVaultTotalValue());
            const lpTokenSupply = new anchor.BN(await this.getLpTokenSupply());

            await this.sleep(1000);

            // This is needed to trigger refresh and fee collection
            // Consider adding an API to client library to trigger refresh
            const qty2 = 10;
            await this.mintReserveToken(this.userReserveTokenAccount, qty2);
            const txs2 = await this.depositToVault(qty2);
            const slots1 = await this.fetchSlots(txs2);

            const expectedFees = await this.calculateFees(
                vaultTotalValue,
                lpTokenSupply,
                slots0[slots0.length - 1],
                slots1[slots1.length - 1],
                feeCarryBps,
                feeMgmtBps,
                referalPct
            );

            const referralAccountInfo =
                await this.vaultClient.getReferralFeeReceiverAccountInfo();
            const feeAccountInfo =
                await this.vaultClient.getFeeReceiverAccountInfo();

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

        it("Update fee rates", async () => {
            const oldConfig = this.vaultClient.getVaultConfig();
            const newConfig = {
                ...oldConfig,
                feeCarryBps: oldConfig.feeCarryBps / 2,
                feeMgmtBps: oldConfig.feeMgmtBps / 2,
                referralFeePct: oldConfig.referralFeePct / 2,
            };
            const txSig = await this.vaultClient.updateConfig(this.owner, newConfig);
            await this.provider.connection.confirmTransaction(txSig, "singleGossip");

            await this.vaultClient.reload();
            assert.equal(
                newConfig.feeCarryBps,
                this.vaultClient.getCarryFee().asBps().toNumber()
            );
            assert.equal(
                newConfig.feeMgmtBps,
                this.vaultClient.getManagementFee().asBps().toNumber()
            );
            assert.equal(
                newConfig.referralFeePct,
                this.vaultClient.getReferralFeeSplit().asPercent().toNumber()
            );
        });

        it("Reject invalid fee rates", async () => {
            this.suppressLogs()

            const errorCode = this.program.idl.errors
                .find((e) => e.name == "InvalidReferralFeeConfig")
                .code.toString(16);

            const oldConfig = this.vaultClient.getVaultConfig();
            const newConfig = {
                ...oldConfig,
                feeCarryBps: oldConfig.feeCarryBps / 2,
                feeMgmtBps: oldConfig.feeMgmtBps / 2,
                referralFeePct: 110,
            };
            try {
                const txSig = await this.vaultClient.updateConfig(this.owner, newConfig);
                await this.provider.connection.confirmTransaction(
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

            await this.vaultClient.reload();
            assert.equal(
                oldConfig.feeCarryBps,
                this.vaultClient.getCarryFee().asBps().toNumber()
            );
            assert.equal(
                oldConfig.feeMgmtBps,
                this.vaultClient.getManagementFee().asBps().toNumber()
            );
            assert.equal(
                oldConfig.referralFeePct,
                this.vaultClient.getReferralFeeSplit().asPercent().toNumber()
            );

            this.restoreLogs()
        });
    }

    testPortRewardClaiming(subReward: boolean) {
        const depositQty = 1024502;

        before(async () => {
            await this.depositToVault(depositQty);
        });

        it("Stake port LP token when rebalancing", async () => {
            await this.performRebalance({
                solend: 0,
                port: 10000,
                jet: 0,
            });

            const maxDiffAllowed = 1;
            const totalValue = await this.getVaultTotalValue();
            assert.isAtMost(
                Math.abs(totalValue - Math.floor(depositQty)),
                maxDiffAllowed
            );
            const portValue = (
                await this.vaultClient.getVaultPortLpTokenAccountValue()
            ).lamports.toNumber();
            assert.isAtMost(
                Math.abs(portValue - Math.floor(depositQty)),
                maxDiffAllowed
            );

            const stakingAccountRaw =
                await this.provider.connection.getAccountInfo(
                    new PublicKey(this.port.accounts.vaultPortStakeAccount)
                );
            const stakingAccount = StakeAccount.fromRaw({
                pubkey: this.port.accounts.vaultPortStakeAccount,
                account: stakingAccountRaw,
            });
            assert.equal(
                stakingAccount.getDepositAmount().toU64().toNumber(),
                depositQty
            );
        });

        it("Withdraws", async () => {
            const oldVaultValue = await this.getVaultTotalValue();
            const oldLpTokenSupply = await this.getLpTokenSupply();

            const withdrawQty = 922051;
            await this.withdrawFromVault(withdrawQty);

            const newUserReserveBalance =
                await this.getUserReserveTokenBalance();
            const newVaultValue = await this.getVaultTotalValue();
            const newLpTokenSupply = await this.getLpTokenSupply();

            const stakingAccountRaw =
                await this.provider.connection.getAccountInfo(
                    new PublicKey(this.port.accounts.vaultPortStakeAccount)
                );
            const stakingAccount = StakeAccount.fromRaw({
                pubkey: this.port.accounts.vaultPortStakeAccount,
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

        it("Claim reward", async () => {
            const accumulatedRewardAmount =
                await this.port.getUnclaimedStakingRewards(this.program);
            assert.isAtLeast(accumulatedRewardAmount, 1);

            await this.vaultClient.claimPortReward();

            const stakingAccountRaw2 =
                await this.provider.connection.getAccountInfo(
                    new PublicKey(this.port.accounts.vaultPortStakeAccount)
                );
            const stakingAccount2 = StakeAccount.fromRaw({
                pubkey: this.port.accounts.vaultPortStakeAccount,
                account: stakingAccountRaw2,
            });
            const rewardAmountAfterClaiming =
                await this.port.getUnclaimedStakingRewards(this.program);
            assert.equal(rewardAmountAfterClaiming, 0);

            const mint = this.port.accounts.stakingRewardTokenMint;
            const rewardToken = new Token(
                this.program.provider.connection,
                mint,
                TOKEN_PROGRAM_ID,
                Keypair.generate()
            );
            const claimedRewardAmount = (
                await rewardToken.getAccountInfo(
                    this.port.accounts.vaultPortRewardToken
                )
            ).amount.toNumber();
            assert.isAtLeast(claimedRewardAmount, accumulatedRewardAmount);

            if (subReward) {
                const subRewardMint =
                    this.port.accounts.stakingSubRewardTokenMint;
                const subRewardToken = new Token(
                    this.program.provider.connection,
                    subRewardMint,
                    TOKEN_PROGRAM_ID,
                    Keypair.generate()
                );
                const claimedSubRewardAmount = (
                    await subRewardToken.getAccountInfo(
                        this.port.accounts.vaultPortSubRewardToken
                    )
                ).amount.toNumber();

                assert.isAtLeast(claimedSubRewardAmount, 1);
            }
        });
    }
}
