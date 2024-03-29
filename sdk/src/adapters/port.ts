import Big from "big.js";

import {
    Cluster,
    Keypair,
    PublicKey,
    SystemProgram,
    Transaction,
    TransactionInstruction,
    SYSVAR_CLOCK_PUBKEY,
    SYSVAR_RENT_PUBKEY,
    TransactionSignature,
    Signer,
} from "@solana/web3.js";
import {
    getAssociatedTokenAddress,
    createAssociatedTokenAccount,
} from "@project-serum/associated-token";
import { TOKEN_PROGRAM_ID, Token as SplToken } from "@solana/spl-token";
import { ENV } from "@solana/spl-token-registry";
import * as anchor from "@castlefinance/anchor";

import {
    AssetConfig,
    AssetDepositConfig,
    AssetDisplayConfig,
    AssetPrice,
    AssetPriceConfig,
    DEFAULT_PORT_LENDING_MARKET,
    Environment,
    MintId,
    Port,
    PORT_STAKING,
    ReserveConfigProto,
    ReserveId,
    TokenAccount,
    StakeAccount,
    StakingPoolLayout,
    initLendingMarketInstruction,
    initReserveInstruction,
    initStakingPoolInstruction,
    initObligationInstruction,
    createStakeAccountInstruction,
    refreshReserveInstruction,
    refreshObligationInstruction,
    depositReserveLiquidityAndAddCollateralInstruction,
    borrowObligationLiquidityInstruction,
} from "@castlefinance/port-sdk";

import { CastleVault } from "../idl";
import { Vault } from "../types";
import { Rate, Token, TokenAmount } from "../utils";

import { LendingMarket } from "./asset";
import { getToken } from "./utils";

interface PortAccounts {
    program: PublicKey;
    market: PublicKey;
    marketAuthority: PublicKey;
    reserve: PublicKey;
    collateralMint: PublicKey;
    oracle: PublicKey;
    liquiditySupply: PublicKey;
    liquidityFeeReceiver: PublicKey;
    lpTokenAccount: PublicKey;
    stakingPool: PublicKey;
    stakingRewardPool: PublicKey;
    stakingRewardTokenMint: PublicKey;
    stakingSubRewardPool: PublicKey;
    stakingSubRewardTokenMint: PublicKey;
    stakingProgram: PublicKey;
    stakingProgamAuthority: PublicKey;
    stakingRewardOracle?: PublicKey;
    stakingSubRewardOracle?: PublicKey;
    // Some port accounts held by the vault. The rest are still in vaultState
    // TODO refactor this in the future, maybe move all port-related accounts to this struct
    vaultPortAdditionalStates?: PublicKey;
    vaultPortObligation?: PublicKey;
    vaultPortStakeAccount?: PublicKey;
    vaultPortRewardToken?: PublicKey;
    vaultPortSubRewardToken?: PublicKey;
}

// TODO use constant from port sdk
// WF port team to make it public
// https://github.com/port-finance/port-sdk/blob/v2/src/utils/AssetConfigs.ts
const DEVNET_ASSETS = [
    new AssetConfig(
        MintId.fromBase58("So11111111111111111111111111111111111111112"),
        new AssetDisplayConfig("Solana", "SOL", "#BC57C4"),
        AssetPriceConfig.fromDecimals(4),
        new AssetDepositConfig(
            ReserveId.fromBase58(
                "6FeVStQAGPWvfWijDHF7cTWRCi7He6vTT3ubfNhe9SPt"
            ),
            {
                min: 100_000_000, // min 0.1 SOL
                remain: 20_000_000, // remain 0.02 SOL
            }
        )
    ),
];

const PORT_USD_PYTH_PRICE_MAINNET = new PublicKey(
    "jrMH4afMEodMqirQ7P89q5bGNJxD8uceELcsZaVBDeh"
);
const PORT_USD_PYTH_PRICE_DEVNET = new PublicKey(
    "33ugpDWbC2mLrYSQvu1BHfykR8bt3MVc4S3YuuXMVRH3"
);

export class PortReserveAsset extends LendingMarket {
    private constructor(
        public provider: anchor.AnchorProvider,
        public accounts: PortAccounts,
        public client: Port,
        public reserveToken: Token,
        public lpToken: Token
    ) {
        super();
    }

    static async load(
        provider: anchor.AnchorProvider,
        cluster: Cluster,
        reserveMint: PublicKey
    ): Promise<PortReserveAsset> {
        let env: Environment;
        let market: PublicKey;
        let portUsdPythPrice: PublicKey;
        if (cluster == "devnet") {
            env = new Environment(
                ENV.Devnet,
                DEVNET_LENDING_PROGRAM_ID,
                PORT_STAKING,
                TOKEN_PROGRAM_ID,
                DEVNET_ASSETS
            );
            market = new PublicKey(
                "H27Quk3DSbu55T4dCr1NddTTSAezXwHU67FPCZVKLhSW"
            );
            portUsdPythPrice = PORT_USD_PYTH_PRICE_DEVNET;
        } else if (cluster == "mainnet-beta") {
            env = Environment.forMainNet();
            market = DEFAULT_PORT_LENDING_MARKET;
            portUsdPythPrice = PORT_USD_PYTH_PRICE_MAINNET;
        } else {
            throw new Error("Cluster ${cluster} not supported");
        }

        const client = new Port(provider.connection, env, market);
        const reserveContext = await client.getReserveContext();
        const reserve = reserveContext.getByAssetMintId(MintId.of(reserveMint));
        const stakingPools = await client.getStakingPoolContext();
        const stakingPoolId = await reserve.getStakingPoolId();
        const targetStakingPool = stakingPools.getStakingPool(stakingPoolId);
        const rewardMintRaw = await provider.connection.getAccountInfo(
            targetStakingPool.getRewardTokenPool()
        );
        const rewardTokenMint = TokenAccount.fromRaw({
            pubkey: targetStakingPool.getRewardTokenPool(),
            account: rewardMintRaw,
        });
        const [stakingProgamAuthority] = await PublicKey.findProgramAddress(
            [targetStakingPool.getId().toBuffer()],
            env.getStakingProgramPk()
        );

        // Get sub-reward accounts, these are optional
        const subRewardPool = targetStakingPool.getSubRewardTokenPool();
        let subrewardMint = undefined;
        let stakingSubRewardOracle = Keypair.generate().publicKey;
        if (subRewardPool != undefined) {
            const subrewardMintRaw = await provider.connection.getAccountInfo(
                targetStakingPool.getSubRewardTokenPool()
            );
            subrewardMint = TokenAccount.fromRaw({
                pubkey: targetStakingPool.getSubRewardTokenPool(),
                account: subrewardMintRaw,
            }).getMintId();

            // TODO get Pyth price accoutn for the subreward token vs USD
            // This is lower priority for now, because there's only one pool with subreward
            // and we're unlikely to create a vault for that soon.
            stakingSubRewardOracle = Keypair.generate().publicKey;
        }

        const [marketAuthority] = await PublicKey.findProgramAddress(
            [market.toBuffer()],
            env.getLendingProgramPk()
        );
        const accounts: PortAccounts = {
            program: env.getLendingProgramPk(),
            market: market,
            marketAuthority: marketAuthority,
            reserve: reserve.getReserveId(),
            collateralMint: reserve.getShareMintId(),
            oracle: reserve.getOracleId(),
            liquiditySupply: reserve.getAssetBalanceId(),
            liquidityFeeReceiver: reserve.getFeeBalanceId(),
            lpTokenAccount: reserve.getShareBalanceId(),
            stakingPool: targetStakingPool.getId(),
            stakingRewardPool: targetStakingPool.getRewardTokenPool(),
            stakingRewardTokenMint: rewardTokenMint.getMintId(),
            stakingSubRewardPool: subRewardPool,
            stakingSubRewardTokenMint: subrewardMint,
            stakingProgram: env.getStakingProgramPk(),
            stakingProgamAuthority: stakingProgamAuthority,
            stakingRewardOracle: portUsdPythPrice,
            stakingSubRewardOracle: stakingSubRewardOracle,
        };

        const lpToken = await getToken(
            provider.connection,
            new PublicKey(reserve.getShareMintId())
        );
        const reserveToken = await getToken(
            provider.connection,
            new PublicKey(reserve.getAssetMintId())
        );

        return new PortReserveAsset(
            provider,
            accounts,
            client,
            reserveToken,
            lpToken
        );
    }

    static async initialize(
        provider: anchor.AnchorProvider,
        owner: Keypair,
        reserveTokenMint: PublicKey,
        pythPrice: PublicKey,
        ownerReserveTokenAccount: PublicKey,
        initialReserveAmount: number,
        createSubRewardPool: boolean
    ): Promise<PortReserveAsset> {
        const env = new Environment(
            ENV.Devnet,
            DEVNET_LENDING_PROGRAM_ID,
            DEVNET_STAKING_PROGRAM_ID,
            TOKEN_PROGRAM_ID,
            []
        );
        const market = await createLendingMarket(provider);
        const accounts = await createDefaultReserve(
            provider,
            env,
            initialReserveAmount,
            reserveTokenMint,
            ownerReserveTokenAccount,
            market.publicKey,
            pythPrice,
            owner,
            createSubRewardPool
        );
        const client = new Port(provider.connection, env, market.publicKey);

        const lpToken = await getToken(
            provider.connection,
            accounts.collateralMint
        );
        const reserveToken = await getToken(
            provider.connection,
            reserveTokenMint
        );

        return new PortReserveAsset(
            provider,
            accounts,
            client,
            reserveToken,
            lpToken
        );
    }

    async loadAdditionalAccounts(
        program: anchor.Program<CastleVault>,
        vaultId: PublicKey,
        vaultState: Vault
    ) {
        const vaultPortAdditionalStateAddress =
            await PublicKey.createProgramAddress(
                [
                    vaultId.toBuffer(),
                    anchor.utils.bytes.utf8.encode("port_additional_state"),
                    new Uint8Array([vaultState.vaultPortAdditionalStateBump]),
                ],
                program.programId
            );
        const vaultPortAdditionalStates =
            await program.account.vaultPortAdditionalState.fetch(
                vaultPortAdditionalStateAddress
            );
        this.accounts.vaultPortAdditionalStates =
            vaultPortAdditionalStateAddress;
        this.accounts.vaultPortObligation =
            await PublicKey.createProgramAddress(
                [
                    vaultId.toBuffer(),
                    anchor.utils.bytes.utf8.encode("port_obligation"),
                    new Uint8Array([
                        vaultPortAdditionalStates.vaultPortObligationBump,
                    ]),
                ],
                program.programId
            );
        this.accounts.vaultPortStakeAccount =
            await PublicKey.createProgramAddress(
                [
                    vaultId.toBuffer(),
                    anchor.utils.bytes.utf8.encode("port_stake"),
                    new Uint8Array([
                        vaultPortAdditionalStates.vaultPortStakeAccountBump,
                    ]),
                ],
                program.programId
            );
        this.accounts.vaultPortRewardToken =
            await PublicKey.createProgramAddress(
                [
                    vaultId.toBuffer(),
                    anchor.utils.bytes.utf8.encode("port_reward"),
                    new Uint8Array([
                        vaultPortAdditionalStates.vaultPortRewardTokenBump,
                    ]),
                ],
                program.programId
            );
        this.accounts.vaultPortSubRewardToken =
            await PublicKey.createProgramAddress(
                [
                    vaultId.toBuffer(),
                    anchor.utils.bytes.utf8.encode("port_sub_reward"),
                    new Uint8Array([
                        vaultPortAdditionalStates.vaultPortSubRewardTokenBump,
                    ]),
                ],
                program.programId
            );
    }

    async borrow(
        user: Signer,
        userReserveTokenAccount: PublicKey,
        borrowAmount: number
    ): Promise<TransactionSignature[]> {
        const depositAmount = borrowAmount * 1.5;

        const userCollateralTokenAccount = await getAssociatedTokenAddress(
            user.publicKey,
            this.accounts.collateralMint
        );

        const ataInitTx = new Transaction().add(
            await createAssociatedTokenAccount(
                user.publicKey,
                user.publicKey,
                this.accounts.collateralMint
            )
        );
        const ataInitSig = await this.provider.sendAndConfirm(ataInitTx, [
            user,
        ]);

        const userObligation = await createAccount(
            this.provider,
            OBLIGATION_LEN,
            DEVNET_LENDING_PROGRAM_ID
        );

        const userStake = await createAccount(
            this.provider,
            STAKE_LEN,
            DEVNET_STAKING_PROGRAM_ID
        );

        const depositTx = new Transaction()
            .add(
                initObligationInstruction(
                    userObligation.publicKey,
                    this.accounts.market,
                    user.publicKey,
                    DEVNET_LENDING_PROGRAM_ID
                )
            )
            .add(
                createStakeAccountInstruction(
                    userStake.publicKey,
                    this.accounts.stakingPool,
                    user.publicKey,
                    DEVNET_STAKING_PROGRAM_ID
                )
            )
            .add(
                refreshReserveInstruction(
                    this.accounts.reserve,
                    this.accounts.oracle,
                    DEVNET_LENDING_PROGRAM_ID
                )
            )
            .add(
                refreshObligationInstruction(
                    userObligation.publicKey,
                    [],
                    [],
                    DEVNET_LENDING_PROGRAM_ID
                )
            )
            .add(
                depositReserveLiquidityAndAddCollateralInstruction(
                    depositAmount,
                    userReserveTokenAccount,
                    userCollateralTokenAccount,
                    this.accounts.reserve,
                    this.accounts.liquiditySupply,
                    this.accounts.collateralMint,
                    this.accounts.market,
                    this.accounts.marketAuthority,
                    this.accounts.lpTokenAccount,
                    userObligation.publicKey,
                    user.publicKey,
                    user.publicKey,
                    DEVNET_LENDING_PROGRAM_ID,
                    userStake.publicKey,
                    this.accounts.stakingPool,
                    DEVNET_STAKING_PROGRAM_ID
                )
            );

        const borrowTx = new Transaction()
            .add(
                refreshReserveInstruction(
                    this.accounts.reserve,
                    this.accounts.oracle,
                    DEVNET_LENDING_PROGRAM_ID
                )
            )
            .add(
                refreshObligationInstruction(
                    userObligation.publicKey,
                    [this.accounts.reserve],
                    [],
                    DEVNET_LENDING_PROGRAM_ID
                )
            )
            .add(
                borrowObligationLiquidityInstruction(
                    borrowAmount,
                    this.accounts.liquiditySupply,
                    userReserveTokenAccount,
                    this.accounts.reserve,
                    this.accounts.liquidityFeeReceiver,
                    userObligation.publicKey,
                    this.accounts.market,
                    this.accounts.marketAuthority,
                    user.publicKey,
                    DEVNET_LENDING_PROGRAM_ID
                )
            );

        const sigs = await this.provider.sendAll([
            { tx: depositTx, signers: [user] },
            { tx: borrowTx, signers: [user] },
        ]);

        return [ataInitSig, ...sigs];
    }

    async getLpTokenAccountValue(vaultState: Vault): Promise<TokenAmount> {
        const reserve = await this.client.getReserve(this.accounts.reserve);
        const exchangeRate = reserve.getExchangeRatio();

        const mint = reserve.getShareMintId();
        const lpToken = new SplToken(
            this.provider.connection,
            mint,
            TOKEN_PROGRAM_ID,
            Keypair.generate() // dummy signer since we aren't making any txs
        );

        const lpTokenAmount = AssetPrice.of(
            mint,
            (
                await lpToken.getAccountInfo(vaultState.vaultPortLpToken)
            ).amount.toNumber()
        );

        // We retrieve the amount of tokens staked and add it to the total Port LP token value.
        // Note that this is the amount of Port LP tokens. We need to convert to Castle LP tokens using the exchange rate.
        const raw = await this.provider.connection.getAccountInfo(
            new PublicKey(this.accounts.vaultPortStakeAccount)
        );
        const stakeAccountData = StakeAccount.fromRaw({
            pubkey: this.accounts.vaultPortStakeAccount,
            account: raw,
        });
        const stakedTokenValue = AssetPrice.of(
            mint,
            stakeAccountData.getDepositAmount().toU64().toNumber()
        );

        return TokenAmount.fromToken(
            this.reserveToken,
            lpTokenAmount
                .add(stakedTokenValue)
                .divide(exchangeRate.getUnchecked())
                .getRaw()
                .round(0, Big.roundDown)
        );
    }

    /**
     * @todo make this the same as program's calculation
     *
     * Continuously compounded APY
     *
     * @returns
     */
    async getApy(): Promise<Rate> {
        const reserve = await this.client.getReserve(this.accounts.reserve);
        const apr = reserve.getSupplyApy().getUnchecked();
        const apy = Math.expm1(apr.toNumber());

        return new Rate(Big(apy));
    }

    async getBorrowedAmount(): Promise<TokenAmount> {
        const reserve = await this.client.getReserve(this.accounts.reserve);
        const borrowed = reserve.getBorrowedAsset();
        // Need to round here because the SDK returns a non-int value
        // and retaining that value might cause problems for the fn caller
        return TokenAmount.fromToken(
            this.reserveToken,
            borrowed.getRaw().round()
        );
    }

    async getDepositedAmount(): Promise<TokenAmount> {
        const reserve = await this.client.getReserve(this.accounts.reserve);
        const total = reserve.getTotalAsset();
        // Need to round here because the SDK returns a non-int value
        // and retaining that value might cause problems for the fn caller
        return TokenAmount.fromToken(this.reserveToken, total.getRaw().round());
    }

    async getRefreshIx(
        program: anchor.Program<CastleVault>,
        vaultId: PublicKey,
        vaultState: Vault
    ): Promise<TransactionInstruction> {
        return program.methods
            .refreshPort()
            .accounts({
                vault: vaultId,
                portAdditionalStates: this.accounts.vaultPortAdditionalStates,
                vaultPortLpToken: vaultState.vaultPortLpToken,
                vaultPortStakeAccount: this.accounts.vaultPortStakeAccount,
                portLendProgram: this.accounts.program,
                portReserve: this.accounts.reserve,
                clock: SYSVAR_CLOCK_PUBKEY,
            })
            .remainingAccounts(
                this.accounts.oracle == null
                    ? []
                    : [
                          {
                              isSigner: false,
                              isWritable: false,
                              pubkey: this.accounts.oracle,
                          },
                      ]
            )
            .instruction();
    }

    async getReconcileIx(
        program: anchor.Program<CastleVault>,
        vaultId: PublicKey,
        vaultState: Vault,
        withdrawOption?: anchor.BN
    ): Promise<TransactionInstruction> {
        return program.methods
            .reconcilePort(
                withdrawOption == null ? new anchor.BN(0) : withdrawOption
            )
            .accounts({
                vault: vaultId,
                vaultAuthority: vaultState.vaultAuthority,
                vaultReserveToken: vaultState.vaultReserveToken,
                vaultPortLpToken: vaultState.vaultPortLpToken,
                portAdditionalStates: this.accounts.vaultPortAdditionalStates,
                vaultPortObligation: this.accounts.vaultPortObligation,
                vaultPortStakeAccount: this.accounts.vaultPortStakeAccount,
                vaultPortRewardToken: this.accounts.vaultPortRewardToken,
                portStakingPool: this.accounts.stakingPool,
                portLendProgram: DEVNET_LENDING_PROGRAM_ID,
                portStakeProgram: DEVNET_STAKING_PROGRAM_ID,
                portStakingAuthority: this.accounts.stakingProgamAuthority,
                portLpTokenAccount: this.accounts.lpTokenAccount,
                portMarketAuthority: this.accounts.marketAuthority,
                portMarket: this.accounts.market,
                portReserve: this.accounts.reserve,
                portLpMint: this.accounts.collateralMint,
                portReserveToken: this.accounts.liquiditySupply,
                clock: SYSVAR_CLOCK_PUBKEY,
                tokenProgram: TOKEN_PROGRAM_ID,
            })
            .instruction();
    }

    async getInitializeIx(
        program: anchor.Program<CastleVault>,
        vaultId: PublicKey,
        vaultAuthority: PublicKey,
        wallet: PublicKey,
        owner: PublicKey
    ): Promise<TransactionInstruction> {
        const [vaultPortLpTokenAccount] = await PublicKey.findProgramAddress(
            [vaultId.toBuffer(), this.accounts.collateralMint.toBuffer()],
            program.programId
        );

        return program.methods
            .initializePort()
            .accounts({
                vault: vaultId,
                vaultAuthority: vaultAuthority,
                vaultPortLpToken: vaultPortLpTokenAccount,
                portLpTokenMint: this.accounts.collateralMint,
                portReserve: this.accounts.reserve,
                owner: owner,
                payer: wallet,
                tokenProgram: TOKEN_PROGRAM_ID,
                systemProgram: SystemProgram.programId,
                rent: SYSVAR_RENT_PUBKEY,
            })
            .instruction();
    }

    async getClaimRewardIx(
        program: anchor.Program<CastleVault>,
        vaultId: PublicKey,
        vaultState: Vault
    ): Promise<TransactionInstruction> {
        const dummyKey = Keypair.generate().publicKey;
        return program.methods
            .claimPortReward()
            .accounts({
                vault: vaultId,
                vaultAuthority: vaultState.vaultAuthority,
                portAdditionalStates: this.accounts.vaultPortAdditionalStates,
                vaultPortStakeAccount: this.accounts.vaultPortStakeAccount,
                vaultPortRewardToken: this.accounts.vaultPortRewardToken,
                vaultPortSubRewardToken: this.accounts.vaultPortSubRewardToken,
                portStakingPool: this.accounts.stakingPool,
                portLendProgram: this.accounts.program,
                portStakeProgram: this.accounts.stakingProgram,
                portStakingRewardPool: this.accounts.stakingRewardPool,
                portStakingSubRewardPool:
                    this.accounts.stakingSubRewardPool != undefined
                        ? this.accounts.stakingSubRewardPool
                        : dummyKey,
                portStakingAuthority: this.accounts.stakingProgamAuthority,
                clock: SYSVAR_CLOCK_PUBKEY,
                tokenProgram: TOKEN_PROGRAM_ID,
            })
            .instruction();
    }

    async getUnclaimedStakingRewards(
        program: anchor.Program<CastleVault>
    ): Promise<number> {
        const stakingAccountRaw =
            await program.provider.connection.getAccountInfo(
                new PublicKey(this.accounts.vaultPortStakeAccount)
            );
        const stakingAccount = StakeAccount.fromRaw({
            pubkey: this.accounts.vaultPortStakeAccount,
            account: stakingAccountRaw,
        });
        return stakingAccount.getUnclaimedReward().toU64().toNumber();
    }
}

export const DEVNET_STAKING_PROGRAM_ID = new PublicKey(
    "stkarvwmSzv2BygN5e2LeTwimTczLWHCKPKGC2zVLiq"
);
const DEVNET_LENDING_PROGRAM_ID = new PublicKey(
    "pdQ2rQQU5zH2rDgZ7xH2azMBJegUzUyunJ5Jd637hC4"
);
const TOKEN_ACCOUNT_LEN = 165;
const TOKEN_MINT_LEN = 82;
const RESERVE_LEN = 575;
const LENDING_MARKET_LEN = 258;
const OBLIGATION_LEN = 916;
const STAKE_LEN = 1 + 16 + 32 + 32 + 8 + 16 + 16 + 1 + 16 + 1 + 94;

const DEFAULT_RESERVE_CONFIG: ReserveConfigProto = {
    optimalUtilizationRate: 80,
    loanToValueRatio: 80,
    liquidationBonus: 5,
    liquidationThreshold: 85,
    minBorrowRate: 0,
    optimalBorrowRate: 40,
    maxBorrowRate: 90,
    fees: {
        borrowFeeWad: new anchor.BN(10000000000000),
        flashLoanFeeWad: new anchor.BN(30000000000000),
        hostFeePercentage: 0,
    },
    stakingPoolOption: 0,
    stakingPool: TOKEN_PROGRAM_ID, // dummy
};

// TODO move to common utils
const createAccount = async (
    provider: anchor.AnchorProvider,
    space: number,
    owner: PublicKey
): Promise<Keypair> => {
    const newAccount = Keypair.generate();
    const createTx = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: provider.wallet.publicKey,
            newAccountPubkey: newAccount.publicKey,
            programId: owner,
            lamports:
                await provider.connection.getMinimumBalanceForRentExemption(
                    space
                ),
            space,
        })
    );
    await provider.sendAndConfirm(createTx, [newAccount]);
    return newAccount;
};

async function createLendingMarket(
    provider: anchor.AnchorProvider
): Promise<Keypair> {
    const lendingMarket = await createAccount(
        provider,
        LENDING_MARKET_LEN,
        DEVNET_LENDING_PROGRAM_ID
    );
    await provider.sendAndConfirm(
        (() => {
            const tx = new Transaction();
            tx.add(
                initLendingMarketInstruction(
                    provider.wallet.publicKey,
                    Buffer.from(
                        "USD\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
                        "ascii"
                    ),
                    lendingMarket.publicKey,
                    DEVNET_LENDING_PROGRAM_ID
                )
            );
            return tx;
        })(),
        []
    );
    return lendingMarket;
}

async function createStakingPool(
    provider: anchor.AnchorProvider,
    owner: Keypair,
    supply: number,
    duration: number,
    rewardTime: number,
    authority: PublicKey,
    createSubRewardPool: boolean
): Promise<PublicKey> {
    const supplyLamports = supply * 1000000;

    // This step will create a staking pool
    const stakingPool = await createAccount(
        provider,
        StakingPoolLayout.span,
        DEVNET_STAKING_PROGRAM_ID
    );

    // This step will create the reward token pool, held by the staking pool to store tokens to be distributed as rewards
    const rewardTokenPool = await createAccount(
        provider,
        TOKEN_ACCOUNT_LEN,
        TOKEN_PROGRAM_ID
    );

    // This step will create a mock reward token and mint some of it for testing
    const rewardMint = await SplToken.createMint(
        provider.connection,
        owner,
        owner.publicKey,
        null,
        6,
        TOKEN_PROGRAM_ID
    );
    const rewardSupply = await rewardMint.createAssociatedTokenAccount(
        owner.publicKey
    );
    await rewardMint.mintTo(rewardSupply, owner, [], supplyLamports);

    // This step will create the sub-reward token pool
    const subRewardTokenPool = await createAccount(
        provider,
        TOKEN_ACCOUNT_LEN,
        TOKEN_PROGRAM_ID
    );

    // TODO briefly explain what this is used for
    const [stakingProgramDerived, bumpSeed] =
        await PublicKey.findProgramAddress(
            [stakingPool.publicKey.toBuffer()],
            DEVNET_STAKING_PROGRAM_ID
        );

    let subReward = undefined;
    if (createSubRewardPool) {
        // This step will create a mock sub-reward token
        const subRewardMint = await SplToken.createMint(
            provider.connection,
            owner,
            owner.publicKey,
            null,
            6,
            TOKEN_PROGRAM_ID
        );
        const subRewardSupply =
            await subRewardMint.createAssociatedTokenAccount(owner.publicKey);
        await subRewardMint.mintTo(subRewardSupply, owner, [], supplyLamports);

        subReward = {
            supply: supplyLamports,
            tokenSupply: subRewardSupply,
            tokenPool: subRewardTokenPool.publicKey,
            rewardTokenMint: subRewardMint.publicKey,
        };
    }

    // Send the actual instruction to init staking pool
    const tx = new Transaction();
    tx.add(
        initStakingPoolInstruction(
            supplyLamports,
            duration,
            rewardTime,
            bumpSeed,
            owner.publicKey,
            rewardSupply,
            rewardTokenPool.publicKey,
            stakingPool.publicKey,
            rewardMint.publicKey,
            stakingProgramDerived,
            authority,
            authority,
            DEVNET_STAKING_PROGRAM_ID,
            subReward
        )
    );

    const sig1 = await provider.sendAll([{ tx: tx, signers: [owner] }]);
    await provider.connection.confirmTransaction(sig1[0], "finalized");

    return stakingPool.publicKey;
}

async function createDefaultReserve(
    provider: anchor.AnchorProvider,
    env: Environment,
    initialLiquidity: number | anchor.BN,
    liquidityMint: PublicKey,
    sourceTokenWallet: PublicKey,
    lendingMarket: PublicKey,
    oracle: PublicKey,
    owner: Keypair,
    createSubRewardPool: boolean
): Promise<PortAccounts> {
    const reserve = await createAccount(
        provider,
        RESERVE_LEN,
        DEVNET_LENDING_PROGRAM_ID
    );

    const collateralMintAccount = await createAccount(
        provider,
        TOKEN_MINT_LEN,
        TOKEN_PROGRAM_ID
    );

    const liquiditySupplyTokenAccount = await createAccount(
        provider,
        TOKEN_ACCOUNT_LEN,
        TOKEN_PROGRAM_ID
    );

    const collateralSupplyTokenAccount = await createAccount(
        provider,
        TOKEN_ACCOUNT_LEN,
        TOKEN_PROGRAM_ID
    );

    const userCollateralTokenAccount = await createAccount(
        provider,
        TOKEN_ACCOUNT_LEN,
        TOKEN_PROGRAM_ID
    );

    const liquidityFeeReceiver = await createAccount(
        provider,
        TOKEN_ACCOUNT_LEN,
        TOKEN_PROGRAM_ID
    );

    const [lendingMarketAuthority] = await PublicKey.findProgramAddress(
        [lendingMarket.toBuffer()],
        DEVNET_LENDING_PROGRAM_ID
    );

    const stakingPool = await createStakingPool(
        provider,
        owner,
        1000,
        5184000,
        0,
        lendingMarketAuthority,
        createSubRewardPool
    );

    const config = DEFAULT_RESERVE_CONFIG;
    config.stakingPoolOption = 1;
    config.stakingPool = stakingPool;

    const tx = new Transaction();

    tx.add(
        SplToken.createApproveInstruction(
            TOKEN_PROGRAM_ID,
            sourceTokenWallet,
            provider.wallet.publicKey,
            owner.publicKey,
            [],
            initialLiquidity
        )
    );
    tx.add(
        initReserveInstruction(
            initialLiquidity,
            0,
            new anchor.BN(0),
            config,
            sourceTokenWallet,
            userCollateralTokenAccount.publicKey,
            reserve.publicKey,
            liquidityMint,
            liquiditySupplyTokenAccount.publicKey,
            liquidityFeeReceiver.publicKey,
            oracle,
            collateralMintAccount.publicKey,
            collateralSupplyTokenAccount.publicKey,
            lendingMarket,
            lendingMarketAuthority,
            provider.wallet.publicKey,
            provider.wallet.publicKey,
            DEVNET_LENDING_PROGRAM_ID
        )
    );

    await provider.sendAndConfirm(tx, [owner]);

    // Double check account values
    const client = new Port(provider.connection, env, lendingMarket);
    const reserveContext = await client.getReserveContext();
    const reserveAcct = reserveContext.getByAssetMintId(
        MintId.of(liquidityMint)
    );
    const stakingPools = await client.getStakingPoolContext();
    const stakingPoolId = await reserveAcct.getStakingPoolId();
    const targetStakingPool = stakingPools.getStakingPool(stakingPoolId);
    const rewardMintRaw = await provider.connection.getAccountInfo(
        targetStakingPool.getRewardTokenPool()
    );
    const rewardTokenMint = TokenAccount.fromRaw({
        pubkey: targetStakingPool.getRewardTokenPool(),
        account: rewardMintRaw,
    });
    const [stakingProgamAuthority] = await PublicKey.findProgramAddress(
        [targetStakingPool.getId().toBuffer()],
        env.getStakingProgramPk()
    );

    const subRewardPool = targetStakingPool.getSubRewardTokenPool();
    let subrewardMint = undefined;
    if (subRewardPool != undefined) {
        const subrewardMintRaw = await provider.connection.getAccountInfo(
            targetStakingPool.getSubRewardTokenPool()
        );
        subrewardMint = TokenAccount.fromRaw({
            pubkey: targetStakingPool.getSubRewardTokenPool(),
            account: subrewardMintRaw,
        }).getMintId();
    }

    return {
        program: DEVNET_LENDING_PROGRAM_ID,
        market: lendingMarket,
        marketAuthority: lendingMarketAuthority,
        reserve: reserve.publicKey,
        oracle: oracle,
        collateralMint: collateralMintAccount.publicKey,
        liquiditySupply: liquiditySupplyTokenAccount.publicKey,
        liquidityFeeReceiver: liquidityFeeReceiver.publicKey,
        lpTokenAccount: collateralSupplyTokenAccount.publicKey,
        stakingPool: targetStakingPool.getId(),
        stakingRewardPool: targetStakingPool.getRewardTokenPool(),
        stakingRewardTokenMint: rewardTokenMint.getMintId(),
        stakingSubRewardPool: subRewardPool,
        stakingSubRewardTokenMint: subrewardMint,
        stakingProgram: DEVNET_STAKING_PROGRAM_ID,
        stakingProgamAuthority: stakingProgamAuthority,
        // We use mainnet pyth acct for the test suit
        // Because we copy the mainnet pyth acct over to the test validator
        stakingRewardOracle: PORT_USD_PYTH_PRICE_MAINNET,
        stakingSubRewardOracle: PORT_USD_PYTH_PRICE_MAINNET,
    };
}
