import Big from "big.js";
import {
    Connection,
    Keypair,
    PublicKey,
    SystemProgram,
    SYSVAR_CLOCK_PUBKEY,
    SYSVAR_RENT_PUBKEY,
    Transaction,
    TransactionInstruction,
    TransactionSignature,
} from "@solana/web3.js";
import {
    AccountInfo,
    AccountLayout,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    MintInfo,
    NATIVE_MINT,
    Token as SplToken,
    TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import * as anchor from "@project-serum/anchor";
import { SendTxRequest } from "@project-serum/anchor/dist/cjs/provider";

import {
    DeploymentEnvs,
    StrategyType,
    RebalanceMode,
    RebalanceModes,
    DeploymentEnv,
} from "@castlefinance/vault-core";

import { CLUSTER_MAP, PROGRAM_IDS } from ".";
import { CastleVault } from "./idl";
import {
    LendingMarket,
    PortReserveAsset,
    SolendReserveAsset,
    JetReserveAsset,
} from "./adapters";
import {
    ProposedWeightsBps,
    RebalanceDataEvent,
    Vault,
    VaultConfig,
    VaultFlags,
} from "./types";
import { ExchangeRate, Rate, Token, TokenAmount } from "./utils";

//type YieldSources = {[key: string]: LendingMarket}
interface YieldSources {
    solend?: SolendReserveAsset;
    port?: PortReserveAsset;
    jet?: JetReserveAsset;
}

interface YieldSources {
    solend?: SolendReserveAsset;
    port?: PortReserveAsset;
    jet?: JetReserveAsset;
}

export class VaultClient {
    private constructor(
        public program: anchor.Program<CastleVault>,
        public vaultId: PublicKey,
        private vaultState: Vault,
        private yieldSources: YieldSources,
        private reserveToken: Token,
        private lpToken: Token,
        private feesEnabled: boolean = false
    ) {}

    static async load(
        provider: anchor.Provider,
        vaultId: PublicKey,
        env: DeploymentEnv = DeploymentEnvs.mainnet
    ): Promise<VaultClient> {
        const program = (await anchor.Program.at(
            PROGRAM_IDS[env],
            provider
        )) as anchor.Program<CastleVault>;
        const vaultState = await program.account.vault.fetch(vaultId);

        const cluster = CLUSTER_MAP[env];
        const reserveMint = vaultState.reserveTokenMint;

        const solend = await SolendReserveAsset.load(
            provider,
            cluster,
            reserveMint
        );
        const port = await PortReserveAsset.load(
            provider,
            cluster,
            reserveMint
        );
        const jet = await JetReserveAsset.load(provider, cluster, reserveMint);

        const [reserveToken, lpToken] = await this.getReserveAndLpTokens(
            provider.connection,
            vaultState
        );

        return new VaultClient(
            program,
            vaultId,
            vaultState,
            {
                solend: solend,
                port: port,
                jet: jet,
            },
            reserveToken,
            lpToken
        );
    }

    async reload() {
        this.vaultState = await this.program.account.vault.fetch(this.vaultId);
        // TODO reload underlying asset data also?
    }

    static async initialize(
        provider: anchor.Provider,
        wallet: anchor.Wallet,
        env: DeploymentEnv,
        reserveTokenMint: PublicKey,
        owner: PublicKey,
        referralFeeOwner: PublicKey,
        config: VaultConfig,
        program?: anchor.Program<CastleVault>
    ): Promise<VaultClient> {
        // TODO Once the below issue is resolved, remove this logic
        // https://github.com/project-serum/anchor/issues/1844
        // Program should only be passed in during testing
        if (program == null) {
            program = (await anchor.Program.at(
                PROGRAM_IDS[env],
                provider
            )) as anchor.Program<CastleVault>;
        }

        const vaultId = Keypair.generate();

        const [vaultAuthority, authorityBump] =
            await PublicKey.findProgramAddress(
                [
                    vaultId.publicKey.toBuffer(),
                    anchor.utils.bytes.utf8.encode("authority"),
                ],
                program.programId
            );

        const [vaultReserveTokenAccount, reserveBump] =
            await PublicKey.findProgramAddress(
                [vaultId.publicKey.toBuffer(), reserveTokenMint.toBuffer()],
                program.programId
            );

        const [lpTokenMint, lpTokenMintBump] =
            await PublicKey.findProgramAddress(
                [
                    vaultId.publicKey.toBuffer(),
                    anchor.utils.bytes.utf8.encode("lp_mint"),
                ],
                program.programId
            );

        const feeReceiver = await SplToken.getAssociatedTokenAddress(
            ASSOCIATED_TOKEN_PROGRAM_ID,
            TOKEN_PROGRAM_ID,
            lpTokenMint,
            owner
        );

        const referralFeeReceiver = await SplToken.getAssociatedTokenAddress(
            ASSOCIATED_TOKEN_PROGRAM_ID,
            TOKEN_PROGRAM_ID,
            lpTokenMint,
            referralFeeOwner
        );

        const defaultConfig: VaultConfig = {
            depositCap: new anchor.BN("18446744073709551615"), // U64::MAX
            feeCarryBps: 0,
            feeMgmtBps: 0,
            referralFeePct: 0,
            allocationCapPct: 100,
            rebalanceMode: { calculator: {} },
            strategyType: { maxYield: {} },
        };

        const txSig = await program.rpc.initialize(
            // Anchor has a bug that decodes nested types incorrectly
            // https://github.com/project-serum/anchor/pull/1726
            //@ts-ignore
            {
                authority: authorityBump,
                reserve: reserveBump,
                lpMint: lpTokenMintBump,
            },
            { ...defaultConfig, ...config },
            {
                accounts: {
                    vault: vaultId.publicKey,
                    vaultAuthority: vaultAuthority,
                    lpTokenMint: lpTokenMint,
                    vaultReserveToken: vaultReserveTokenAccount,
                    reserveTokenMint: reserveTokenMint,
                    feeReceiver: feeReceiver,
                    referralFeeReceiver: referralFeeReceiver,
                    referralFeeOwner: referralFeeOwner,
                    payer: wallet.payer.publicKey,
                    owner: owner,
                    systemProgram: SystemProgram.programId,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
                    rent: SYSVAR_RENT_PUBKEY,
                },
                signers: [vaultId, wallet.payer],
                instructions: [
                    await program.account.vault.createInstruction(vaultId),
                ],
            }
        );
        await program.provider.connection.confirmTransaction(
            txSig,
            "finalized"
        );

        const vaultState = await program.account.vault.fetch(vaultId.publicKey);

        const [reserveToken, lpToken] = await this.getReserveAndLpTokens(
            provider.connection,
            vaultState
        );

        return new VaultClient(
            program,
            vaultId.publicKey,
            vaultState,
            {},
            reserveToken,
            lpToken
        );
    }

    async initializeSolend(
        wallet: anchor.Wallet,
        solend: SolendReserveAsset,
        owner: Keypair
    ) {
        const tx = new Transaction();
        tx.add(
            await solend.getInitializeIx(
                this.program,
                this.vaultId,
                this.vaultState.vaultAuthority,
                wallet.payer.publicKey,
                owner.publicKey
            )
        );

        const txSig = await this.program.provider.send(tx, [
            owner,
            wallet.payer,
        ]);
        await this.program.provider.connection.confirmTransaction(
            txSig,
            "finalized"
        );
        this.yieldSources.solend = solend;
    }

    async initializePort(
        wallet: anchor.Wallet,
        port: PortReserveAsset,
        owner: Keypair
    ) {
        const tx = new Transaction();
        tx.add(
            await port.getInitializeIx(
                this.program,
                this.vaultId,
                this.vaultState.vaultAuthority,
                wallet.payer.publicKey,
                owner.publicKey
            )
        );

        const txSig = await this.program.provider.send(tx, [
            owner,
            wallet.payer,
        ]);
        await this.program.provider.connection.confirmTransaction(
            txSig,
            "finalized"
        );
        this.yieldSources.port = port;
    }

    async initializeJet(
        wallet: anchor.Wallet,
        jet: JetReserveAsset,
        owner: Keypair
    ) {
        const tx = new Transaction();
        tx.add(
            await jet.getInitializeIx(
                this.program,
                this.vaultId,
                this.vaultState.vaultAuthority,
                wallet.payer.publicKey,
                owner.publicKey
            )
        );

        const txSig = await this.program.provider.send(tx, [
            owner,
            wallet.payer,
        ]);
        await this.program.provider.connection.confirmTransaction(
            txSig,
            "finalized"
        );
        this.yieldSources.jet = jet;
    }

    getRefreshIxs(): TransactionInstruction[] {
        return Object.keys(this.yieldSources)
            .map((k) => {
                return this.yieldSources[k].getRefreshIx(
                    this.program,
                    this.vaultId,
                    this.vaultState
                );
            })
            .concat([this.getConsolidateRefreshIx()]);
    }

    private static async getReserveAndLpTokens(
        connection: Connection,
        vaultState: Vault
    ): Promise<Token[]> {
        const lpSplToken = new SplToken(
            connection,
            vaultState.lpTokenMint,
            TOKEN_PROGRAM_ID,
            Keypair.generate() // dummy since we don't need to send txs
        );
        const lpToken = new Token(
            vaultState.lpTokenMint,
            await lpSplToken.getMintInfo()
        );

        const reserveSplToken = new SplToken(
            connection,
            vaultState.reserveTokenMint,
            TOKEN_PROGRAM_ID,
            Keypair.generate() // dummy since we don't need to send txs
        );
        const reserveToken = new Token(
            vaultState.reserveTokenMint,
            await reserveSplToken.getMintInfo()
        );

        return [reserveToken, lpToken];
    }

    getConsolidateRefreshIx(): TransactionInstruction {
        const feeAccounts = this.feesEnabled
            ? [
                  {
                      isSigner: false,
                      isWritable: true,
                      pubkey: this.vaultState.feeReceiver,
                  },
                  {
                      isSigner: false,
                      isWritable: true,
                      pubkey: this.vaultState.referralFeeReceiver,
                  },
              ]
            : [];

        return this.program.instruction.consolidateRefresh({
            accounts: {
                vault: this.vaultId,
                vaultAuthority: this.vaultState.vaultAuthority,
                vaultReserveToken: this.vaultState.vaultReserveToken,
                lpTokenMint: this.vaultState.lpTokenMint,
                tokenProgram: TOKEN_PROGRAM_ID,
            },
            remainingAccounts: feeAccounts,
        });
    }

    /**
     *
     * @param wallet
     * @param lamports amount depositing
     * @returns
     */
    private async getWrappedSolIxs(
        wallet: anchor.Wallet,
        lamports = 0
    ): Promise<WrapSolIxResponse> {
        const userReserveKeypair = Keypair.generate();
        const userReserveTokenAccount = userReserveKeypair.publicKey;

        const rent = await SplToken.getMinBalanceRentForExemptAccount(
            this.program.provider.connection
        );
        return {
            openIxs: [
                SystemProgram.createAccount({
                    fromPubkey: wallet.publicKey,
                    newAccountPubkey: userReserveTokenAccount,
                    programId: TOKEN_PROGRAM_ID,
                    space: AccountLayout.span,
                    lamports: lamports + rent,
                }),
                SplToken.createInitAccountInstruction(
                    TOKEN_PROGRAM_ID,
                    NATIVE_MINT,
                    userReserveTokenAccount,
                    wallet.publicKey
                ),
            ],
            closeIx: SplToken.createCloseAccountInstruction(
                TOKEN_PROGRAM_ID,
                userReserveTokenAccount,
                wallet.publicKey,
                wallet.publicKey,
                []
            ),
            keyPair: userReserveKeypair,
        };
    }

    /**
     * @param new_value
     * @returns
     */
    async updateFlags(
        owner: Keypair,
        flags: number
    ): Promise<TransactionSignature> {
        const updateTx = new Transaction();
        updateTx.add(
            this.program.instruction.updateFlags(flags, {
                accounts: {
                    vault: this.vaultId,
                    owner: owner.publicKey,
                },
            })
        );
        return await this.program.provider.send(updateTx, [owner]);
    }

    /**
     * @param new_value
     * @returns
     */
    async updateConfig(
        owner: Keypair,
        config: VaultConfig
    ): Promise<TransactionSignature> {
        const updateTx = new Transaction();
        updateTx.add(
            // Anchor has a bug that decodes nested types incorrectly
            // https://github.com/project-serum/anchor/pull/1726
            //@ts-ignore
            this.program.instruction.updateConfig(config, {
                accounts: {
                    vault: this.vaultId,
                    owner: owner.publicKey,
                },
            })
        );
        return await this.program.provider.send(updateTx, [owner]);
    }

    getDepositIx(
        amount: anchor.BN,
        userAuthority: PublicKey,
        userLpTokenAccount: PublicKey,
        userReserveTokenAccount: PublicKey
    ) {
        return this.program.instruction.deposit(amount, {
            accounts: {
                vault: this.vaultId,
                vaultAuthority: this.vaultState.vaultAuthority,
                vaultReserveToken: this.vaultState.vaultReserveToken,
                lpTokenMint: this.vaultState.lpTokenMint,
                userReserveToken: userReserveTokenAccount,
                userLpToken: userLpTokenAccount,
                userAuthority: userAuthority,
                tokenProgram: TOKEN_PROGRAM_ID,
                clock: SYSVAR_CLOCK_PUBKEY,
            },
        });
    }

    /**
     *
     *
     * @param wallet
     * @param amount
     * @param userReserveTokenAccount
     * @returns
     */
    async deposit(
        wallet: anchor.Wallet,
        amount: number,
        userReserveTokenAccount: PublicKey
    ): Promise<TransactionSignature[]> {
        const depositTx = new Transaction();

        let wrappedSolIxResponse: WrapSolIxResponse;
        if (this.vaultState.reserveTokenMint.equals(NATIVE_MINT)) {
            wrappedSolIxResponse = await this.getWrappedSolIxs(wallet, amount);
            depositTx.add(...wrappedSolIxResponse.openIxs);
            userReserveTokenAccount = wrappedSolIxResponse.keyPair.publicKey;
        }

        const userLpTokenAccount = await this.getUserLpTokenAccount(
            wallet.publicKey
        );
        const userLpTokenAccountInfo =
            await this.program.provider.connection.getAccountInfo(
                userLpTokenAccount
            );

        // Create account if it does not exist
        let createLpAcctTx: Transaction;
        if (userLpTokenAccountInfo == null) {
            createLpAcctTx = new Transaction().add(
                createAta(
                    wallet.publicKey,
                    this.vaultState.lpTokenMint,
                    userLpTokenAccount
                )
            );
        }

        this.getRefreshIxs().forEach((element) => {
            depositTx.add(element);
        });
        depositTx.add(
            this.getDepositIx(
                new anchor.BN(amount),
                wallet.publicKey,
                userLpTokenAccount,
                userReserveTokenAccount
            )
        );

        const txs: SendTxRequest[] = [];
        if (createLpAcctTx != null) {
            txs.push({ tx: createLpAcctTx, signers: [] });
        }

        if (wrappedSolIxResponse != null) {
            depositTx.add(wrappedSolIxResponse.closeIx);
            txs.push({
                tx: depositTx,
                signers: [wrappedSolIxResponse.keyPair],
            });
        } else {
            txs.push({ tx: depositTx, signers: [] });
        }

        return await this.program.provider.sendAll(txs);
    }

    getWithdrawIx(
        amount: anchor.BN,
        userAuthority: PublicKey,
        userLpTokenAccount: PublicKey,
        userReserveTokenAccount: PublicKey
    ) {
        return this.program.instruction.withdraw(amount, {
            accounts: {
                vault: this.vaultId,
                vaultAuthority: this.vaultState.vaultAuthority,
                userAuthority: userAuthority,
                userLpToken: userLpTokenAccount,
                userReserveToken: userReserveTokenAccount,
                vaultReserveToken: this.vaultState.vaultReserveToken,
                lpTokenMint: this.vaultState.lpTokenMint,
                tokenProgram: TOKEN_PROGRAM_ID,
                clock: SYSVAR_CLOCK_PUBKEY,
            },
        });
    }

    /**
     *
     * @param wallet
     * @param amount denominated in lp tokens
     * @returns
     */
    async withdraw(
        wallet: anchor.Wallet,
        amount: number
    ): Promise<TransactionSignature[]> {
        const userLpTokenAccount = await this.getUserLpTokenAccount(
            wallet.publicKey
        );

        const txs: SendTxRequest[] = [];

        // Add reconcile txs
        const reconcileTxs = await this.getReconcileTxs(amount);
        reconcileTxs.forEach((tx) => txs.push({ tx, signers: [] }));

        const withdrawTx = new Transaction();
        let userReserveTokenAccount: PublicKey;
        let wrappedSolIxResponse: WrapSolIxResponse;
        if (this.vaultState.reserveTokenMint.equals(NATIVE_MINT)) {
            wrappedSolIxResponse = await this.getWrappedSolIxs(wallet);
            withdrawTx.add(...wrappedSolIxResponse.openIxs);
            userReserveTokenAccount = wrappedSolIxResponse.keyPair.publicKey;
        } else {
            userReserveTokenAccount = await this.getUserReserveTokenAccount(
                wallet.publicKey
            );
            // Create reserve token account to withdraw into if it does not exist
            const userReserveTokenAccountInfo =
                await this.program.provider.connection.getAccountInfo(
                    userReserveTokenAccount
                );
            if (userReserveTokenAccountInfo == null) {
                withdrawTx.add(
                    createAta(
                        wallet.publicKey,
                        this.vaultState.reserveTokenMint,
                        userReserveTokenAccount
                    )
                );
            }
        }

        this.getRefreshIxs().forEach((element) => {
            withdrawTx.add(element);
        });
        withdrawTx.add(
            this.getWithdrawIx(
                new anchor.BN(amount),
                wallet.publicKey,
                userLpTokenAccount,
                userReserveTokenAccount
            )
        );

        if (wrappedSolIxResponse != null) {
            withdrawTx.add(wrappedSolIxResponse.closeIx);
            txs.push({
                tx: withdrawTx,
                signers: [wrappedSolIxResponse.keyPair],
            });
        } else {
            txs.push({ tx: withdrawTx, signers: [] });
        }
        return this.program.provider.sendAll(txs);
    }

    async getReconcileTxs(amount: number) {
        const txs: Transaction[] = [];

        // Withdraw from lending markets if not enough reserves in vault
        const vaultReserveTokenAccountInfo =
            await this.getReserveTokenAccountInfo(
                this.vaultState.vaultReserveToken
            );
        const vaultReserveAmount = new Big(
            vaultReserveTokenAccountInfo.amount.toString()
        ).round(0, Big.roundDown);

        // Convert from lp tokens to reserve tokens
        // NOTE: this rate is slightly lower than what it will be in the transaction
        //  by about 1/10000th of the current yield (1bp per 100%).
        //  To avoid a insufficient funds error, we slightly over-correct for this
        //  This does not work when withdrawing the last tokens from the vault
        const exchangeRate = await this.getLpExchangeRate();
        const adjustFactor = (await this.getApy()).toBig().div(10000);
        const convertedAmount = exchangeRate
            .toBig()
            .mul(amount)
            .mul(new Big(1).add(adjustFactor))
            .round(0, Big.roundUp);

        if (vaultReserveAmount.lt(convertedAmount)) {
            // Sort reconcile ixs by most to least $ to minimize number of TXs sent
            const reconcileIxs = (
                await Promise.all(
                    Object.keys(this.yieldSources).map(
                        async (k): Promise<[Big, string]> => {
                            const alloc: Big = (
                                await this.yieldSources[
                                    k
                                ].getLpTokenAccountValue(this.vaultState)
                            ).lamports;
                            return [alloc, k];
                        }
                    )
                )
            ).sort((a, b) => b[0].sub(a[0]).toNumber());

            const toReconcileAmount = convertedAmount.sub(vaultReserveAmount);
            let reconciledAmount = Big(0);
            let n = 0;
            while (reconciledAmount.lt(toReconcileAmount)) {
                const [alloc, k] = reconcileIxs[n];

                // min of alloc and toWithdrawAmount - withdrawnAmount
                const reconcileAmount = alloc.gt(
                    toReconcileAmount.sub(reconciledAmount)
                )
                    ? toReconcileAmount
                    : alloc;

                if (!Big(0).eq(reconcileAmount)) {
                    const reconcileTx = new Transaction();
                    reconcileTx.add(
                        this.yieldSources[k].getRefreshIx(
                            this.program,
                            this.vaultId,
                            this.vaultState
                        ),
                        this.yieldSources[k].getReconcileIx(
                            this.program,
                            this.vaultId,
                            this.vaultState,
                            new anchor.BN(reconcileAmount.toString())
                        )
                    );
                    txs.push(reconcileTx);
                }
                reconciledAmount = reconciledAmount.add(reconcileAmount);
                n += 1;
            }
        }

        return txs;
    }

    getRebalanceTx(proposedWeights: ProposedWeightsBps): Transaction {
        const rebalanceTx = new Transaction();
        this.getRefreshIxs().forEach((element) => {
            rebalanceTx.add(element);
        });
        rebalanceTx.add(
            this.program.instruction.rebalance(proposedWeights, {
                accounts: {
                    vault: this.vaultId,
                    solendReserve: this.yieldSources.solend.accounts.reserve,
                    portReserve: this.yieldSources.port.accounts.reserve,
                    jetReserve: this.yieldSources.jet.accounts.reserve,
                    clock: SYSVAR_CLOCK_PUBKEY,
                },
            })
        );
        return rebalanceTx;
    }

    /**
     *
     * @param proposedWeights
     * @returns
     */
    async rebalance(
        proposedWeights?: ProposedWeightsBps
    ): Promise<TransactionSignature[]> {
        if (
            this.getRebalanceMode() == RebalanceModes.proofChecker &&
            proposedWeights == null
        ) {
            throw new Error(
                "Proposed weights must be passed in for a vault running in proofChecker mode"
            );
        }

        // Sort ixs in descending order of outflows
        const newAllocations = (
            await this.program.simulate.rebalance(proposedWeights, {
                accounts: {
                    vault: this.vaultId,
                    solendReserve:
                        this.yieldSources.solend != null
                            ? this.yieldSources.solend.accounts.reserve
                            : Keypair.generate().publicKey,
                    portReserve:
                        this.yieldSources.port != null
                            ? this.yieldSources.port.accounts.reserve
                            : Keypair.generate().publicKey,
                    jetReserve:
                        this.yieldSources.jet != null
                            ? this.yieldSources.jet.accounts.reserve
                            : Keypair.generate().publicKey,
                    clock: SYSVAR_CLOCK_PUBKEY,
                },
                instructions: this.getRefreshIxs(),
            })
        ).events[1].data as RebalanceDataEvent;

        const newAndOldallocations = await Promise.all(
            Object.entries(this.yieldSources).map(
                async ([k, v]): Promise<[LendingMarket, Big, Big]> => {
                    const newAlloc = new Big(newAllocations[k].toString());
                    const oldAlloc = (
                        await this.yieldSources[k].getLpTokenAccountValue(
                            this.vaultState
                        )
                    ).lamports;
                    return [v, newAlloc, oldAlloc];
                }
            )
        );

        const allocationDiffsWithReconcileTxs: [Big, Transaction][] =
            newAndOldallocations.map(([yieldSource, newAlloc, oldAlloc]) => [
                newAlloc.sub(oldAlloc),
                new Transaction().add(
                    yieldSource.getRefreshIx(
                        this.program,
                        this.vaultId,
                        this.vaultState
                    ),
                    yieldSource.getReconcileIx(
                        this.program,
                        this.vaultId,
                        this.vaultState
                    )
                ),
            ]);

        const reconcileTxs = allocationDiffsWithReconcileTxs
            .sort((a, b) => a[0].sub(b[0]).toNumber())
            .map(([, tx]) => {
                return { tx: tx, signers: [] };
            });

        const txs: SendTxRequest[] = [
            { tx: this.getRebalanceTx(proposedWeights), signers: [] },
            ...reconcileTxs,
        ];

        return this.program.provider.sendAll(txs);
    }

    /**
     *
     * @returns Weighted average of APYs by allocation
     */
    async getApy(): Promise<Rate> {
        const assetApysAndValues: [Rate, Big][] = [
            [
                Rate.zero(),
                (await this.getVaultReserveTokenAccountValue()).lamports,
            ],
        ].concat(
            await Promise.all(
                Object.entries(this.yieldSources).map(
                    async ([k, v]): Promise<[Rate, Big]> => {
                        return [
                            await this.yieldSources[k].getApy(),
                            (
                                await this.yieldSources[
                                    k
                                ].getLpTokenAccountValue(this.vaultState)
                            ).lamports,
                        ];
                    }
                )
            )
        );

        const [valueSum, weightSum] = assetApysAndValues.reduce(
            ([valueSum, weightSum], [value, weight]) => [
                valueSum.add(value.mul(weight)),
                weightSum.add(weight),
            ],
            [Rate.zero(), new Big(0)]
        );
        if (weightSum.eq(new Big(0))) {
            return Rate.zero();
        } else {
            return valueSum.div(weightSum);
        }
    }

    // Denominated in reserve tokens per LP token
    async getLpExchangeRate(): Promise<ExchangeRate> {
        const totalValue = (await this.getTotalValue()).lamports;
        const lpTokenSupply = new Big(
            (await this.getLpTokenMintInfo()).supply.toString()
        );

        const bigZero = new Big(0);
        if (lpTokenSupply.eq(bigZero) || totalValue.eq(bigZero)) {
            return new ExchangeRate(Big(1), this.reserveToken, this.lpToken);
        } else {
            return new ExchangeRate(
                totalValue.div(lpTokenSupply),
                this.reserveToken,
                this.lpToken
            );
        }
    }

    /**
     * Gets the total value stored in the vault, denominated in reserve tokens
     *
     * @returns
     */
    async getTotalValue(): Promise<TokenAmount> {
        await this.reload();

        const values: [TokenAmount] = (
            await Promise.all(
                Object.entries(this.yieldSources).map(
                    async ([k, v]): Promise<TokenAmount> => {
                        return await this.yieldSources[
                            k
                        ].getLpTokenAccountValue(this.vaultState);
                    }
                )
            )
        ).concat([await this.getVaultReserveTokenAccountValue()]);

        return values.reduce(
            (a, b) => a.add(b),
            TokenAmount.zero(this.reserveToken.mintInfo.decimals)
        );
    }

    async getUserValue(address: PublicKey): Promise<TokenAmount> {
        const userLpTokenAccount = await this.getUserLpTokenAccount(address);
        try {
            const userLpTokenAmount = TokenAmount.fromTokenAccountInfo(
                await this.getLpTokenAccountInfo(userLpTokenAccount),
                this.lpToken.mintInfo.decimals
            );
            const exchangeRate = await this.getLpExchangeRate();
            return exchangeRate.convertToBase(userLpTokenAmount);
        } catch {
            return TokenAmount.fromToken(this.reserveToken, Big(0));
        }
    }

    async getVaultReserveTokenAccountValue(): Promise<TokenAmount> {
        return TokenAmount.fromTokenAccountInfo(
            await this.getReserveTokenAccountInfo(
                this.getVaultReserveTokenAccount()
            ),
            this.lpToken.mintInfo.decimals
        );
    }

    async getVaultSolendLpTokenAccountValue(): Promise<TokenAmount> {
        return this.yieldSources.solend.getLpTokenAccountValue(this.vaultState);
    }

    async getVaultPortLpTokenAccountValue(): Promise<TokenAmount> {
        return this.yieldSources.port.getLpTokenAccountValue(this.vaultState);
    }

    async getVaultJetLpTokenAccountValue(): Promise<TokenAmount> {
        return this.yieldSources.jet.getLpTokenAccountValue(this.vaultState);
    }

    /**
     * Calculates the ATA given the user's address and vault mint
     * @param address Users public key
     * @returns Users reserve ATA given vault reserve mint
     */
    async getUserReserveTokenAccount(address: PublicKey): Promise<PublicKey> {
        return SplToken.getAssociatedTokenAddress(
            ASSOCIATED_TOKEN_PROGRAM_ID,
            TOKEN_PROGRAM_ID,
            this.reserveToken.mint,
            address
        );
    }

    async getReserveTokenAccountInfo(address: PublicKey): Promise<AccountInfo> {
        const reserveToken = new SplToken(
            this.program.provider.connection,
            this.reserveToken.mint,
            TOKEN_PROGRAM_ID,
            Keypair.generate() // dummy since we don't need to send txs
        );
        return reserveToken.getAccountInfo(address);
    }

    async getUserLpTokenAccount(address: PublicKey): Promise<PublicKey> {
        return SplToken.getAssociatedTokenAddress(
            ASSOCIATED_TOKEN_PROGRAM_ID,
            TOKEN_PROGRAM_ID,
            this.lpToken.mint,
            address,
            true
        );
    }

    async getLpTokenAccountInfo(address: PublicKey): Promise<AccountInfo> {
        const lpToken = new SplToken(
            this.program.provider.connection,
            this.lpToken.mint,
            TOKEN_PROGRAM_ID,
            Keypair.generate() // dummy since we don't need to send txs
        );
        return lpToken.getAccountInfo(address);
    }

    async getFeeReceiverAccountInfo(): Promise<AccountInfo> {
        const lpToken = new SplToken(
            this.program.provider.connection,
            this.lpToken.mint,
            TOKEN_PROGRAM_ID,
            Keypair.generate() // dummy since we don't need to send txs
        );
        return lpToken.getAccountInfo(this.vaultState.feeReceiver);
    }

    async getReferralFeeReceiverAccountInfo(): Promise<AccountInfo> {
        const lpToken = new SplToken(
            this.program.provider.connection,
            this.lpToken.mint,
            TOKEN_PROGRAM_ID,
            Keypair.generate() // dummy since we don't need to send txs
        );
        return lpToken.getAccountInfo(this.vaultState.referralFeeReceiver);
    }

    async getLpTokenMintInfo(): Promise<MintInfo> {
        const lpToken = new SplToken(
            this.program.provider.connection,
            this.vaultState.lpTokenMint,
            TOKEN_PROGRAM_ID,
            Keypair.generate() // dummy since we don't need to send txs
        );
        return lpToken.getMintInfo();
    }

    getVaultConfig(): VaultConfig {
        return this.vaultState.config;
    }

    getReserveTokenMint(): PublicKey {
        return this.vaultState.reserveTokenMint;
    }

    getLpTokenMint(): PublicKey {
        return this.vaultState.lpTokenMint;
    }

    getReserveToken(): Token {
        return this.reserveToken;
    }

    getLpToken(): Token {
        return this.lpToken;
    }

    getDepositCap(): TokenAmount {
        return TokenAmount.fromToken(
            this.reserveToken,
            Big(this.vaultState.config.depositCap.toString())
        );
    }

    getAllocationCap(): Rate {
        return Rate.fromPercent(this.vaultState.config.allocationCapPct);
    }

    getVaultReserveTokenAccount(): PublicKey {
        return this.vaultState.vaultReserveToken;
    }

    getStrategyType(): StrategyType {
        return Object.keys(
            this.vaultState.config.strategyType
        )[0] as StrategyType;
    }

    getRebalanceMode(): RebalanceMode {
        return Object.keys(
            this.vaultState.config.rebalanceMode
        )[0] as RebalanceMode;
    }

    getSolend(): SolendReserveAsset {
        return this.yieldSources.solend;
    }

    getPort(): PortReserveAsset {
        return this.yieldSources.port;
    }

    getJet(): JetReserveAsset {
        return this.yieldSources.jet;
    }

    getReferralFeeSplit(): Rate {
        return Rate.fromPercent(this.vaultState.config.referralFeePct);
    }

    getCarryFee(): Rate {
        return Rate.fromBps(this.vaultState.config.feeCarryBps);
    }

    getManagementFee(): Rate {
        return Rate.fromBps(this.vaultState.config.feeMgmtBps);
    }

    getFlags(): VaultFlags {
        return this.vaultState.bitflags;
    }

    getYiledSourceFlags(): YiledSourceFlags {
        return this.vaultState.yieldSourceFlags;
    }
}

const createAta = (
    owner: PublicKey,
    mint: PublicKey,
    address: PublicKey,
    feePayer?: PublicKey
): TransactionInstruction => {
    return SplToken.createAssociatedTokenAccountInstruction(
        ASSOCIATED_TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        mint,
        address,
        owner,
        feePayer ? feePayer : owner
    );
};

interface WrapSolIxResponse {
    openIxs: [TransactionInstruction, TransactionInstruction];
    closeIx: TransactionInstruction;
    keyPair: Keypair;
}
