import {
    Cluster,
    Keypair,
    PublicKey,
    SystemProgram,
    Transaction,
    TransactionInstruction,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, Token } from "@solana/spl-token";
import * as BufferLayout from "@solana/buffer-layout";
import * as anchor from "@project-serum/anchor";
import { OrcaPoolConfig } from "@orca-so/sdk";
import { orcaPoolConfigs } from "@orca-so/sdk/dist/constants/pools";
import { OrcaPoolParams } from "@orca-so/sdk/dist/model/orca/pool/pool-types";

export interface OrcaLegacyAccounts {
    marketId: number;
    programId: PublicKey;
    swapProgram: PublicKey;
    swapAuthority: PublicKey;
    poolTokenMint: PublicKey;
    feeAccount: PublicKey;
    tokenAccountA: PublicKey;
    tokenAccountB: PublicKey;
    vaultOrcaLegacyAccount?: PublicKey;
}

export class OrcaLegacySwap {
    private constructor(public accounts: OrcaLegacyAccounts) {}

    static load(
        tokenA: PublicKey,
        tokenB: PublicKey,
        cluster: Cluster
    ): OrcaLegacySwap {
        const tokenPairSig = tokenA.toString() + tokenB.toString();
        let tokenPairToOrcaLegacyPool;

        if (cluster == "devnet") {
            // TODO mock orca pool
        } else if (cluster == "mainnet-beta") {
            // Load orca pool parameters from the sdk for look-up.
            // Because the sdk doesn't support look-up using token mint pubkeys.
            tokenPairToOrcaLegacyPool = Object.fromEntries(
                Object.values(OrcaPoolConfig).map((v) => {
                    const params = orcaPoolConfigs[v];
                    const tokens = Object.keys(params.tokens);
                    return [
                        tokens[0].toString() + tokens[1].toString(),
                        params,
                    ];
                })
            );
        } else {
            throw new Error("Cluster ${cluster} not supported");
        }

        const params: OrcaPoolParams = tokenPairToOrcaLegacyPool[tokenPairSig];
        if (params == undefined) {
            throw new Error("Token pair not supported");
        }

        // TODO marketId to trading pair mapping
        const marketId = 0;

        const accounts = {
            marketId: marketId,
            programId: DEVNET_ORCA_TOKEN_SWAP_ID,
            swapProgram: params.address,
            swapAuthority: params.authority,
            poolTokenMint: params.poolTokenMint,
            feeAccount: params.feeAccount,
            tokenAccountA: tokenA,
            tokenAccountB: tokenB,
        };
        return new OrcaLegacySwap(accounts);
    }

    // This is used ONLY to create a mock orca swap for testing
    static async initialize(
        provider: anchor.AnchorProvider,
        owner: Keypair, // owner of the pool
        tokenA: Token, // mint of token A
        tokenB: Token, // mint of token B
        tokenOwnerA: Keypair, // acct that can mint token A
        tokenOwnerB: Keypair // acct that can mint token B
    ): Promise<OrcaLegacySwap> {
        const accounts = await createMockSwap(
            provider,
            owner,
            tokenA,
            tokenB,
            tokenOwnerA,
            tokenOwnerB
        );
        return new OrcaLegacySwap(accounts);
    }
}

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

// constants: https://github.com/orca-so/typescript-sdk/blob/main/src/public/utils/constants.ts
const DEVNET_ORCA_TOKEN_SWAP_ID = new PublicKey(
    "3xQ8SWv2GaFXXpHZNqkXsdxq5DZciHBz6ZFoPPfbFd7U"
);
const ORCA_TOKEN_SWAP_ACCOUNT_LEN = 324;

interface InitOrcaSwapIxData {
    instruction: number;
    nonce: number;
    tradeFeeNumerator: number;
    tradeFeeDenominator: number;
    ownerTradeFeeNumerator: number;
    ownerTradeFeeDenominator: number;
    ownerWithdrawFeeNumerator: number;
    ownerWithdrawFeeDenominator: number;
    hostFeeNumerator: number;
    hostFeeDenominator: number;
    curveType: number;
    curveParameters: Uint8Array;
}

const initOrcaSwapIxDataLayout = BufferLayout.struct<InitOrcaSwapIxData>([
    BufferLayout.u8("instruction"),
    BufferLayout.u8("nonce"),
    BufferLayout.nu64("tradeFeeNumerator"),
    BufferLayout.nu64("tradeFeeDenominator"),
    BufferLayout.nu64("ownerTradeFeeNumerator"),
    BufferLayout.nu64("ownerTradeFeeDenominator"),
    BufferLayout.nu64("ownerWithdrawFeeNumerator"),
    BufferLayout.nu64("ownerWithdrawFeeDenominator"),
    BufferLayout.nu64("hostFeeNumerator"),
    BufferLayout.nu64("hostFeeDenominator"),
    BufferLayout.u8("curveType"),
    BufferLayout.blob(32, "curveParameters"),
]);

async function createMockSwap(
    provider: anchor.AnchorProvider,
    owner: Keypair, // owner of the pool
    tokenA: Token, // mint of token A
    tokenB: Token, // mint of token B
    tokenOwnerA: Keypair, // acct that can mint token A
    tokenOwnerB: Keypair // acct that can mint token B
): Promise<OrcaLegacyAccounts> {
    // This step will create the swap program account, which is used to store states of the pool
    const swapProgram = await createAccount(
        provider,
        ORCA_TOKEN_SWAP_ACCOUNT_LEN,
        DEVNET_ORCA_TOKEN_SWAP_ID
    );
    const [authority, bump] = await PublicKey.findProgramAddress(
        [swapProgram.publicKey.toBuffer()],
        DEVNET_ORCA_TOKEN_SWAP_ID
    );

    // This step will create the pool's native LP token
    const poolTokenMint = await Token.createMint(
        provider.connection,
        owner,
        authority,
        null,
        6,
        TOKEN_PROGRAM_ID
    );

    // This step will create the pool's fee account
    const feeAccount = await poolTokenMint.createAssociatedTokenAccount(
        poolTokenMint.publicKey
    );

    // This step will mint some mock tokens to be swaped for testing purposes
    const tokenSupplyA = (
        await tokenA.getOrCreateAssociatedAccountInfo(tokenOwnerA.publicKey)
    ).address;
    await tokenA.mintTo(tokenSupplyA, tokenOwnerA, [], 1000000000);

    const tokenSupplyB = (
        await tokenB.getOrCreateAssociatedAccountInfo(tokenOwnerB.publicKey)
    ).address;
    await tokenB.mintTo(tokenSupplyB, tokenOwnerB, [], 2000000000);

    // This step will transfer the ownership of token A & B accounts to the pool.
    await tokenA.setAuthority(
        tokenSupplyA,
        authority,
        "AccountOwner",
        tokenOwnerA.publicKey,
        []
    );
    await tokenB.setAuthority(
        tokenSupplyB,
        authority,
        "AccountOwner",
        tokenOwnerB.publicKey,
        []
    );

    // define trading fees
    const tradeFeeNumerator = 25;
    const tradeFeeDenominator = 10000;
    const ownerFeeNumerator = 5;
    const ownerFeeDenominator = 10000;

    const ixData = Buffer.alloc(99);
    initOrcaSwapIxDataLayout.encode(
        {
            instruction: 0x00, // Init
            nonce: bump,
            tradeFeeNumerator: tradeFeeNumerator,
            tradeFeeDenominator: tradeFeeDenominator,
            ownerTradeFeeNumerator: ownerFeeNumerator,
            ownerTradeFeeDenominator: ownerFeeDenominator,
            ownerWithdrawFeeNumerator: 0,
            ownerWithdrawFeeDenominator: 0,
            hostFeeNumerator: 0,
            hostFeeDenominator: 0,
            curveType: 0,
            curveParameters: Buffer.alloc(32),
        },
        ixData
    );

    const keys = [
        { pubkey: swapProgram.publicKey, isSigner: false, isWritable: true },
        { pubkey: authority, isSigner: false, isWritable: false },
        { pubkey: tokenSupplyA, isSigner: false, isWritable: false },
        { pubkey: tokenSupplyB, isSigner: false, isWritable: false },
        { pubkey: poolTokenMint.publicKey, isSigner: false, isWritable: true },
        { pubkey: feeAccount, isSigner: false, isWritable: true },
        { pubkey: feeAccount, isSigner: false, isWritable: true },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ];

    const tx = new Transaction().add(
        new TransactionInstruction({
            keys,
            programId: DEVNET_ORCA_TOKEN_SWAP_ID,
            data: ixData,
        })
    );

    await provider.sendAndConfirm(tx, [owner]);

    const orcaAccounts: OrcaLegacyAccounts = {
        marketId: 0,
        programId: DEVNET_ORCA_TOKEN_SWAP_ID,
        swapProgram: swapProgram.publicKey,
        swapAuthority: authority,
        poolTokenMint: poolTokenMint.publicKey,
        feeAccount: feeAccount,
        tokenAccountA: tokenSupplyA,
        tokenAccountB: tokenSupplyB,
    };
    return orcaAccounts;
}
