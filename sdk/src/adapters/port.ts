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
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, Token as SplToken } from "@solana/spl-token";
import { ENV } from "@solana/spl-token-registry";
import * as anchor from "@project-serum/anchor";

import {
    AssetConfig,
    AssetDepositConfig,
    AssetDisplayConfig,
    AssetPrice,
    AssetPriceConfig,
    DEFAULT_PORT_LENDING_MARKET,
    Environment,
    initLendingMarketInstruction,
    initReserveInstruction,
    MintId,
    Port,
    PORT_STAKING,
    ReserveConfigProto,
    ReserveId,
} from "@port.finance/port-sdk";

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
        } else if (cluster == "mainnet-beta") {
            env = Environment.forMainNet();
            market = DEFAULT_PORT_LENDING_MARKET;
        } else {
            throw new Error("Cluster ${cluster} not supported");
        }
        const client = new Port(provider.connection, env, market);
        const reserveContext = await client.getReserveContext();
        const reserve = reserveContext.getByAssetMintId(MintId.of(reserveMint));
        const [authority, _] = await PublicKey.findProgramAddress(
            [market.toBuffer()],
            env.getLendingProgramPk()
        );
        const accounts: PortAccounts = {
            program: env.getLendingProgramPk(),
            market: market,
            marketAuthority: authority,
            reserve: reserve.getReserveId(),
            collateralMint: reserve.getShareMintId(),
            oracle: reserve.getOracleId(),
            liquiditySupply: reserve.getAssetBalanceId(),
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
        initialReserveAmount: number
    ): Promise<PortReserveAsset> {
        const market = await createLendingMarket(provider);
        const accounts = await createDefaultReserve(
            provider,
            initialReserveAmount,
            reserveTokenMint,
            ownerReserveTokenAccount,
            market.publicKey,
            pythPrice,
            owner
        );
        const env = new Environment(
            ENV.Devnet,
            DEVNET_LENDING_PROGRAM_ID,
            null,
            TOKEN_PROGRAM_ID,
            []
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

        return TokenAmount.fromToken(
            this.reserveToken,
            lpTokenAmount
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
        return program.methods.refreshPort()
        .accounts({
            vault: vaultId,
            vaultPortLpToken: vaultState.vaultPortLpToken,
            portProgram: this.accounts.program,
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
                ],
        )
        .instruction();
    }

    async getReconcileIx(
        program: anchor.Program<CastleVault>,
        vaultId: PublicKey,
        vaultState: Vault,
        withdrawOption?: anchor.BN
    ): Promise<TransactionInstruction> {
        return program.methods.reconcilePort(
            withdrawOption == null ? new anchor.BN(0) : withdrawOption,
        )
        .accounts({
            vault: vaultId,
            vaultAuthority: vaultState.vaultAuthority,
            vaultReserveToken: vaultState.vaultReserveToken,
            vaultPortLpToken: vaultState.vaultPortLpToken,
            portProgram: this.accounts.program,
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
        const [vaultPortLpTokenAccount, portLpBump] =
            await PublicKey.findProgramAddress(
                [vaultId.toBuffer(), this.accounts.collateralMint.toBuffer()],
                program.programId
            );

        return program.methods.initializePort(portLpBump)
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
}

const DEVNET_LENDING_PROGRAM_ID = new PublicKey(
    "pdQ2rQQU5zH2rDgZ7xH2azMBJegUzUyunJ5Jd637hC4"
);
const TOKEN_ACCOUNT_LEN = 165;
const TOKEN_MINT_LEN = 82;
const RESERVE_LEN = 575;
const LENDING_MARKET_LEN = 258;

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

async function createDefaultReserve(
    provider: anchor.AnchorProvider,
    initialLiquidity: number | anchor.BN,
    liquidityMint: PublicKey,
    sourceTokenWallet: PublicKey,
    lendingMarket: PublicKey,
    oracle: PublicKey,
    owner: Keypair
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
            DEFAULT_RESERVE_CONFIG,
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

    return {
        program: DEVNET_LENDING_PROGRAM_ID,
        market: lendingMarket,
        marketAuthority: lendingMarketAuthority,
        reserve: reserve.publicKey,
        oracle: oracle,
        collateralMint: collateralMintAccount.publicKey,
        liquiditySupply: liquiditySupplyTokenAccount.publicKey,
    };
}
