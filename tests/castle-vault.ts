import { assert } from "chai";
import * as anchor from "@project-serum/anchor";
import { TOKEN_PROGRAM_ID, Token } from "@solana/spl-token";
import {
    Keypair,
    PublicKey,
    Transaction,
    TransactionSignature,
} from "@solana/web3.js";
import { StakeAccount, AssetPrice, MintId } from "@castlefinance/port-sdk";

import {
    SolendReserveAsset,
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
import { OrcaLegacySwap } from "../sdk/src/dex";

describe("castle-vault", () => {
    const provider = anchor.AnchorProvider.env();
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

    // suppress noisy tx failure logs for "reject __" tests
    const enableSuppressLogs = true;
    let oldConsoleError;

    const slotsPerYear = 63072000;
    const initialReserveAmount = 10000000 * 1000;
    const initialCollateralRatio = 1.0;
    const referralFeeOwner = Keypair.generate().publicKey;
    const vaultDepositCap = 10 * 10 ** 9;
    const vaultAllocationCap = 76;

    let reserveToken: Token;

    let solend: SolendReserveAsset;
    let port: PortReserveAsset;
    let orca: OrcaLegacySwap;

    let vaultClient: VaultClient;
    let userReserveTokenAccount: PublicKey;

    function suppressLogs() {
        if (enableSuppressLogs) {
            oldConsoleError = console.error;
            console.error = function () {
                const _noop = "";
            };
        }
    }

    function restoreLogs() {
        if (enableSuppressLogs) {
            console.error = oldConsoleError;
        }
    }

    async function printLendingStats() {
        await vaultClient.reload();

        let vaultState = await vaultClient.getVaultState();

        let vaultApy = (await vaultClient.getApy()).toNumber();
        let solend = vaultClient.getSolend();
        let port = vaultClient.getPort();

        let solendApy = (await solend.getApy()).toNumber();
        let solendBorrow = (
            await solend.getBorrowedAmount()
        ).lamports.toNumber();
        let solendDeposit = (
            await solend.getDepositedAmount()
        ).lamports.toNumber();
        let solendLpToken = (
            await solend.getLpTokenAccountValue(vaultState)
        ).lamports.toNumber();

        let portApy = (await port.getApy()).toNumber();
        let portBorrow = (await port.getBorrowedAmount()).lamports.toNumber();
        let portDeposit = (await port.getDepositedAmount()).lamports.toNumber();
        let portLpToken = (
            await port.getLpTokenAccountValue(vaultState)
        ).lamports.toNumber();

        console.log("Vault APY: ", vaultApy);
        console.log("");
        console.log("Solend APY: " + solendApy);
        console.log("Solend Borrow: " + solendBorrow);
        console.log("Solend Deposit: " + solendDeposit);
        console.log("Solend LP Tokens: " + solendLpToken);
        console.log("");
        console.log("Port APY: " + portApy);
        console.log("Port Borrow: " + portBorrow);
        console.log("Port Deposit: " + portDeposit);
        console.log("Port LP Tokens: " + portLpToken);
    }

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

    async function initLendingMarkets(createPortSubReward: boolean = false) {
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
            initialReserveAmount,
            createPortSubReward
        );

        const borrowAmount = initialReserveAmount / 2;

        let portBorrowTxs = await port.borrow(
            owner,
            ownerReserveTokenAccount,
            borrowAmount
        );

        for (let tx of portBorrowTxs) {
            await provider.connection.confirmTransaction(tx, "finalized");
        }

        const tokenMintA = new Token(
            program.provider.connection,
            port.accounts.stakingRewardTokenMint,
            TOKEN_PROGRAM_ID,
            owner
        );
        const tokenMintB = reserveToken;
        orca = await OrcaLegacySwap.initialize(
            provider,
            wallet.payer,
            tokenMintA,
            tokenMintB,
            owner,
            owner
        );
    }

    async function initializeVault(
        config: VaultConfig,
        solendAvailable: boolean = true,
        portAvailable: boolean = true
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
        ]);

        await vaultClient.initializeDexStates(
            provider.wallet as anchor.Wallet,
            owner
        );

        if (portAvailable) {
            await vaultClient.initializePortAdditionalState(
                provider.wallet as anchor.Wallet,
                owner
            );

            await vaultClient.initializePortRewardAccounts(
                provider.wallet as anchor.Wallet,
                owner,
                provider,
                DeploymentEnvs.devnetStaging,
                program
            );

            await vaultClient.loadPortAdditionalAccounts();

            await vaultClient.initializeOrcaLegacy(
                provider.wallet as anchor.Wallet,
                owner,
                DeploymentEnvs.devnetStaging,
                orca
            );

            await vaultClient.initializeOrcaLegacyMarket(
                provider.wallet as anchor.Wallet,
                owner
            );
        }

        await vaultClient.reload();

        userReserveTokenAccount = await reserveToken.createAccount(
            wallet.publicKey
        );
    }

    async function getSplTokenAccountBalance(
        mint: PublicKey,
        account: PublicKey
    ): Promise<number> {
        const tokenMint = new Token(
            program.provider.connection,
            mint,
            TOKEN_PROGRAM_ID,
            Keypair.generate()
        );
        return (await tokenMint.getAccountInfo(account)).amount.toNumber();
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
            const preRefresh = (await vaultClient.getPreRefreshTxs()).map(
                (tx) => {
                    return { tx: tx, signers: [] };
                }
            );
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
            assert.equal(0b11, vaultClient.getYieldSourceFlags());
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
            suppressLogs();

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

            restoreLogs();
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
            suppressLogs();

            const errorCode = "0x7d1";
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

            restoreLogs();
        });
    }

    function testVaultFlags() {
        it("Reject deposit transaction if vault is halted", async function () {
            suppressLogs();

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

            restoreLogs();
        });

        it("Reject rebalance transaction if vault is halted", async function () {
            suppressLogs();

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

            restoreLogs();
        });

        it("Update halt flags", async function () {
            const flags = 0;
            const tx = await vaultClient.updateHaltFlags(owner, flags);
            await provider.connection.confirmTransaction(tx, "singleGossip");
            await vaultClient.reload();
            assert.equal(flags, vaultClient.getHaltFlags());
        });

        it("Update yield source flags", async function () {
            const flags = 1;
            const tx = await vaultClient.updateYieldSourceFlags(owner, flags);
            await provider.connection.confirmTransaction(tx, "singleGossip");
            await vaultClient.reload();
            assert.equal(flags, vaultClient.getYieldSourceFlags());
        });

        it("Reject invalid flags update", async function () {
            suppressLogs();

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

            restoreLogs();
        });

        it("Reject unauthorized flags update", async function () {
            suppressLogs();

            const errorCode = "0x7d1";

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

            restoreLogs();
        });
    }

    function testRebalanceWithdraw(
        expectedSolendRatio: number,
        expectedPortRatio: number,
        rebalanceMode: RebalanceMode = RebalanceModes.calculator,
        solendAvailable: boolean = true,
        portAvailable: boolean = true
    ) {
        // NOTE: should not be divisible by the number of markets, 2 in this case
        // The TODO to correct this is below
        const depositQty = 1000001;

        before(async () => {
            await mintReserveToken(userReserveTokenAccount, depositQty);
            await depositToVault(depositQty);
        });

        if (rebalanceMode == RebalanceModes.proofChecker) {
            it("Reject transaction if weights don't equal 100%", async () => {
                suppressLogs();

                const errorCode = program.idl.errors
                    .find((e) => e.name == "InvalidProposedWeights")
                    .code.toString(16);
                try {
                    await performRebalance(
                        {
                            solend: 0,
                            port: 0,
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

                restoreLogs();
            });

            it("Reject transaction if weights are suboptimal", async () => {
                suppressLogs();

                const errorCode = program.idl.errors
                    .find((e) => e.name == "RebalanceProofCheckFailed")
                    .code.toString(16);
                try {
                    // should be suboptimal since solend apy is 0
                    await performRebalance(
                        {
                            solend: 5000,
                            port: 5000,
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

                restoreLogs();
            });

            it("Reject transaction if weights exceed the cap", async () => {
                suppressLogs();

                const errorCode = program.idl.errors
                    .find((e) => e.name == "InvalidProposedWeights")
                    .code.toString(16);
                try {
                    await performRebalance(
                        {
                            solend: 10000,
                            port: 0,
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

                restoreLogs();
            });
        }

        it("Rebalances", async () => {
            await performRebalance({
                solend: expectedSolendRatio * 10000,
                port: expectedPortRatio * 10000,
            });

            // TODO Resolve the difference
            // Use isAtMost because the on-chain rust program handles rounding differently than TypeScript.
            // However the difference should not exceed 1 token.
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
            assert.isAtMost(
                Math.abs(newUserReserveBalance - actualWithdrawAmount),
                maxDiffAllowed
            );
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

            // TODO why are management fees specifically off by 2 tokens?
            const maxDiffAllowed = 2;

            assert.isAtMost(
                Math.abs(actualMgmtFees - expectedFees.primary.toNumber()),
                maxDiffAllowed
            );
            assert.isAtMost(
                Math.abs(actualReferralFees - expectedFees.referral.toNumber()),
                maxDiffAllowed
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
            suppressLogs();

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

            restoreLogs();
        });
    }

    function testPortRewardClaiming(subReward: boolean) {
        const depositQty = 3025004087;

        before(async () => {
            await mintReserveToken(userReserveTokenAccount, depositQty);
            await depositToVault(depositQty);
        });

        it("Stake port LP token when rebalancing", async () => {
            await performRebalance({
                solend: 0,
                port: 10000,
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

            // Check that all allocations to port are staked for earning rewards
            const stakingAccountRaw = await provider.connection.getAccountInfo(
                new PublicKey(port.accounts.vaultPortStakeAccount)
            );
            const stakingAccount = StakeAccount.fromRaw({
                pubkey: port.accounts.vaultPortStakeAccount,
                account: stakingAccountRaw,
            });
            const reserve = await port.client.getReserve(port.accounts.reserve);
            const exchangeRate = reserve.getExchangeRatio();
            const stakedTokenValue = AssetPrice.of(
                MintId.of(port.accounts.stakingRewardTokenMint),
                stakingAccount.getDepositAmount().toU64().toNumber()
            ).divide(exchangeRate.getUnchecked());
            assert.isAtMost(
                Math.abs(stakedTokenValue.getRaw().toNumber() - depositQty),
                maxDiffAllowed
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

            // Make sure that the correct amount is un-staked when the user withdraws
            let maxDiffAllowed = 3;
            const stakingAccountRaw = await provider.connection.getAccountInfo(
                new PublicKey(port.accounts.vaultPortStakeAccount)
            );
            const stakingAccount = StakeAccount.fromRaw({
                pubkey: port.accounts.vaultPortStakeAccount,
                account: stakingAccountRaw,
            });
            const reserve = await port.client.getReserve(port.accounts.reserve);
            const exchangeRate = reserve.getExchangeRatio();
            const stakedTokenValue = AssetPrice.of(
                MintId.of(port.accounts.stakingRewardTokenMint),
                stakingAccount.getDepositAmount().toU64().toNumber()
            ).divide(exchangeRate.getUnchecked());

            assert.isAtMost(
                Math.abs(
                    depositQty -
                        withdrawQty -
                        stakedTokenValue.getRaw().toNumber()
                ),
                maxDiffAllowed
            );

            // Allow max different of 1 token because of rounding error.
            const actualWithdrawAmount = oldVaultValue - newVaultValue;
            assert.isAtMost(
                Math.abs(actualWithdrawAmount - withdrawQty),
                maxDiffAllowed
            );
            // Actual should <= requested because we rounds down.
            assert.isAtMost(actualWithdrawAmount, withdrawQty);
            assert.isAtMost(oldLpTokenSupply - newLpTokenSupply, withdrawQty);
            assert.isAtMost(
                Math.abs(newUserReserveBalance - actualWithdrawAmount),
                maxDiffAllowed
            );
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

            const claimedRewardAmount = await getSplTokenAccountBalance(
                port.accounts.stakingRewardTokenMint,
                port.accounts.vaultPortRewardToken
            );
            assert.isAtLeast(claimedRewardAmount, accumulatedRewardAmount);

            if (subReward) {
                const claimedSubRewardAmount = await getSplTokenAccountBalance(
                    port.accounts.stakingSubRewardTokenMint,
                    port.accounts.vaultPortSubRewardToken
                );
                assert.isAtLeast(claimedSubRewardAmount, 1);
            }
        });

        it("Sell reward", async function () {
            const claimedRewardAmount = await getSplTokenAccountBalance(
                port.accounts.stakingRewardTokenMint,
                port.accounts.vaultPortRewardToken
            );
            const oldReserveBalance = await getVaultReserveTokenBalance();

            await vaultClient.sellPortReward();

            const remainingAmount = await getSplTokenAccountBalance(
                port.accounts.stakingRewardTokenMint,
                port.accounts.vaultPortRewardToken
            );
            const newReserveBalance = await getVaultReserveTokenBalance();

            assert.isAtMost(remainingAmount, 1);
            assert.isAtLeast(
                newReserveBalance - oldReserveBalance,
                claimedRewardAmount
            );
        });
    }

    describe("Equal allocation strategy", () => {
        describe("Deposit and withdrawal", () => {
            before(initLendingMarkets);
            before(async function () {
                await initializeVault({
                    rebalanceMode: { [RebalanceModes.calculator]: {} },
                    strategyType: { [StrategyTypes.equalAllocation]: {} },
                });
            });
            testDepositAndWithdrawal();
        });

        describe("Deposit cap and vault flags", () => {
            before(initLendingMarkets);
            before(async function () {
                await initializeVault({
                    depositCap: new anchor.BN(vaultDepositCap),
                    rebalanceMode: { [RebalanceModes.calculator]: {} },
                    strategyType: { [StrategyTypes.equalAllocation]: {} },
                });
            });
            testDepositCap();
            testVaultFlags();
        });

        describe("Rebalance", () => {
            before(initLendingMarkets);
            before(async function () {
                await initializeVault({
                    rebalanceMode: { [RebalanceModes.calculator]: {} },
                    strategyType: { [StrategyTypes.equalAllocation]: {} },
                });
            });
            testRebalanceWithdraw(1 / 2, 1 / 2);
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
                    rebalanceMode: { [RebalanceModes.calculator]: {} },
                    strategyType: { [StrategyTypes.equalAllocation]: {} },
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

            // Port should get max alloc since only it has > 0 APY
            testRebalanceWithdraw(
                1 - vaultAllocationCap / 100,
                vaultAllocationCap / 100
            );
        });
    });

    describe("Max yield proof checker", () => {
        describe("Rebalance", () => {
            const rebalanceMode = RebalanceModes.proofChecker;
            before(initLendingMarkets);
            before(async function () {
                await initializeVault({
                    allocationCapPct: vaultAllocationCap,
                    rebalanceMode: { [RebalanceModes.proofChecker]: {} },
                    strategyType: { [StrategyTypes.maxYield]: {} },
                });
            });
            testRebalanceWithdraw(
                1 - vaultAllocationCap / 100,
                vaultAllocationCap / 100,
                rebalanceMode
            );
        });
    });

    describe("Refresh Check", () => {
        before(initLendingMarkets);
        before(async function () {
            await initializeVault({
                allocationCapPct: vaultAllocationCap,
                strategyType: { [StrategyTypes.equalAllocation]: {} },
                rebalanceMode: { [RebalanceModes.calculator]: {} },
            });
        });

        it("Reject incomplete refresh after rebalance", async function () {
            suppressLogs();

            const depositQty = 3000000;
            await mintReserveToken(userReserveTokenAccount, depositQty);
            await depositToVault(depositQty);

            await performRebalance({
                solend: (1 / 2) * 10000,
                port: (1 / 2) * 10000,
            });

            const maxDiffAllowed = 1;

            // Deliberately craft an incomplete refresh
            const refreshTx = new Transaction();
            refreshTx.add(
                await vaultClient
                    .getSolend()
                    .getRefreshIx(
                        program,
                        vaultClient.vaultId,
                        vaultClient.getVaultState()
                    )
            );
            refreshTx.add(await vaultClient.getConsolidateRefreshIx());

            // TODO the program spits out base 10 (e.g. 6003) for this particular error
            const expectedErrorCode = program.idl.errors
                .find((e) => e.name == "AllocationIsNotUpdated")
                .code.toString();

            try {
                const sig = await program.provider.sendAndConfirm(refreshTx);
                await program.provider.connection.confirmTransaction(
                    sig,
                    "singleGossip"
                );
            } catch (err) {
                assert.isTrue(
                    err.message.includes(expectedErrorCode),
                    `Error code ${expectedErrorCode} not included in error message: ${err}`
                );
            }
            restoreLogs();

            await vaultClient.reload();
            const vaultValueStoredOnChain = vaultClient
                .getVaultState()
                .value.value.toNumber();
            assert.isAtMost(
                Math.abs(vaultValueStoredOnChain - Math.floor(depositQty)),
                maxDiffAllowed
            );

            restoreLogs();
        });
    });

    describe("Disabled pools", () => {
        describe("Rebalance with equal allocation strategy missing 1 pool", () => {
            const rebalanceMode = RebalanceModes.calculator;
            before(initLendingMarkets);
            before(async function () {
                await initializeVault(
                    {
                        allocationCapPct: 100,
                        rebalanceMode: { [RebalanceModes.calculator]: {} },
                        strategyType: { [StrategyTypes.equalAllocation]: {} },
                    },
                    true,
                    false
                );
            });

            it("Initialize fewer yield sources", async function () {
                assert.equal(0b01, vaultClient.getYieldSourceFlags());
            });

            testRebalanceWithdraw(1, 0, rebalanceMode, true, false);
        });
    });

    describe("Stake Port LP token and claim reward", () => {
        describe("Sub-reward enabled", () => {
            before(async function () {
                await initLendingMarkets(true);
            });
            before(async function () {
                await initializeVault(
                    {
                        allocationCapPct: 100,
                        strategyType: { [StrategyTypes.equalAllocation]: {} },
                        rebalanceMode: { [RebalanceModes.calculator]: {} },
                    },
                    false,
                    true
                );
            });

            testPortRewardClaiming(true);
        });

        describe("Sub-reward disabled", () => {
            before(async function () {
                await initLendingMarkets(false);
            });
            before(async function () {
                await initializeVault(
                    {
                        allocationCapPct: 100,
                        strategyType: { [StrategyTypes.equalAllocation]: {} },
                        rebalanceMode: { [RebalanceModes.calculator]: {} },
                    },
                    false,
                    true
                );
            });

            testPortRewardClaiming(false);
        });
    });

    describe("Orca swap", () => {
        it("Find market", () => {
            const portToken = new PublicKey(
                "PoRTjZMPXb9T7dyU7tpLEZRQj7e6ssfAE62j2oQuc6y"
            );
            const usdcToken = new PublicKey(
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
            );
            const orca = OrcaLegacySwap.load(
                portToken,
                usdcToken,
                "mainnet-beta"
            );

            assert.equal(orca.accounts.marketId, 0);
            assert.equal(orca.accounts.tokenAccountA, portToken);
            assert.equal(orca.accounts.tokenAccountB, usdcToken);
            assert.equal(
                orca.accounts.swapProgram.toString(),
                // ORCA mainnet market for PORT/USDC
                "4if9Gy7dvjU7XwunKxdnCcPsaT3yAHPXdz2XS1eo19LG"
            );
        });
    });
});
