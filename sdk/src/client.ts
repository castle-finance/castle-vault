import Big from "big.js";
import {
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
    Token,
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

export class VaultClient {
    private constructor(
        public program: anchor.Program<CastleVault>,
        public vaultId: PublicKey,
        private vaultState: Vault,
        private solend: SolendReserveAsset,
        private port: PortReserveAsset,
        private jet: JetReserveAsset,
        private feesEnabled: boolean = false
    ) {}

    static async load(
        provider: anchor.AnchorProvider,
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

        return new VaultClient(program, vaultId, vaultState, solend, port, jet);
    }

    async reload() {
        this.vaultState = await this.program.account.vault.fetch(this.vaultId);
        // TODO reload underlying asset data also?
    }

    static async initialize(
        provider: anchor.AnchorProvider,
        wallet: anchor.Wallet,
        env: DeploymentEnv,
        reserveTokenMint: PublicKey,
        solend: SolendReserveAsset,
        port: PortReserveAsset,
        jet: JetReserveAsset,
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

        const [vaultSolendLpTokenAccount, solendLpBump] =
            await PublicKey.findProgramAddress(
                [
                    vaultId.publicKey.toBuffer(),
                    solend.accounts.collateralMint.toBuffer(),
                ],
                program.programId
            );

        const [vaultPortLpTokenAccount, portLpBump] =
            await PublicKey.findProgramAddress(
                [
                    vaultId.publicKey.toBuffer(),
                    port.accounts.collateralMint.toBuffer(),
                ],
                program.programId
            );

        const [vaultJetLpTokenAccount, jetLpBump] =
            await PublicKey.findProgramAddress(
                [
                    vaultId.publicKey.toBuffer(),
                    jet.accounts.depositNoteMint.toBuffer(),
                ],
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

        const feeReceiver = await Token.getAssociatedTokenAddress(
            ASSOCIATED_TOKEN_PROGRAM_ID,
            TOKEN_PROGRAM_ID,
            lpTokenMint,
            owner
        );

        const referralFeeReceiver = await Token.getAssociatedTokenAddress(
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
                solendLp: solendLpBump,
                portLp: portLpBump,
                jetLp: jetLpBump,
            },
            { ...defaultConfig, ...config },
            {
                accounts: {
                    vault: vaultId.publicKey,
                    vaultAuthority: vaultAuthority,
                    lpTokenMint: lpTokenMint,
                    vaultReserveToken: vaultReserveTokenAccount,
                    vaultSolendLpToken: vaultSolendLpTokenAccount,
                    vaultPortLpToken: vaultPortLpTokenAccount,
                    vaultJetLpToken: vaultJetLpTokenAccount,
                    reserveTokenMint: reserveTokenMint,
                    solendLpTokenMint: solend.accounts.collateralMint,
                    portLpTokenMint: port.accounts.collateralMint,
                    jetLpTokenMint: jet.accounts.depositNoteMint,
                    solendReserve: solend.accounts.reserve,
                    portReserve: port.accounts.reserve,
                    jetReserve: jet.accounts.reserve,
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

        return new VaultClient(
            program,
            vaultId.publicKey,
            vaultState,
            solend,
            port,
            jet
        );
    }

    getRefreshIx(): TransactionInstruction {
        // Port does not accept an oracle as input if the reserve is denominated
        // in the same token as the market quote currency (USDC).
        // We account for this by passing in an argument that indicates whether
        // or not to use the given oracle value.
        let usePortOracle = true;
        let portOracle = this.port.accounts.oracle;
        if (portOracle == null) {
            usePortOracle = false;
            portOracle = Keypair.generate().publicKey;
        }

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

        return this.program.instruction.refresh(usePortOracle, {
            accounts: {
                vault: this.vaultId,
                vaultAuthority: this.vaultState.vaultAuthority,
                vaultReserveToken: this.vaultState.vaultReserveToken,
                vaultSolendLpToken: this.vaultState.vaultSolendLpToken,
                vaultPortLpToken: this.vaultState.vaultPortLpToken,
                vaultJetLpToken: this.vaultState.vaultJetLpToken,
                lpTokenMint: this.vaultState.lpTokenMint,
                solendProgram: this.solend.accounts.program,
                solendReserve: this.solend.accounts.reserve,
                solendPyth: this.solend.accounts.pythPrice,
                solendSwitchboard: this.solend.accounts.switchboardFeed,
                portProgram: this.port.accounts.program,
                portReserve: this.port.accounts.reserve,
                portOracle: portOracle,
                jetProgram: this.jet.accounts.program,
                jetMarket: this.jet.accounts.market,
                jetMarketAuthority: this.jet.accounts.marketAuthority,
                jetReserve: this.jet.accounts.reserve,
                jetFeeNoteVault: this.jet.accounts.feeNoteVault,
                jetDepositNoteMint: this.jet.accounts.depositNoteMint,
                jetPyth: this.jet.accounts.pythPrice,
                tokenProgram: TOKEN_PROGRAM_ID,
                clock: SYSVAR_CLOCK_PUBKEY,
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

        const rent = await Token.getMinBalanceRentForExemptAccount(
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
                Token.createInitAccountInstruction(
                    TOKEN_PROGRAM_ID,
                    NATIVE_MINT,
                    userReserveTokenAccount,
                    wallet.publicKey
                ),
            ],
            closeIx: Token.createCloseAccountInstruction(
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

        depositTx.add(this.getRefreshIx());
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

        withdrawTx.add(this.getRefreshIx());
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
        const adjustFactor = (await this.getApy()).mul(0.0001);
        const convertedAmount = exchangeRate
            .mul(amount)
            .mul(new Big(1).add(adjustFactor))
            .round(0, Big.roundUp);

        if (vaultReserveAmount.lt(convertedAmount)) {
            // Sort reconcile ixs by most to least $ to minimize number of TXs sent
            const reconcileIxs = [
                [
                    await this.solend.getLpTokenAccountValue(
                        this.vaultState.vaultSolendLpToken
                    ),
                    this.getReconcileSolendIx.bind(this),
                ],
                [
                    await this.port.getLpTokenAccountValue(
                        this.vaultState.vaultPortLpToken
                    ),
                    this.getReconcilePortIx.bind(this),
                ],
                [
                    await this.jet.getLpTokenAccountValue(
                        this.vaultState.vaultJetLpToken
                    ),
                    this.getReconcileJetIx.bind(this),
                ],
            ].sort((a, b) => b[0].sub(a[0]).toNumber());

            const toReconcileAmount = convertedAmount.sub(vaultReserveAmount);
            let reconciledAmount = Big(0);
            let n = 0;
            while (reconciledAmount.lt(toReconcileAmount)) {
                const [alloc, ix] = reconcileIxs[n];
                // min of alloc and toWithdrawAmount - withdrawnAmount
                const reconcileAmount = alloc.gt(
                    toReconcileAmount.sub(reconciledAmount)
                )
                    ? toReconcileAmount
                    : alloc;

                if (!Big(0).eq(reconcileAmount)) {
                    const reconcileTx = new Transaction();
                    reconcileTx.add(this.getRefreshIx());
                    reconcileTx.add(
                        ix(new anchor.BN(reconcileAmount.toString()))
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
        rebalanceTx.add(this.getRefreshIx());
        rebalanceTx.add(
            this.program.instruction.rebalance(proposedWeights, {
                accounts: {
                    vault: this.vaultId,
                    solendReserve: this.solend.accounts.reserve,
                    portReserve: this.port.accounts.reserve,
                    jetReserve: this.jet.accounts.reserve,
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
                    solendReserve: this.solend.accounts.reserve,
                    portReserve: this.port.accounts.reserve,
                    jetReserve: this.jet.accounts.reserve,
                    clock: SYSVAR_CLOCK_PUBKEY,
                },
                instructions: [this.getRefreshIx()],
            })
        ).events[1].data as RebalanceDataEvent;

        const newAndOldallocationsWithReconcileIxs = [
            [
                new Big(newAllocations.solend.toString()),
                await this.solend.getLpTokenAccountValue(
                    this.vaultState.vaultSolendLpToken
                ),
                this.getReconcileSolendIx.bind(this),
            ],
            [
                new Big(newAllocations.port.toString()),
                await this.port.getLpTokenAccountValue(
                    this.vaultState.vaultPortLpToken
                ),
                this.getReconcilePortIx.bind(this),
            ],
            [
                new Big(newAllocations.jet.toString()),
                await this.jet.getLpTokenAccountValue(
                    this.vaultState.vaultJetLpToken
                ),
                this.getReconcileJetIx.bind(this),
            ],
        ];

        const allocationDiffsWithReconcileIxs: [Big, TransactionInstruction][] =
            newAndOldallocationsWithReconcileIxs.map((e) => [
                e[0].sub(e[1]),
                e[2](),
            ]);

        const reconcileIxs = allocationDiffsWithReconcileIxs
            .sort((a, b) => a[0].sub(b[0]).toNumber())
            .map((e) => e[1]);

        const txs: SendTxRequest[] = [
            { tx: this.getRebalanceTx(proposedWeights), signers: [] },
        ];

        for (const ix of reconcileIxs) {
            const reconcileTx = new Transaction();
            reconcileTx.add(this.getRefreshIx());
            reconcileTx.add(ix);
            txs.push({ tx: reconcileTx, signers: [] });
        }
        return this.program.provider.sendAll(txs);
    }

    private getReconcilePortIx(
        withdrawOption: anchor.BN = new anchor.BN(0)
    ): TransactionInstruction {
        return this.program.instruction.reconcilePort(withdrawOption, {
            accounts: {
                vault: this.vaultId,
                vaultAuthority: this.vaultState.vaultAuthority,
                vaultReserveToken: this.vaultState.vaultReserveToken,
                vaultPortLpToken: this.vaultState.vaultPortLpToken,
                portProgram: this.port.accounts.program,
                portMarketAuthority: this.port.accounts.marketAuthority,
                portMarket: this.port.accounts.market,
                portReserve: this.port.accounts.reserve,
                portLpMint: this.port.accounts.collateralMint,
                portReserveToken: this.port.accounts.liquiditySupply,
                clock: SYSVAR_CLOCK_PUBKEY,
                tokenProgram: TOKEN_PROGRAM_ID,
            },
        });
    }

    private getReconcileJetIx(
        withdrawOption: anchor.BN = new anchor.BN(0)
    ): TransactionInstruction {
        return this.program.instruction.reconcileJet(withdrawOption, {
            accounts: {
                vault: this.vaultId,
                vaultAuthority: this.vaultState.vaultAuthority,
                vaultReserveToken: this.vaultState.vaultReserveToken,
                vaultJetLpToken: this.vaultState.vaultJetLpToken,
                jetProgram: this.jet.accounts.program,
                jetMarket: this.jet.accounts.market,
                jetMarketAuthority: this.jet.accounts.marketAuthority,
                jetReserve: this.jet.accounts.reserve,
                jetReserveToken: this.jet.accounts.liquiditySupply,
                jetLpMint: this.jet.accounts.depositNoteMint,
                tokenProgram: TOKEN_PROGRAM_ID,
            },
        });
    }

    private getReconcileSolendIx(
        withdrawOption: anchor.BN = new anchor.BN(0)
    ): TransactionInstruction {
        return this.program.instruction.reconcileSolend(withdrawOption, {
            accounts: {
                vault: this.vaultId,
                vaultAuthority: this.vaultState.vaultAuthority,
                vaultReserveToken: this.vaultState.vaultReserveToken,
                vaultSolendLpToken: this.vaultState.vaultSolendLpToken,
                solendProgram: this.solend.accounts.program,
                solendMarketAuthority: this.solend.accounts.marketAuthority,
                solendMarket: this.solend.accounts.market,
                solendReserve: this.solend.accounts.reserve,
                solendLpMint: this.solend.accounts.collateralMint,
                solendReserveToken: this.solend.accounts.liquiditySupply,
                clock: SYSVAR_CLOCK_PUBKEY,
                tokenProgram: TOKEN_PROGRAM_ID,
            },
        });
    }

    /**
     * @todo account for unallocated tokens
     *
     * @returns
     */
    async getApy(): Promise<Big> {
        // Weighted average of APYs by allocation
        const assetApysAndValues: [Big, Big][] = [
            [
                new Big(0),
                new Big(
                    (
                        await this.getReserveTokenAccountInfo(
                            this.vaultState.vaultReserveToken
                        )
                    ).amount.toString()
                ),
            ],
            [
                await this.solend.getApy(),
                await this.solend.getLpTokenAccountValue(
                    this.vaultState.vaultSolendLpToken
                ),
            ],
            [
                await this.port.getApy(),
                await this.port.getLpTokenAccountValue(
                    this.vaultState.vaultPortLpToken
                ),
            ],
            [
                await this.jet.getApy(),
                await this.jet.getLpTokenAccountValue(
                    this.vaultState.vaultJetLpToken
                ),
            ],
        ];
        const [valueSum, weightSum] = assetApysAndValues.reduce(
            ([valueSum, weightSum], [value, weight]) => [
                weight.mul(value).add(valueSum),
                weightSum.add(weight),
            ],
            [new Big(0), new Big(0)]
        );
        if (weightSum.eq(new Big(0))) {
            return new Big(0);
        } else {
            return valueSum.div(weightSum);
        }
    }

    // Denominated in reserve tokens per LP token
    async getLpExchangeRate(): Promise<Big> {
        const totalValue = await this.getTotalValue();
        const lpTokenMintInfo = await this.getLpTokenMintInfo();
        const lpTokenSupply = new Big(lpTokenMintInfo.supply.toString());

        const bigZero = new Big(0);
        if (lpTokenSupply.eq(bigZero) || totalValue.eq(bigZero)) {
            return new Big(1);
        } else {
            return totalValue.div(lpTokenSupply);
        }
    }

    /**
     * Gets the total value stored in the vault, denominated in reserve tokens
     *
     * @returns
     */
    async getTotalValue(): Promise<Big> {
        await this.reload();

        const values = [
            await this.solend.getLpTokenAccountValue(
                this.vaultState.vaultSolendLpToken
            ),
            await this.port.getLpTokenAccountValue(
                this.vaultState.vaultPortLpToken
            ),
            await this.jet.getLpTokenAccountValue(
                this.vaultState.vaultJetLpToken
            ),
            new Big(
                (
                    await this.getReserveTokenAccountInfo(
                        this.vaultState.vaultReserveToken
                    )
                ).amount.toString()
            ),
        ];
        const valueSum = values.reduce((a, b) => a.add(b), new Big(0));

        return valueSum;
    }

    async getUserValue(address: PublicKey): Promise<Big> {
        const userLpTokenAccount = await this.getUserLpTokenAccount(address);
        try {
            const userLpTokenAccountInfo = await this.getLpTokenAccountInfo(
                userLpTokenAccount
            );
            const userLpTokenAmount = new Big(
                userLpTokenAccountInfo.amount.toString()
            );
            const exchangeRate = await this.getLpExchangeRate();
            return userLpTokenAmount.mul(exchangeRate);
        } catch {
            return new Big(0);
        }
    }

    async getVaultReserveTokenAccountValue(): Promise<Big> {
        return Big(
            (
                await this.getReserveTokenAccountInfo(
                    this.getVaultReserveTokenAccount()
                )
            ).amount.toString()
        );
    }

    async getVaultSolendLpTokenAccountValue(): Promise<Big> {
        return this.solend.getLpTokenAccountValue(
            this.getVaultSolendLpTokenAccount()
        );
    }

    async getVaultPortLpTokenAccountValue(): Promise<Big> {
        return this.port.getLpTokenAccountValue(
            this.getVaultPortLpTokenAccount()
        );
    }

    async getVaultJetLpTokenAccountValue(): Promise<Big> {
        return this.jet.getLpTokenAccountValue(
            this.getVaultJetLpTokenAccount()
        );
    }

    /**
     * Calculates the ATA given the user's address and vault mint
     * @param address Users public key
     * @returns Users reserve ATA given vault reserve mint
     */
    async getUserReserveTokenAccount(address: PublicKey): Promise<PublicKey> {
        return await Token.getAssociatedTokenAddress(
            ASSOCIATED_TOKEN_PROGRAM_ID,
            TOKEN_PROGRAM_ID,
            this.vaultState.reserveTokenMint,
            address
        );
    }

    async getReserveTokenAccountInfo(address: PublicKey): Promise<AccountInfo> {
        const reserveToken = new Token(
            this.program.provider.connection,
            this.vaultState.reserveTokenMint,
            TOKEN_PROGRAM_ID,
            Keypair.generate() // dummy since we don't need to send txs
        );
        return reserveToken.getAccountInfo(address);
    }

    async getUserLpTokenAccount(address: PublicKey): Promise<PublicKey> {
        return await Token.getAssociatedTokenAddress(
            ASSOCIATED_TOKEN_PROGRAM_ID,
            TOKEN_PROGRAM_ID,
            this.vaultState.lpTokenMint,
            address,
            true
        );
    }

    async getLpTokenAccountInfo(address: PublicKey): Promise<AccountInfo> {
        const lpToken = new Token(
            this.program.provider.connection,
            this.vaultState.lpTokenMint,
            TOKEN_PROGRAM_ID,
            Keypair.generate() // dummy since we don't need to send txs
        );
        return lpToken.getAccountInfo(address);
    }

    async getLpTokenMintInfo(): Promise<MintInfo> {
        const lpToken = new Token(
            this.program.provider.connection,
            this.vaultState.lpTokenMint,
            TOKEN_PROGRAM_ID,
            Keypair.generate() // dummy since we don't need to send txs
        );
        return lpToken.getMintInfo();
    }

    async getFeeReceiverAccountInfo(): Promise<AccountInfo> {
        const lpToken = new Token(
            this.program.provider.connection,
            this.vaultState.lpTokenMint,
            TOKEN_PROGRAM_ID,
            Keypair.generate() // dummy since we don't need to send txs
        );
        return lpToken.getAccountInfo(this.vaultState.feeReceiver);
    }

    async getReferralFeeReceiverAccountInfo(): Promise<AccountInfo> {
        const lpToken = new Token(
            this.program.provider.connection,
            this.vaultState.lpTokenMint,
            TOKEN_PROGRAM_ID,
            Keypair.generate() // dummy since we don't need to send txs
        );
        return lpToken.getAccountInfo(this.vaultState.referralFeeReceiver);
    }

    // TODO remove this
    getVaultState(): Vault {
        return this.vaultState;
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

    getDepositCap(): Big {
        return new Big(this.vaultState.config.depositCap.toString());
    }

    getAllocationCap(): number {
        return this.vaultState.config.allocationCapPct;
    }

    getVaultReserveTokenAccount(): PublicKey {
        return this.vaultState.vaultReserveToken;
    }

    getVaultSolendLpTokenAccount(): PublicKey {
        return this.vaultState.vaultSolendLpToken;
    }

    getVaultPortLpTokenAccount(): PublicKey {
        return this.vaultState.vaultPortLpToken;
    }

    getVaultJetLpTokenAccount(): PublicKey {
        return this.vaultState.vaultJetLpToken;
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
        return this.solend;
    }

    getPort(): PortReserveAsset {
        return this.port;
    }

    getJet(): JetReserveAsset {
        return this.jet;
    }

    getReferralFeeSplit(): number {
        return this.vaultState.config.referralFeePct / 100;
    }

    getCarryFee(): number {
        return this.vaultState.config.feeCarryBps / 10000;
    }

    getManagementFee(): number {
        return this.vaultState.config.feeMgmtBps / 10000;
    }

    getFlags(): VaultFlags {
        return this.vaultState.bitflags;
    }
}

const createAta = (
    owner: PublicKey,
    mint: PublicKey,
    address: PublicKey,
    feePayer?: PublicKey
): TransactionInstruction => {
    return Token.createAssociatedTokenAccountInstruction(
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
