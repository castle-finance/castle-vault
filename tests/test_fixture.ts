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
}
