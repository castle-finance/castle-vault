import {
    initLendingMarketInstruction,
    initReserveInstruction,
} from '@port.finance/port-sdk';
import { ReserveConfig } from '@port.finance/port-sdk/src/structs/ReserveData';
import { BN, Provider } from '@project-serum/anchor';
import { getTokenAccount } from '@project-serum/common';
import { Token, TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { Keypair, PublicKey } from '@solana/web3.js';
import { Transaction } from '@solana/web3.js';
import { SystemProgram } from '@solana/web3.js';

export const TOKEN_ACCOUNT_LEN = 165;
export const TOKEN_MINT_LEN = 82;
export const RESERVE_LEN = 575;
export const LENDING_MARKET_LEN = 258;
export const STAKING_POOL_LEN = 298;
// TODO change to mainnet
export const PORT_LENDING = new PublicKey(
    'pdQ2rQQU5zH2rDgZ7xH2azMBJegUzUyunJ5Jd637hC4',
);

export const DEFAULT_RESERVE_CONFIG: ReserveConfig = {
    optimalUtilizationRate: 80,
    loanToValueRatio: 80,
    liquidationBonus: 5,
    liquidationThreshold: 85,
    minBorrowRate: 0,
    optimalBorrowRate: 40,
    maxBorrowRate: 90,
    fees: {
        borrowFeeWad: new BN(10000000000000),
        flashLoanFeeWad: new BN(30000000000000),
        hostFeePercentage: 0,
    },
    stakingPoolOption: 0,
    stakingPool: TOKEN_PROGRAM_ID, // dummy
};

export const createAccount = async (
    provider: Provider,
    space: number,
    owner: PublicKey,
): Promise<Keypair> => {
    const newAccount = Keypair.generate();
    const createTx = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: provider.wallet.publicKey,
            newAccountPubkey: newAccount.publicKey,
            programId: owner,
            lamports: await provider.connection.getMinimumBalanceForRentExemption(
                space,
            ),
            space,
        }),
    );
    await provider.send(createTx, [newAccount]);
    return newAccount;
};

export async function createLendingMarket(
    provider: Provider,
): Promise<Keypair> {
    const lendingMarket = await createAccount(
        provider,
        LENDING_MARKET_LEN,
        PORT_LENDING,
    );
    await provider.send(
        (() => {
            const tx = new Transaction();
            tx.add(
                initLendingMarketInstruction(
                    provider.wallet.publicKey,
                    Buffer.from(
                        'USD\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0',
                        'ascii',
                    ),
                    lendingMarket.publicKey,
                ),
            );
            return tx;
        })(),
        [],
    );
    return lendingMarket;
}

export interface ReserveState {
    address: PublicKey;
    liquiditySupplyPubkey: PublicKey;
    collateralMintAccount: PublicKey;
    collateralSupplyTokenAccount: PublicKey;
    liquidityFeeReceiver: PublicKey;
    userCollateralAccount: PublicKey;
    oracle: PublicKey;
}

export async function createDefaultReserve(
    provider: Provider,
    initialLiquidity: number | BN,
    sourceTokenWallet: PublicKey,
    lendingMarket: PublicKey,
    oracle: PublicKey,
    owner: Keypair,
    config: ReserveConfig,
): Promise<ReserveState> {
    const reserve = await createAccount(provider, RESERVE_LEN, PORT_LENDING);

    const collateralMintAccount = await createAccount(
        provider,
        TOKEN_MINT_LEN,
        TOKEN_PROGRAM_ID,
    );

    const liquiditySupplyTokenAccount = await createAccount(
        provider,
        TOKEN_ACCOUNT_LEN,
        TOKEN_PROGRAM_ID,
    );

    const collateralSupplyTokenAccount = await createAccount(
        provider,
        TOKEN_ACCOUNT_LEN,
        TOKEN_PROGRAM_ID,
    );

    const userCollateralTokenAccount = await createAccount(
        provider,
        TOKEN_ACCOUNT_LEN,
        TOKEN_PROGRAM_ID,
    );

    const liquidityFeeReceiver = await createAccount(
        provider,
        TOKEN_ACCOUNT_LEN,
        TOKEN_PROGRAM_ID,
    );

    const [lendingMarketAuthority] = await PublicKey.findProgramAddress(
        [lendingMarket.toBuffer()],
        PORT_LENDING,
    );

    const tokenAccount = await getTokenAccount(provider, sourceTokenWallet);

    const tx = new Transaction();

    tx.add(
        Token.createApproveInstruction(
            TOKEN_PROGRAM_ID,
            sourceTokenWallet,
            provider.wallet.publicKey,
            owner.publicKey,
            [],
            initialLiquidity,
        )
    );
    tx.add(
        initReserveInstruction(
            initialLiquidity,
            1,
            new BN('100000000000000000000000'),
            config,
            sourceTokenWallet,
            userCollateralTokenAccount.publicKey,
            reserve.publicKey,
            tokenAccount.mint,
            liquiditySupplyTokenAccount.publicKey,
            liquidityFeeReceiver.publicKey,
            oracle,
            collateralMintAccount.publicKey,
            collateralSupplyTokenAccount.publicKey,
            lendingMarket,
            lendingMarketAuthority,
            provider.wallet.publicKey,
            provider.wallet.publicKey,
        ),
    );

    await provider.send(tx, [owner]);

    return {
        address: reserve.publicKey,
        liquiditySupplyPubkey: liquiditySupplyTokenAccount.publicKey,
        collateralMintAccount: collateralMintAccount.publicKey,
        collateralSupplyTokenAccount: collateralSupplyTokenAccount.publicKey,
        liquidityFeeReceiver: liquidityFeeReceiver.publicKey,
        userCollateralAccount: userCollateralTokenAccount.publicKey,
        oracle: oracle,
    };
}
