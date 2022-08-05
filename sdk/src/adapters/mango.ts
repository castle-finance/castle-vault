import { blob, struct, u8, Layout } from "buffer-layout";
import { toBigIntLE, toBufferLE } from "bigint-buffer";
import Big from "big.js";
import BN from "bn.js";
import {
    DexInstructions,
    Market,
    TokenInstructions,
} from "@project-serum/serum";

import {
    Cluster,
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
    Token as SplToken,
    MintLayout,
    AccountLayout,
    TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import * as anchor from "@project-serum/anchor";

import { CastleVault } from "../idl";
import { Vault } from "../types";
import { LendingMarket } from "./asset";
import { Rate, Token, TokenAmount } from "../utils";
import { getToken } from "./utils";
import {
    MangoAccount,
    MangoClient,
    MangoGroup,
    MangoCache,
    RootBank,
    StubOracleLayout,
    I80F48,
} from "@blockworks-foundation/mango-client";

async function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

const enableSuppressLogs = true;
let oldConsoleLog;
let oldConsoleError;

function suppressLogs() {
    if (enableSuppressLogs) {
        oldConsoleLog = console.log;
        oldConsoleError = console.error;
        console.log = function () {
            const _noop = "";
        };
        console.error = function () {
            const _noop = "";
        };
    }
}

function restoreLogs() {
    if (enableSuppressLogs) {
        console.log = oldConsoleLog;
        console.error = oldConsoleError;
    }
}

const BORROW_INDEX = 0;
const QUOTE_INDEX = 15;

const OPTIMAL_UTIL = 0.7;
const OPTIMAL_RATE = 0.06;
const MAX_RATE = 1.5;
const ZERO_BN = new BN(0);
const ZERO_KEY = new PublicKey(new Uint8Array(32));

export interface MangoAccounts {
    program: PublicKey;
    market: PublicKey;
    marketAuthority: PublicKey;
    reserve: PublicKey;
    pythPrice: PublicKey;
    switchboardFeed: PublicKey;
    collateralMint: PublicKey;
    liquiditySupply: PublicKey;
}

export class MangoReserveAsset extends LendingMarket {
    private constructor(
        public provider: anchor.AnchorProvider,
        public accounts: MangoAccounts,
        public client: MangoClient,
        public reserveToken: Token,
        public lpToken: Token
    ) {
        super();
    }

    static async load(
        provider: anchor.AnchorProvider,
        cluster: Cluster,
        reserveMint: PublicKey
    ): Promise<MangoReserveAsset> {
        return new MangoReserveAsset(provider, null, null, null, null);
    }

    static async initialize(
        provider: anchor.AnchorProvider,
        owner: Keypair,
        ownerReserveTokenAccount: PublicKey,
        reserveToken: SplToken,
        feesVault: PublicKey
    ): Promise<MangoReserveAsset> {
        // mango sdk functions console.log() in sendTransaction for some reason
        suppressLogs();

        // set up eth as non-quote token for cross collat mango group
        let ethToken = await SplToken.createMint(
            provider.connection,
            owner,
            owner.publicKey,
            null,
            2,
            TOKEN_PROGRAM_ID
        );

        // set up user to deposit eth for owner to borrow
        let user = Keypair.generate();
        let airdropSig = await provider.connection.requestAirdrop(
            user.publicKey,
            100000000000
        );
        await provider.connection.confirmTransaction(
            airdropSig,
            "singleGossip"
        );

        let userEthTokenAccount = await ethToken.createAccount(user.publicKey);
        await ethToken.mintTo(userEthTokenAccount, owner, [], 10000);

        // set up mango group
        let client: MangoClient;
        client = new MangoClient(provider.connection, DEVNET_MANGO_ID);

        const groupKey = await client.initMangoGroup(
            reserveToken.publicKey,
            ZERO_KEY,
            DEVNET_SERUM_ID,
            feesVault,
            5000,
            0.7,
            0.06,
            1.5,
            owner
        );
        let group = await client.getMangoGroup(groupKey);

        // set up user and owner mango accounts
        let ownerMangoPk = await client.initMangoAccount(group, owner);
        let ownerMangoAccount = await client.getMangoAccount(
            ownerMangoPk,
            DEVNET_SERUM_ID
        );

        let userMangoPk = await client.initMangoAccount(group, user);
        let userMangoAccount = await client.getMangoAccount(
            userMangoPk,
            DEVNET_SERUM_ID
        );

        // list and add serum market to mango group
        let ethSpotMarket = await listMarket(
            provider.connection,
            owner,
            ethToken.publicKey,
            reserveToken.publicKey,
            100,
            10,
            DEVNET_SERUM_ID
        );

        await addSpotMarketToMangoGroup(
            client,
            owner,
            group,
            ethToken,
            ethSpotMarket,
            2000
        );

        group = await client.getMangoGroup(groupKey);
        let cache = await group.loadCache(provider.connection);
        let rootBanks = await group.loadRootBanks(client.connection);
        let usdcRootBank = rootBanks[QUOTE_INDEX];
        let ethRootBank = rootBanks[BORROW_INDEX];
        let nodeBanks = await usdcRootBank.loadNodeBanks(client.connection);
        let filteredNodeBanks = nodeBanks.filter((nodeBank) => !!nodeBank);

        // owner deposits usdc
        await client.deposit(
            group,
            ownerMangoAccount,
            owner,
            group.tokens[QUOTE_INDEX].rootBank,
            usdcRootBank.nodeBanks[0],
            filteredNodeBanks[0]!.vault,
            ownerReserveTokenAccount,
            10000
        );

        group = await client.getMangoGroup(groupKey);
        rootBanks = await group.loadRootBanks(client.connection);
        usdcRootBank = rootBanks[QUOTE_INDEX];
        ethRootBank = rootBanks[BORROW_INDEX];
        nodeBanks = await ethRootBank.loadNodeBanks(client.connection);
        filteredNodeBanks = nodeBanks.filter((nodeBank) => !!nodeBank);

        // user deposits eth
        await client.deposit(
            group,
            userMangoAccount,
            user,
            group.tokens[BORROW_INDEX].rootBank,
            ethRootBank.nodeBanks[0],
            filteredNodeBanks[0]!.vault,
            userEthTokenAccount,
            100
        );

        group = await client.getMangoGroup(groupKey);
        cache = await group.loadCache(provider.connection);
        rootBanks = await group.loadRootBanks(client.connection);
        usdcRootBank = rootBanks[QUOTE_INDEX];
        ethRootBank = rootBanks[BORROW_INDEX];
        nodeBanks = await ethRootBank.loadNodeBanks(client.connection);
        filteredNodeBanks = nodeBanks.filter((nodeBank) => !!nodeBank);
        userMangoAccount = await client.getMangoAccount(
            userMangoPk,
            DEVNET_SERUM_ID
        );

        // cache banks and prices in preparation to borrow
        await cacheRootBanks(client, owner, group, [BORROW_INDEX, QUOTE_INDEX]);

        await cachePrices(client, owner, group, [BORROW_INDEX]);

        // owner borrows eth
        await client.withdraw(
            group,
            ownerMangoAccount,
            owner,
            group.tokens[BORROW_INDEX].rootBank,
            ethRootBank.nodeBanks[0],
            filteredNodeBanks[0]!.vault,
            1,
            true
        );

        group = await client.getMangoGroup(groupKey);
        cache = await group.loadCache(provider.connection);
        rootBanks = await group.loadRootBanks(client.connection);
        usdcRootBank = rootBanks[QUOTE_INDEX];
        ethRootBank = rootBanks[BORROW_INDEX];
        ownerMangoAccount = await client.getMangoAccount(
            ownerMangoPk,
            DEVNET_SERUM_ID
        );

        restoreLogs();

        logGroup(group);
        logAccount(ownerMangoAccount, group, cache, usdcRootBank, ethRootBank);

        return new MangoReserveAsset(provider, null, null, null, null);
    }

    async getLpTokenAccountValue(vaultState: Vault): Promise<TokenAmount> {
        return null;
    }

    /**
     *
     * @todo make this the same as program's calculation
     *
     * @returns
     */
    async getApy(): Promise<Rate> {
        return null;
    }

    async getBorrowedAmount(): Promise<TokenAmount> {
        return null;
    }

    async getDepositedAmount(): Promise<TokenAmount> {
        return null;
    }

    async getRefreshIx(
        program: anchor.Program<CastleVault>,
        vaultId: PublicKey,
        vaultState: Vault
    ): Promise<TransactionInstruction> {
        return null;
    }

    async getReconcileIx(
        program: anchor.Program<CastleVault>,
        vaultId: PublicKey,
        vaultState: Vault,
        withdrawOption?: anchor.BN
    ): Promise<TransactionInstruction> {
        return null;
    }

    async getInitializeIx(
        program: anchor.Program<CastleVault>,
        vaultId: PublicKey,
        vaultAuthority: PublicKey,
        wallet: PublicKey,
        owner: PublicKey
    ): Promise<TransactionInstruction> {
        return null;
    }
}

const DEVNET_MANGO_ID = new PublicKey(
    "4skJ85cdxQAFVKbcGgfun8iZPL7BadVYXG3kGEGkufqA"
);

const DEVNET_SERUM_ID = new PublicKey(
    "DESVgJVGajEgKGXhb6XmqDHGz3VjdgP7rEVESBgxmroY"
);

async function _sendTransaction(
    connection: Connection,
    transaction: Transaction,
    signers: Keypair[]
): Promise<TransactionSignature> {
    const signature = await connection.sendTransaction(transaction, signers);
    try {
        await connection.confirmTransaction(signature);
    } catch (e) {
        console.info("Error while confirming, trying again");
        await connection.confirmTransaction(signature);
    }
    return signature;
}

async function listMarket(
    connection: Connection,
    payer: Keypair,
    baseMint: PublicKey,
    quoteMint: PublicKey,
    baseLotSize: number,
    quoteLotSize: number,
    dexProgramId: PublicKey
): Promise<PublicKey> {
    const market = new Keypair();
    const requestQueue = new Keypair();
    const eventQueue = new Keypair();
    const bids = new Keypair();
    const asks = new Keypair();
    const baseVault = new Keypair();
    const quoteVault = new Keypair();
    const feeRateBps = 0;
    const quoteDustThreshold = new BN(100);

    async function getVaultOwnerAndNonce() {
        const nonce = ZERO_BN;
        // eslint-disable-next-line
        while (true) {
            try {
                const vaultOwner = await PublicKey.createProgramAddress(
                    [
                        market.publicKey.toBuffer(),
                        nonce.toArrayLike(Buffer, "le", 8),
                    ],
                    dexProgramId
                );
                return [vaultOwner, nonce];
            } catch (e) {
                nonce.iaddn(1);
            }
        }
    }
    const [vaultOwner, vaultSignerNonce] = await getVaultOwnerAndNonce();

    const tx1 = new Transaction();
    tx1.add(
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: baseVault.publicKey,
            lamports: await connection.getMinimumBalanceForRentExemption(165),
            space: 165,
            programId: TokenInstructions.TOKEN_PROGRAM_ID,
        }),
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: quoteVault.publicKey,
            lamports: await connection.getMinimumBalanceForRentExemption(165),
            space: 165,
            programId: TokenInstructions.TOKEN_PROGRAM_ID,
        }),
        TokenInstructions.initializeAccount({
            account: baseVault.publicKey,
            mint: baseMint,
            owner: vaultOwner,
        }),
        TokenInstructions.initializeAccount({
            account: quoteVault.publicKey,
            mint: quoteMint,
            owner: vaultOwner,
        })
    );

    const tx2 = new Transaction();
    tx2.add(
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: market.publicKey,
            lamports: await connection.getMinimumBalanceForRentExemption(
                Market.getLayout(dexProgramId).span
            ),
            space: Market.getLayout(dexProgramId).span,
            programId: dexProgramId,
        }),
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: requestQueue.publicKey,
            lamports: await connection.getMinimumBalanceForRentExemption(
                5120 + 12
            ),
            space: 5120 + 12,
            programId: dexProgramId,
        }),
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: eventQueue.publicKey,
            lamports: await connection.getMinimumBalanceForRentExemption(
                262144 + 12
            ),
            space: 262144 + 12,
            programId: dexProgramId,
        }),
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: bids.publicKey,
            lamports: await connection.getMinimumBalanceForRentExemption(
                65536 + 12
            ),
            space: 65536 + 12,
            programId: dexProgramId,
        }),
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: asks.publicKey,
            lamports: await connection.getMinimumBalanceForRentExemption(
                65536 + 12
            ),
            space: 65536 + 12,
            programId: dexProgramId,
        }),
        DexInstructions.initializeMarket({
            market: market.publicKey,
            requestQueue: requestQueue.publicKey,
            eventQueue: eventQueue.publicKey,
            bids: bids.publicKey,
            asks: asks.publicKey,
            baseVault: baseVault.publicKey,
            quoteVault: quoteVault.publicKey,
            baseMint,
            quoteMint,
            baseLotSize: new BN(baseLotSize),
            quoteLotSize: new BN(quoteLotSize),
            feeRateBps,
            vaultSignerNonce,
            quoteDustThreshold,
            programId: dexProgramId,
        })
    );
    await _sendTransaction(connection, tx1, [payer, baseVault, quoteVault]);
    await _sendTransaction(connection, tx2, [
        payer,
        market,
        requestQueue,
        eventQueue,
        bids,
        asks,
    ]);

    return market.publicKey;
}

async function createAccountInstruction(
    connection: Connection,
    payer: PublicKey,
    space: number,
    owner: PublicKey,
    lamports?: number
): Promise<{ account: Keypair; instruction: TransactionInstruction }> {
    const account = new Keypair();
    const instruction = SystemProgram.createAccount({
        fromPubkey: payer,
        newAccountPubkey: account.publicKey,
        lamports: lamports
            ? lamports
            : await connection.getMinimumBalanceForRentExemption(space),
        space,
        programId: owner,
    });

    return { account, instruction };
}

async function createOracle(
    connection: Connection,
    programId: PublicKey,
    payer: Keypair
): Promise<PublicKey> {
    const createOracleIns = await createAccountInstruction(
        connection,
        payer.publicKey,
        StubOracleLayout.span,
        programId
    );
    const tx = new Transaction();
    tx.add(createOracleIns.instruction);

    const signers = [payer, createOracleIns.account];
    const signerPks = signers.map((x) => x.publicKey);
    tx.setSigners(...signerPks);
    await _sendTransaction(connection, tx, signers);
    return createOracleIns.account.publicKey;
}

async function addSpotMarketToMangoGroup(
    client: MangoClient,
    payer: Keypair,
    mangoGroup: MangoGroup,
    mint: SplToken,
    spotMarketPk: PublicKey,
    initialPrice: number
): Promise<void> {
    const oraclePk = await createOracle(
        client.connection,
        DEVNET_MANGO_ID,
        payer
    );
    await client.addOracle(mangoGroup, oraclePk, payer);
    await client.setOracle(
        mangoGroup,
        oraclePk,
        payer,
        I80F48.fromNumber(initialPrice)
    );
    const initLeverage = 5;
    const maintLeverage = initLeverage * 2;
    const liquidationFee = 1 / (2 * maintLeverage);
    await client.addSpotMarket(
        mangoGroup,
        oraclePk,
        spotMarketPk,
        mint.publicKey,
        payer,
        maintLeverage,
        initLeverage,
        liquidationFee,
        OPTIMAL_UTIL,
        OPTIMAL_RATE,
        MAX_RATE
    );
}

async function cachePrices(
    client: MangoClient,
    payer: Keypair,
    mangoGroup: MangoGroup,
    oracleIndices: number[]
): Promise<void> {
    const pricesToCache: PublicKey[] = [];
    for (let oracleIndex of oracleIndices) {
        pricesToCache.push(mangoGroup.oracles[oracleIndex]);
    }
    await client.cachePrices(
        mangoGroup.publicKey,
        mangoGroup.mangoCache,
        pricesToCache,
        payer
    );
}

async function cacheRootBanks(
    client: MangoClient,
    payer: Keypair,
    mangoGroup: MangoGroup,
    rootBankIndices: number[]
): Promise<void> {
    const rootBanksToCache: PublicKey[] = [];
    for (let rootBankIndex of rootBankIndices) {
        rootBanksToCache.push(mangoGroup.tokens[rootBankIndex].rootBank);
    }
    await client.cacheRootBanks(
        mangoGroup.publicKey,
        mangoGroup.mangoCache,
        rootBanksToCache,
        payer
    );
}

function logGroup(group: MangoGroup) {
    console.log("Mango Group Info:");
    console.log(
        "- Quote Borrow Rate:",
        group.getBorrowRate(QUOTE_INDEX).toNumber()
    );
    console.log(
        "- Quote Deposit Rate:",
        group.getDepositRate(QUOTE_INDEX).toNumber()
    );
    console.log(
        "- Quote Total Borrow:",
        group.getUiTotalBorrow(QUOTE_INDEX).toNumber()
    );
    console.log(
        "- Quote Total Deposit:",
        group.getUiTotalDeposit(QUOTE_INDEX).toNumber()
    );
    console.log(
        "- ETH Borrow Rate:",
        group.getBorrowRate(BORROW_INDEX).toNumber()
    );
    console.log(
        "- ETH Deposit Rate:",
        group.getDepositRate(BORROW_INDEX).toNumber()
    );
    console.log(
        "- ETH Total Borrow:",
        group.getUiTotalBorrow(BORROW_INDEX).toNumber()
    );
    console.log(
        "- ETH Total Deposit:",
        group.getUiTotalDeposit(BORROW_INDEX).toNumber()
    );
}

function logAccount(
    account: MangoAccount,
    group: MangoGroup,
    cache: MangoCache,
    quoteRootBank: RootBank,
    ethRootBank: RootBank
) {
    console.log("Mango Account Info:");
    console.log(
        "- Assets Value:",
        account.getAssetsVal(group, cache).toNumber()
    );
    console.log(
        "- Collateral Value:",
        account.getCollateralValueUi(group, cache)
    );
    console.log(
        "- Quote Deposits:",
        account.getUiDeposit(quoteRootBank, group, QUOTE_INDEX).toNumber()
    );
    console.log(
        "- ETH Deposits:",
        account.getUiDeposit(ethRootBank, group, BORROW_INDEX).toNumber()
    );
    console.log(
        "- Quote Borrows:",
        account.getUiBorrow(quoteRootBank, group, QUOTE_INDEX).toNumber()
    );
    console.log(
        "- ETH Borrows:",
        account.getUiBorrow(ethRootBank, group, BORROW_INDEX).toNumber()
    );
}
