import Big from "big.js";

import {
    Cluster,
    Keypair,
    PublicKey,
    Signer,
    SystemProgram,
    TransactionInstruction,
    SYSVAR_CLOCK_PUBKEY,
    SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import { Token as SplToken, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import * as anchor from "@project-serum/anchor";

import {
    Amount,
    DEX_ID,
    JetClient,
    JetMarket,
    JetReserve,
    JetUser,
    JET_ID,
    JET_MARKET_ADDRESS,
    JET_MARKET_ADDRESS_DEVNET,
    ReserveConfig,
} from "@jet-lab/jet-engine";

import { CastleVault } from "../idl";
import { Vault } from "../types";
import { Rate, Token, TokenAmount } from "../utils";

import { LendingMarket } from "./asset";
import { getToken } from "./utils";

export interface JetAccounts {
    program: PublicKey;
    reserve: PublicKey;
    market: PublicKey;
    marketAuthority: PublicKey;
    feeNoteVault: PublicKey;
    depositNoteMint: PublicKey;
    liquiditySupply: PublicKey;
    pythPrice: PublicKey;
}

export class JetReserveAsset extends LendingMarket {
    private constructor(
        public provider: anchor.Provider,
        public accounts: JetAccounts,
        public market: JetMarket,
        public reserve: JetReserve,
        public reserveToken: Token,
        public lpToken: Token
    ) {
        super();
    }

    static async load(
        provider: anchor.Provider,
        cluster: Cluster,
        reserveMint: PublicKey
    ): Promise<JetReserveAsset> {
        let client: JetClient;
        let market: JetMarket;
        if (cluster == "devnet") {
            client = await JetClient.connect(provider, true);
            market = await JetMarket.load(client, JET_MARKET_ADDRESS_DEVNET);
        } else if (cluster == "mainnet-beta") {
            client = await JetClient.connect(provider, false);
            market = await JetMarket.load(client, JET_MARKET_ADDRESS);
        } else {
            throw new Error("Cluster ${cluster} not supported");
        }
        const reserves = await JetReserve.loadMultiple(client, market);
        const reserve = reserves.find((res) =>
            res.data.tokenMint.equals(reserveMint)
        );

        const accounts: JetAccounts = {
            program: JET_ID,
            reserve: reserve.data.address,
            market: market.address,
            marketAuthority: market.marketAuthority,
            feeNoteVault: reserve.data.feeNoteVault,
            depositNoteMint: reserve.data.depositNoteMint,
            liquiditySupply: reserve.data.vault,
            pythPrice: reserve.data.pythOraclePrice,
        };

        const lpToken = await getToken(
            provider.connection,
            reserve.data.depositNoteMint
        );
        const reserveToken = await getToken(
            provider.connection,
            reserve.data.tokenMint
        );

        return new JetReserveAsset(
            provider,
            accounts,
            market,
            reserve,
            reserveToken,
            lpToken
        );
    }

    /**
     * Creates a market, reserves, and adds initial liquidity
     *
     * TODO Split into create market adding reserves to it
     *
     * @param provider
     * @param owner
     * @param marketQuoteTokenMint
     * @param reserveSplToken
     * @param pythPrice
     * @param pythProduct
     * @param ownerReserveTokenAccount
     * @param initialReserveAmount
     * @returns
     */
    static async initialize(
        provider: anchor.Provider,
        wallet: anchor.Wallet,
        owner: Signer,
        marketQuoteTokenMint: PublicKey,
        reserveSplToken: SplToken,
        pythPrice: PublicKey,
        pythProduct: PublicKey,
        ownerReserveTokenAccount: PublicKey,
        initialReserveAmount: number
    ): Promise<JetReserveAsset> {
        const client = await JetClient.connect(provider, true);
        const market = await createLendingMarket(
            client,
            wallet,
            marketQuoteTokenMint
        );

        const accounts = await createReserve(
            wallet,
            client,
            market.address,
            marketQuoteTokenMint,
            reserveSplToken,
            TOKEN_PROGRAM_ID, // dummy dex market addr
            pythPrice,
            pythProduct
        );

        const reserve = await JetReserve.load(client, accounts.reserve);
        const jetUser = await JetUser.load(
            client,
            market,
            [reserve],
            owner.publicKey
        );
        const depositTx = await jetUser.makeDepositTx(
            reserve,
            ownerReserveTokenAccount,
            Amount.tokens(initialReserveAmount)
        );
        await provider.send(depositTx, [owner]);

        const lpToken = await getToken(
            provider.connection,
            reserve.data.depositNoteMint
        );
        const reserveToken = await getToken(
            provider.connection,
            reserve.data.tokenMint
        );

        return new JetReserveAsset(
            provider,
            accounts,
            market,
            reserve,
            reserveToken,
            lpToken
        );
    }

    async borrow(
        owner: Signer,
        reserveTokenAccount: PublicKey,
        amount: number
    ): Promise<string[]> {
        const jetUser = await JetUser.load(
            await JetClient.connect(this.provider, true),
            this.market,
            [this.reserve],
            owner.publicKey
        );
        const depositCollateralTx = await jetUser.makeDepositCollateralTx(
            this.reserve,
            Amount.tokens(amount * 1.5)
        );
        const borrowTx = await jetUser.makeBorrowTx(
            this.reserve,
            reserveTokenAccount,
            Amount.tokens(amount)
        );
        return await this.provider.sendAll([
            { tx: depositCollateralTx, signers: [owner] },
            { tx: borrowTx, signers: [owner] },
        ]);
    }

    async getLpTokenAccountValue(vaultState: Vault): Promise<TokenAmount> {
        await this.market.refresh();

        const reserveInfo = this.market.reserves[this.reserve.data.index];
        const exchangeRate = new Big(
            reserveInfo.depositNoteExchangeRate.toString()
        ).div(new Big(1e15));

        const lpToken = new SplToken(
            this.provider.connection,
            this.reserve.data.depositNoteMint,
            TOKEN_PROGRAM_ID,
            Keypair.generate() // dummy signer since we aren't making any txs
        );

        const lpTokenAccountInfo = await lpToken.getAccountInfo(
            vaultState.vaultJetLpToken
        );
        const lpTokenAmount = new Big(lpTokenAccountInfo.amount.toString());

        return TokenAmount.fromToken(
            this.reserveToken,
            exchangeRate.mul(lpTokenAmount).round(0, Big.roundDown)
        );
    }

    async getApy(): Promise<Rate> {
        await this.reserve.refresh();
        const apr = this.reserve.data.depositApy;
        const apy = Math.expm1(apr);
        return new Rate(Big(apy));
    }

    async getDepositedAmount(): Promise<TokenAmount> {
        await this.reserve.refresh();
        return TokenAmount.fromToken(
            this.reserveToken,
            Big(this.reserve.data.marketSize.lamports.toString())
        );
    }

    async getBorrowedAmount(): Promise<TokenAmount> {
        await this.reserve.refresh();
        const borrowed = this.reserve.data.marketSize.sub(
            this.reserve.data.availableLiquidity
        );
        return TokenAmount.fromToken(
            this.reserveToken,
            Big(borrowed.lamports.toString())
        );
    }

    getRefreshIx(
        program: anchor.Program<CastleVault>,
        vaultId: PublicKey,
        vaultState: Vault
    ): TransactionInstruction {
        return program.instruction.refreshJet({
            accounts: {
                vault: vaultId,
                vaultJetLpToken: vaultState.vaultJetLpToken,
                jetProgram: this.accounts.program,
                jetMarket: this.accounts.market,
                jetMarketAuthority: this.accounts.marketAuthority,
                jetReserve: this.accounts.reserve,
                jetFeeNoteVault: this.accounts.feeNoteVault,
                jetDepositNoteMint: this.accounts.depositNoteMint,
                jetPyth: this.accounts.pythPrice,
                tokenProgram: TOKEN_PROGRAM_ID,
                clock: SYSVAR_CLOCK_PUBKEY,
            },
        });
    }

    getReconcileIx(
        program: anchor.Program<CastleVault>,
        vaultId: PublicKey,
        vaultState: Vault,
        withdrawOption?: anchor.BN
    ): TransactionInstruction {
        return program.instruction.reconcileJet(
            withdrawOption == null ? new anchor.BN(0) : withdrawOption,
            {
                accounts: {
                    vault: vaultId,
                    vaultAuthority: vaultState.vaultAuthority,
                    vaultReserveToken: vaultState.vaultReserveToken,
                    vaultJetLpToken: vaultState.vaultJetLpToken,
                    jetProgram: this.accounts.program,
                    jetMarket: this.accounts.market,
                    jetMarketAuthority: this.accounts.marketAuthority,
                    jetReserve: this.accounts.reserve,
                    jetReserveToken: this.accounts.liquiditySupply,
                    jetLpMint: this.accounts.depositNoteMint,
                    tokenProgram: TOKEN_PROGRAM_ID,
                },
            }
        );
    }

    async getInitializeIx(
        program: anchor.Program<CastleVault>,
        vaultId: PublicKey,
        vaultAuthority: PublicKey,
        wallet: PublicKey,
        owner: PublicKey
    ): Promise<TransactionInstruction> {
        const [vaultJetLpTokenAccount, jetLpBump] =
            await PublicKey.findProgramAddress(
                [vaultId.toBuffer(), this.accounts.depositNoteMint.toBuffer()],
                program.programId
            );

        return program.instruction.initializeJet(jetLpBump, {
            accounts: {
                vault: vaultId,
                vaultAuthority: vaultAuthority,
                vaultJetLpToken: vaultJetLpTokenAccount,
                jetLpTokenMint: this.accounts.depositNoteMint,
                jetReserve: this.accounts.reserve,
                owner: owner,
                payer: wallet,
                tokenProgram: TOKEN_PROGRAM_ID,
                systemProgram: SystemProgram.programId,
                rent: SYSVAR_RENT_PUBKEY,
            },
        });
    }
}

async function createLendingMarket(
    client: JetClient,
    wallet: anchor.Wallet,
    quoteCurrencyMint: PublicKey
): Promise<JetMarket> {
    const account = Keypair.generate();

    await client.program.rpc.initMarket(
        wallet.publicKey,
        "USD",
        quoteCurrencyMint,
        {
            accounts: {
                market: account.publicKey,
            },
            signers: [account],
            instructions: [
                await client.program.account.market.createInstruction(account),
            ],
        }
    );

    return JetMarket.load(client, account.publicKey);
}

async function createReserve(
    wallet: anchor.Wallet,
    client: JetClient,
    market: PublicKey,
    quoteTokenMint: PublicKey,
    tokenMint: SplToken,
    dexMarket: PublicKey,
    pythPrice: PublicKey,
    pythProduct: PublicKey
): Promise<JetAccounts> {
    const reserve = Keypair.generate();
    const [depositNoteMint, depositNoteMintBump] = await findProgramAddress(
        client.program.programId,
        ["deposits", reserve, tokenMint]
    );
    const [loanNoteMint, loanNoteMintBump] = await findProgramAddress(
        client.program.programId,
        ["loans", reserve, tokenMint]
    );
    const [vault, vaultBump] = await findProgramAddress(
        client.program.programId,
        ["vault", reserve]
    );
    const [feeNoteVault, feeNoteVaultBump] = await findProgramAddress(
        client.program.programId,
        ["fee-vault", reserve]
    );
    const [dexSwapTokens, dexSwapTokensBump] = await findProgramAddress(
        client.program.programId,
        ["dex-swap-tokens", reserve]
    );
    const [dexOpenOrders, dexOpenOrdersBump] = await findProgramAddress(
        client.program.programId,
        ["dex-open-orders", reserve]
    );
    const [marketAuthority] = await findProgramAddress(
        client.program.programId,
        [market]
    );

    const reserveAccounts = {
        accounts: {
            reserve,
            vault,
            feeNoteVault,
            dexOpenOrders,
            dexSwapTokens,
            tokenMint,

            dexMarket,
            pythPrice,
            pythProduct,

            depositNoteMint,
            loanNoteMint,
        },

        bump: {
            vault: vaultBump,
            feeNoteVault: feeNoteVaultBump,
            dexOpenOrders: dexOpenOrdersBump,
            dexSwapTokens: dexSwapTokensBump,
            depositNoteMint: depositNoteMintBump,
            loanNoteMint: loanNoteMintBump,
        },
    };

    const reserveConfig: ReserveConfig = {
        utilizationRate1: 8500,
        utilizationRate2: 9500,
        borrowRate0: 50,
        borrowRate1: 600,
        borrowRate2: 4000,
        borrowRate3: 1600,
        minCollateralRatio: 12500,
        liquidationPremium: 300,
        manageFeeRate: 0,
        manageFeeCollectionThreshold: new anchor.BN(10),
        loanOriginationFee: 0,
        liquidationSlippage: 300,
        liquidationDexTradeMax: new anchor.BN(100),
        reserved0: 0,
        reserved1: Array(24).fill(0),
    };

    await client.program.rpc.initReserve(reserveAccounts.bump, reserveConfig, {
        accounts: toPublicKeys({
            market,
            marketAuthority,
            owner: wallet.publicKey,

            oracleProduct: reserveAccounts.accounts.pythProduct,
            oraclePrice: reserveAccounts.accounts.pythPrice,

            quoteTokenMint,

            tokenProgram: TOKEN_PROGRAM_ID,
            dexProgram: DEX_ID,
            clock: SYSVAR_CLOCK_PUBKEY,
            rent: SYSVAR_RENT_PUBKEY,
            systemProgram: SystemProgram.programId,

            ...reserveAccounts.accounts,
        }),
        signers: [reserveAccounts.accounts.reserve, wallet.payer],
        instructions: [
            await client.program.account.reserve.createInstruction(
                reserveAccounts.accounts.reserve
            ),
        ],
    });

    return {
        program: JET_ID,
        reserve: reserve.publicKey,
        market: market,
        marketAuthority: marketAuthority,
        feeNoteVault: feeNoteVault,
        depositNoteMint: depositNoteMint,
        liquiditySupply: vault,
        pythPrice: pythPrice,
    };
}

/**
 * Find a program derived address
 * @param programId The program the address is being derived for
 * @param seeds The seeds to find the address
 * @returns The address found and the bump seed required
 */
async function findProgramAddress(
    programId: PublicKey,
    seeds: (HasPublicKey | ToBytes | Uint8Array | string)[]
): Promise<[PublicKey, number]> {
    const seed_bytes = seeds.map((s) => {
        if (typeof s == "string") {
            return Buffer.from(s);
        } else if ("publicKey" in s) {
            return s.publicKey.toBytes();
        } else if ("toBytes" in s) {
            return s.toBytes();
        } else {
            return s;
        }
    });
    return await PublicKey.findProgramAddress(seed_bytes, programId);
}

interface ToBytes {
    toBytes(): Uint8Array;
}

interface HasPublicKey {
    publicKey: PublicKey;
}

/**
 * Convert some object of fields with address-like values,
 * such that the values are converted to their `PublicKey` form.
 * @param obj The object to convert
 */
function toPublicKeys(
    obj: Record<string, string | PublicKey | HasPublicKey | any>
): any {
    const newObj = {};

    for (const key in obj) {
        const value = obj[key];

        if (typeof value == "string") {
            newObj[key] = new PublicKey(value);
        } else if (typeof value == "object" && "publicKey" in value) {
            newObj[key] = value.publicKey;
        } else {
            newObj[key] = value;
        }
    }

    return newObj;
}
