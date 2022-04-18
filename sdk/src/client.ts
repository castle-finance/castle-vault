import Big from "big.js";
import {
    Cluster,
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

import { PROGRAM_ID } from ".";
import { CastleLendingAggregator } from "./castle_lending_aggregator";
import {
    PortReserveAsset,
    SolendReserveAsset,
    JetReserveAsset,
} from "./adapters";
import {
    StrategyType,
    RebalanceEvent,
    Vault,
    FeeArgs,
    ProposedWeightsBps,
    RebalanceMode,
    VaultFees,
} from "./types";

export class VaultClient {
    private constructor(
        public program: anchor.Program<CastleLendingAggregator>,
        public vaultId: PublicKey,
        private vaultState: Vault,
        private solend: SolendReserveAsset,
        private port: PortReserveAsset,
        private jet: JetReserveAsset
    ) {}

    // TODO add function to change wallet

    static async load(
        provider: anchor.Provider,
        cluster: Cluster,
        reserveMint: PublicKey,
        vaultId: PublicKey
    ): Promise<VaultClient> {
        const program = (await anchor.Program.at(
            PROGRAM_ID,
            provider
        )) as anchor.Program<CastleLendingAggregator>;
        const vaultState = await program.account.vault.fetch(vaultId);

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
        program: anchor.Program<CastleLendingAggregator>,
        wallet: anchor.Wallet,
        reserveTokenMint: PublicKey,
        solend: SolendReserveAsset,
        port: PortReserveAsset,
        jet: JetReserveAsset,
        strategyType: StrategyType,
        rebalanceMode: RebalanceMode,
        owner: PublicKey,
        feeData: FeeArgs,
        poolSizeLimit: number = 10000000000
    ): Promise<VaultClient> {
        const { feeCarryBps, feeMgmtBps, referralFeeOwner, referralFeePct } =
            feeData;

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

        const txSig = await program.rpc.initialize(
            {
                authority: authorityBump,
                reserve: reserveBump,
                lpMint: lpTokenMintBump,
                solendLp: solendLpBump,
                portLp: portLpBump,
                jetLp: jetLpBump,
            },
            strategyType,
            rebalanceMode,
            {
                feeCarryBps: new anchor.BN(feeCarryBps),
                feeMgmtBps: new anchor.BN(feeMgmtBps),
                referralFeePct: new anchor.BN(referralFeePct),
            },
            new anchor.BN(poolSizeLimit),
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

    private getRefreshIx(): TransactionInstruction {
        return this.program.instruction.refresh({
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
                portOracle: this.port.accounts.oracle,
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
            remainingAccounts: [
                {
                    isSigner: false,
                    isWritable: true,
                    pubkey: this.vaultState.fees.feeReceiver,
                },
                {
                    isSigner: false,
                    isWritable: true,
                    pubkey: this.vaultState.fees.referralFeeReceiver,
                },
            ],
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
    async updateDepositCap(
        owner: Keypair,
        new_value: number
    ): Promise<TransactionSignature[]> {
        const updateCommand = new Transaction();
        updateCommand.add(
            this.program.instruction.updateDepositCap(
                new anchor.BN(new_value),
                {
                    accounts: {
                        vault: this.vaultId,
                        owner: owner.publicKey,
                    },
                }
            )
        );
        return [await this.program.provider.send(updateCommand, [owner])];
    }

    /**
     * @param new_value
     * @returns
     */
    async updateFees(
        owner: Keypair,
        feeCarryBps: number,
        feeMgmtBps: number,
        referralFeePct: number
    ): Promise<TransactionSignature[]> {
        const updateCommand = new Transaction();
        updateCommand.add(
            this.program.instruction.updateFees(
                {
                    feeCarryBps: feeCarryBps,
                    feeMgmtBps: feeMgmtBps,
                    referralFeePct: referralFeePct,
                },
                {
                    accounts: {
                        vault: this.vaultId,
                        owner: owner.publicKey,
                    },
                }
            )
        );
        return [await this.program.provider.send(updateCommand, [owner])];
    }

    /**
     *
     * TODO refactor to be more clear
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
                    wallet,
                    this.vaultState.lpTokenMint,
                    userLpTokenAccount
                )
            );
        }

        depositTx.add(this.getRefreshIx());
        depositTx.add(
            this.program.instruction.deposit(new anchor.BN(amount), {
                accounts: {
                    vault: this.vaultId,
                    vaultAuthority: this.vaultState.vaultAuthority,
                    vaultReserveToken: this.vaultState.vaultReserveToken,
                    lpTokenMint: this.vaultState.lpTokenMint,
                    userReserveToken: userReserveTokenAccount,
                    userLpToken: userLpTokenAccount,
                    userAuthority: wallet.publicKey,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    clock: SYSVAR_CLOCK_PUBKEY,
                },
            })
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

        // Withdraw from lending markets if not enough reserves in vault
        const vaultReserveTokenAccountInfo =
            await this.getReserveTokenAccountInfo(
                this.vaultState.vaultReserveToken
            );
        const vaultReserveAmount = new Big(
            vaultReserveTokenAccountInfo.amount.toString()
        ).round(0, Big.roundDown);

        // Convert from reserve tokens to LP tokens
        // NOTE: this rate is slightly lower than what it will be in the transaction
        //  by about 1/10000th of the current yield (1bp per 100%).
        //  To avoid a insufficient funds error, we slightly over-correct for this
        const exchangeRate = await this.getLpExchangeRate();
        const adjustFactor = (await this.getApy()).mul(0.0001);
        const convertedAmount = exchangeRate
            .mul(amount)
            .mul(new Big(1).add(adjustFactor))
            .round(0, Big.roundUp);

        if (vaultReserveAmount.lt(convertedAmount)) {
            // Sort reconcile ixs by most to least outflows
            const reconcileIxs = (
                await this.newAndOldallocationsWithReconcileIxs()
            ).sort((a, b) => a[0].sub(a[1]).sub(b[0].sub(b[1])).toNumber());

            const toWithdrawAmount = convertedAmount
                .sub(vaultReserveAmount)
                .toNumber();
            // TODO use bignumber
            let withdrawnAmount = 0;
            let n = 0;
            while (withdrawnAmount < toWithdrawAmount) {
                const [, oldAlloc, ix] = reconcileIxs[n];
                const reconcileAmount = Math.min(
                    oldAlloc.toNumber(),
                    toWithdrawAmount
                );
                if (reconcileAmount != 0) {
                    const reconcileTx = new Transaction();
                    reconcileTx.add(this.getRefreshIx());
                    reconcileTx.add(ix(new anchor.BN(reconcileAmount)));

                    txs.push({ tx: reconcileTx, signers: [] });
                }

                withdrawnAmount += oldAlloc.toNumber();
                n += 1;
            }
        }

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
                        wallet,
                        this.vaultState.reserveTokenMint,
                        userReserveTokenAccount
                    )
                );
            }
        }

        withdrawTx.add(this.getRefreshIx());
        withdrawTx.add(
            this.program.instruction.withdraw(
                new anchor.BN(Math.floor(amount)),
                {
                    accounts: {
                        vault: this.vaultId,
                        vaultAuthority: this.vaultState.vaultAuthority,
                        userAuthority: wallet.publicKey,
                        userLpToken: userLpTokenAccount,
                        userReserveToken: userReserveTokenAccount,
                        vaultReserveToken: this.vaultState.vaultReserveToken,
                        lpTokenMint: this.vaultState.lpTokenMint,
                        tokenProgram: TOKEN_PROGRAM_ID,
                        clock: SYSVAR_CLOCK_PUBKEY,
                    },
                }
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
            this.vaultState.rebalanceMode == { proofChecker: {} } &&
            proposedWeights == null
        ) {
            throw new Error(
                "Proposed weights must be passed in for a vault running in proofChecker mode"
            );
        }

        const txs: SendTxRequest[] = [
            { tx: this.getRebalanceTx(proposedWeights), signers: [] },
        ];

        // Sort ixs in ascending order of outflows
        const oldAndNewallocationsWithReconcileIxs =
            await this.newAndOldallocationsWithReconcileIxs(proposedWeights);

        const allocationDiffsWithReconcileIxs: [Big, TransactionInstruction][] =
            oldAndNewallocationsWithReconcileIxs.map((e) => [
                e[0].sub(e[1]),
                e[2](),
            ]);

        const reconcileIxs = allocationDiffsWithReconcileIxs
            .sort((a, b) => a[0].sub(b[0]).toNumber())
            .map((e) => e[1]);

        for (const ix of reconcileIxs) {
            const reconcileTx = new Transaction();
            reconcileTx.add(this.getRefreshIx());
            reconcileTx.add(ix);
            txs.push({ tx: reconcileTx, signers: [] });
        }
        return this.program.provider.sendAll(txs);
    }

    // TODO this is probably not the best way to do this?
    private async newAndOldallocationsWithReconcileIxs(
        proposedWeights?: ProposedWeightsBps
    ): Promise<
        [Big, Big, (withdrawOption?: anchor.BN) => TransactionInstruction][]
    > {
        if (proposedWeights == null) {
            proposedWeights = {
                solend: 0,
                port: 0,
                jet: 0,
            };
        }
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
        ).events[0].data as RebalanceEvent;

        return [
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

    getDepositCap(): Big {
        return new Big(this.vaultState.depositCap.toString());
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
            address
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
        return lpToken.getAccountInfo(this.vaultState.fees.feeReceiver);
    }

    async getReferralFeeReceiverAccountInfo(): Promise<AccountInfo> {
        const lpToken = new Token(
            this.program.provider.connection,
            this.vaultState.lpTokenMint,
            TOKEN_PROGRAM_ID,
            Keypair.generate() // dummy since we don't need to send txs
        );
        return lpToken.getAccountInfo(this.vaultState.fees.referralFeeReceiver);
    }

    async debug_log() {
        console.log(
            "solend value: ",
            (
                await this.solend.getLpTokenAccountValue(
                    this.vaultState.vaultSolendLpToken
                )
            ).toNumber()
        );
        console.log(
            "port value: ",
            (
                await this.port.getLpTokenAccountValue(
                    this.vaultState.vaultPortLpToken
                )
            ).toNumber()
        );
        console.log(
            "jet value: ",
            (
                await this.jet.getLpTokenAccountValue(
                    this.vaultState.vaultJetLpToken
                )
            ).toNumber()
        );
    }

    // NOTE: These should really only be used for testing

    getSolend(): SolendReserveAsset {
        return this.solend;
    }

    getPort(): PortReserveAsset {
        return this.port;
    }

    getJet(): JetReserveAsset {
        return this.jet;
    }

    getFees(): VaultFees {
        return this.vaultState.fees;
    }
}

const createAta = (
    wallet: anchor.Wallet,
    mint: PublicKey,
    address: PublicKey
): TransactionInstruction => {
    return Token.createAssociatedTokenAccountInstruction(
        ASSOCIATED_TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        mint,
        address,
        wallet.publicKey,
        wallet.publicKey
    );
};

interface WrapSolIxResponse {
    openIxs: [TransactionInstruction, TransactionInstruction];
    closeIx: TransactionInstruction;
    keyPair: Keypair;
}
