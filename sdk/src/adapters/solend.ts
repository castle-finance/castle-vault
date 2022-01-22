import {
  Token,
  MintLayout,
  AccountLayout,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_CLOCK_PUBKEY,
  SYSVAR_RENT_PUBKEY,
  TransactionInstruction,
} from "@solana/web3.js";
import * as anchor from "@project-serum/anchor";
import { blob, struct, u8, Layout } from "buffer-layout";
import { toBigIntLE, toBufferLE } from "bigint-buffer";
import {
  LENDING_MARKET_SIZE,
  //RESERVE_SIZE
} from "@solendprotocol/solend-sdk";

import { Asset } from "./asset";

export interface SolendAccounts {
  program: PublicKey;
  market: PublicKey;
  marketAuthority: PublicKey;
  reserve: PublicKey;
  pythPrice: PublicKey;
  switchboardFeed: PublicKey;
  collateralMint: PublicKey;
  liquiditySupply: PublicKey;
}

export class SolendReserveAsset extends Asset {
  provider: anchor.Provider;
  accounts: SolendAccounts;

  private constructor(provider: anchor.Provider, accounts: SolendAccounts) {
    super();
    this.provider = provider;
    this.accounts = accounts;
  }

  static async initialize(
    provider: anchor.Provider,
    owner: Keypair,
    wallet: anchor.Wallet,
    reserveTokenMint: PublicKey,
    pythProgram: PublicKey,
    switchboardProgram: PublicKey,
    pythProduct: PublicKey,
    pythPrice: PublicKey,
    switchboardFeed: PublicKey,
    ownerReserveTokenAccount: PublicKey,
    initialReserveAmount: number
  ): Promise<SolendReserveAsset> {
    const market = await initLendingMarket(
      provider,
      owner.publicKey,
      wallet.payer,
      pythProgram,
      switchboardProgram
    );
    const accounts = await addReserve(
      provider,
      initialReserveAmount,
      ownerReserveTokenAccount,
      owner,
      wallet.payer,
      reserveTokenMint,
      pythProduct,
      pythPrice,
      switchboardFeed,
      market.publicKey
    );

    return new SolendReserveAsset(provider, accounts);
  }

  async getLpTokenAccountValue(address: PublicKey): Promise<number> {
    throw new Error("Method not implemented.");
  }
  async getApy(): Promise<number> {
    throw new Error("Method not implemented.");
  }
}

const DEVNET_PROGRAM_ID = new PublicKey(
  "ALend7Ketfx5bxh6ghsCDXAoDrhvEmsXT3cynB6aPLgx"
);

export async function initLendingMarket(
  provider: anchor.Provider,
  owner: PublicKey,
  payer: Keypair,
  pythProgramId: PublicKey,
  switchboardProgramId: PublicKey
): Promise<Keypair> {
  const lendingMarketAccount = anchor.web3.Keypair.generate();
  const balanceNeeded =
    await provider.connection.getMinimumBalanceForRentExemption(
      LENDING_MARKET_SIZE
    );

  const initTx = new anchor.web3.Transaction()
    .add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: lendingMarketAccount.publicKey,
        lamports: balanceNeeded,
        space: LENDING_MARKET_SIZE,
        programId: DEVNET_PROGRAM_ID,
      })
    )
    .add(
      initLendingMarketInstruction(
        owner,
        quoteCurrency("USD"),
        lendingMarketAccount.publicKey,
        pythProgramId,
        switchboardProgramId,
        DEVNET_PROGRAM_ID
      )
    );
  await provider.send(initTx, [payer, lendingMarketAccount]);
  return lendingMarketAccount;
}

export async function addReserve(
  provider: anchor.Provider,
  liquidityAmount: number,
  ownerReserveTokenAccount: PublicKey,
  owner: Keypair,
  payer: Keypair,
  reserveTokenMint: PublicKey,
  pythProduct: PublicKey,
  pythPrice: PublicKey,
  switchboardFeed: PublicKey,
  lendingMarket: PublicKey
): Promise<SolendAccounts> {
  const RESERVE_SIZE = 619;
  const collateralMint = anchor.web3.Keypair.generate();
  const [lendingMarketAuthority] = await PublicKey.findProgramAddress(
    [lendingMarket.toBuffer()],
    DEVNET_PROGRAM_ID
  );

  const reserve = anchor.web3.Keypair.generate();
  const liquiditySupply = anchor.web3.Keypair.generate();
  const collateralSupply = anchor.web3.Keypair.generate();
  const liquidityFeeReceiver = anchor.web3.Keypair.generate();
  const userCollateral = anchor.web3.Keypair.generate();
  const userTransferAuthority = anchor.web3.Keypair.generate();

  const reserveBalance =
    await provider.connection.getMinimumBalanceForRentExemption(RESERVE_SIZE);
  const mintBalance =
    await provider.connection.getMinimumBalanceForRentExemption(
      MintLayout.span
    );
  const accountBalance =
    await provider.connection.getMinimumBalanceForRentExemption(
      AccountLayout.span
    );

  const tx1 = new anchor.web3.Transaction()
    .add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: reserve.publicKey,
        lamports: reserveBalance,
        space: RESERVE_SIZE,
        programId: DEVNET_PROGRAM_ID,
      })
    )
    .add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: collateralMint.publicKey,
        lamports: mintBalance,
        space: MintLayout.span,
        programId: TOKEN_PROGRAM_ID,
      })
    )
    .add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: collateralSupply.publicKey,
        lamports: accountBalance,
        space: AccountLayout.span,
        programId: TOKEN_PROGRAM_ID,
      })
    )
    .add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: userCollateral.publicKey,
        lamports: accountBalance,
        space: AccountLayout.span,
        programId: TOKEN_PROGRAM_ID,
      })
    );
  const tx2 = new anchor.web3.Transaction()
    .add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: liquiditySupply.publicKey,
        lamports: accountBalance,
        space: AccountLayout.span,
        programId: TOKEN_PROGRAM_ID,
      })
    )
    .add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: liquidityFeeReceiver.publicKey,
        lamports: accountBalance,
        space: AccountLayout.span,
        programId: TOKEN_PROGRAM_ID,
      })
    );

  const reserveConfig = {
    optimalUtilizationRate: 80,
    loanToValueRatio: 50,
    liquidationBonus: 5,
    liquidationThreshold: 55,
    minBorrowRate: 0,
    optimalBorrowRate: 4,
    maxBorrowRate: 30,
    fees: {
      /// 0.00001% (Aave borrow fee)
      borrowFeeWad: BigInt(100_000_000_000),
      /// 0.3% (Aave flash loan fee)
      flashLoanFeeWad: BigInt(3_000_000_000_000_000),
      hostFeePercentage: 20,
    },
    depositLimit: BigInt(100_000_000),
    borrowLimit: BigInt(100_000_000),
    feeReceiver: liquidityFeeReceiver.publicKey,
  };

  const tx3 = new anchor.web3.Transaction()
    .add(
      Token.createApproveInstruction(
        TOKEN_PROGRAM_ID,
        ownerReserveTokenAccount,
        userTransferAuthority.publicKey,
        owner.publicKey,
        [],
        liquidityAmount
      )
    )
    .add(
      initReserveInstruction(
        liquidityAmount,
        reserveConfig,
        ownerReserveTokenAccount,
        userCollateral.publicKey,
        reserve.publicKey,
        reserveTokenMint,
        liquiditySupply.publicKey,
        liquidityFeeReceiver.publicKey,
        pythProduct,
        pythPrice,
        collateralMint.publicKey,
        collateralSupply.publicKey,
        lendingMarket,
        lendingMarketAuthority,
        owner.publicKey,
        userTransferAuthority.publicKey,
        switchboardFeed,
        DEVNET_PROGRAM_ID
      )
    );
  await provider.sendAll([
    {
      tx: tx1,
      signers: [
        payer,
        reserve,
        collateralMint,
        collateralSupply,
        userCollateral,
      ],
    },
    { tx: tx2, signers: [payer, liquiditySupply, liquidityFeeReceiver] },
    { tx: tx3, signers: [payer, owner, userTransferAuthority] },
  ]);
  return {
    program: DEVNET_PROGRAM_ID,
    reserve: reserve.publicKey,
    pythPrice: pythPrice,
    switchboardFeed: switchboardFeed,
    collateralMint: collateralMint.publicKey,
    liquiditySupply: liquiditySupply.publicKey,
    market: lendingMarket,
    marketAuthority: lendingMarketAuthority,
  };
}

const quoteCurrency = (s: string) => {
  const buf = Buffer.alloc(32);
  const strBuf = Buffer.from(s);
  strBuf.copy(buf, 0);
  return buf;
};

// TODO delete this and import from solend sdk
// Waiting for instructions to be merged

interface EncodeDecode<T> {
  decode: (buffer: Buffer, offset?: number) => T;
  encode: (src: T, buffer: Buffer, offset?: number) => number;
}

const encodeDecode = <T>(layout: Layout<T>): EncodeDecode<T> => {
  const decode = layout.decode.bind(layout);
  const encode = layout.encode.bind(layout);
  return { decode, encode };
};

const publicKey = (property = "publicKey"): Layout<PublicKey> => {
  const layout = blob(32, property);
  const { encode, decode } = encodeDecode(layout);

  const publicKeyLayout = layout as Layout<unknown> as Layout<PublicKey>;

  publicKeyLayout.decode = (buffer: Buffer, offset: number) => {
    const src = decode(buffer, offset);
    return new PublicKey(src);
  };

  publicKeyLayout.encode = (
    publicKey: PublicKey,
    buffer: Buffer,
    offset: number
  ) => {
    const src = publicKey.toBuffer();
    return encode(src, buffer, offset);
  };

  return publicKeyLayout;
};

const initLendingMarketInstruction = (
  owner: PublicKey,
  quoteCurrency: Buffer,
  lendingMarket: PublicKey,
  pythOracle: PublicKey,
  switchboardOracle: PublicKey,
  lendingProgram: PublicKey
): TransactionInstruction => {
  interface Data {
    instruction: number;
    owner: PublicKey;
    quoteCurrency: Buffer;
  }
  const DataLayout = struct<Data>([
    u8("instruction"),
    publicKey("owner"),
    blob(32, "quoteCurrency"),
  ]);
  const data = Buffer.alloc(DataLayout.span);
  DataLayout.encode(
    {
      instruction: 0,
      owner,
      quoteCurrency,
    },
    data
  );

  const keys = [
    { pubkey: lendingMarket, isSigner: false, isWritable: true },
    { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    { pubkey: pythOracle, isSigner: false, isWritable: false },
    { pubkey: switchboardOracle, isSigner: false, isWritable: false },
  ];

  return new TransactionInstruction({
    keys,
    programId: lendingProgram,
    data,
  });
};

interface ReserveFees {
  borrowFeeWad: bigint;
  flashLoanFeeWad: bigint;
  hostFeePercentage: number;
}

const bigInt =
  (length: number) =>
  (property = "bigInt"): Layout<bigint> => {
    const layout = blob(length, property);
    const { encode, decode } = encodeDecode(layout);

    const bigIntLayout = layout as Layout<unknown> as Layout<bigint>;

    bigIntLayout.decode = (buffer: Buffer, offset: number) => {
      const src = decode(buffer, offset);
      return toBigIntLE(src as Buffer);
    };

    bigIntLayout.encode = (bigInt: bigint, buffer: Buffer, offset: number) => {
      const src = toBufferLE(bigInt, length);
      return encode(src, buffer, offset);
    };

    return bigIntLayout;
  };

const u64 = bigInt(8);

const ReserveFeesLayout = struct<ReserveFees>(
  [u64("borrowFeeWad"), u64("flashLoanFeeWad"), u8("hostFeePercentage")],
  "fees"
);

interface ReserveConfig {
  optimalUtilizationRate: number;
  loanToValueRatio: number;
  liquidationBonus: number;
  liquidationThreshold: number;
  minBorrowRate: number;
  optimalBorrowRate: number;
  maxBorrowRate: number;
  fees: ReserveFees;
  depositLimit: bigint;
  borrowLimit: bigint;
  feeReceiver: PublicKey;
}

const ReserveConfigLayout = struct<ReserveConfig>(
  [
    u8("optimalUtilizationRate"),
    u8("loanToValueRatio"),
    u8("liquidationBonus"),
    u8("liquidationThreshold"),
    u8("minBorrowRate"),
    u8("optimalBorrowRate"),
    u8("maxBorrowRate"),
    ReserveFeesLayout,
    u64("depositLimit"),
    u64("borrowLimit"),
    publicKey("feeReceiver"),
  ],
  "config"
);

const initReserveInstruction = (
  liquidityAmount: number | bigint,
  config: ReserveConfig,
  sourceLiquidity: PublicKey,
  destinationCollateral: PublicKey,
  reserve: PublicKey,
  liquidityMint: PublicKey,
  liquiditySupply: PublicKey,
  liquidityFeeReceiver: PublicKey,
  pythProduct: PublicKey,
  pythPrice: PublicKey,
  collateralMint: PublicKey,
  collateralSupply: PublicKey,
  lendingMarket: PublicKey,
  lendingMarketAuthority: PublicKey,
  lendingMarketOwner: PublicKey,
  transferAuthority: PublicKey,
  switchboardFeed: PublicKey,
  lendingProgram: PublicKey
): TransactionInstruction => {
  interface Data {
    instruction: number;
    liquidityAmount: bigint;
    config: ReserveConfig;
  }

  const DataLayout = struct<Data>([
    u8("instruction"),
    u64("liquidityAmount"),
    ReserveConfigLayout,
  ]);
  const data = Buffer.alloc(DataLayout.span);
  DataLayout.encode(
    {
      instruction: 2,
      liquidityAmount: BigInt(liquidityAmount),
      config,
    },
    data
  );

  const keys = [
    { pubkey: sourceLiquidity, isSigner: false, isWritable: true },
    { pubkey: destinationCollateral, isSigner: false, isWritable: true },
    { pubkey: reserve, isSigner: false, isWritable: true },
    { pubkey: liquidityMint, isSigner: false, isWritable: false },
    { pubkey: liquiditySupply, isSigner: false, isWritable: true },
    { pubkey: liquidityFeeReceiver, isSigner: false, isWritable: true },
    { pubkey: collateralMint, isSigner: false, isWritable: true },
    { pubkey: collateralSupply, isSigner: false, isWritable: true },
    { pubkey: pythProduct, isSigner: false, isWritable: false },
    { pubkey: pythPrice, isSigner: false, isWritable: false },
    { pubkey: switchboardFeed, isSigner: false, isWritable: false },
    { pubkey: lendingMarket, isSigner: false, isWritable: true },
    { pubkey: lendingMarketAuthority, isSigner: false, isWritable: false },
    { pubkey: lendingMarketOwner, isSigner: true, isWritable: false },
    { pubkey: transferAuthority, isSigner: true, isWritable: false },
    { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
    { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
  ];

  return new TransactionInstruction({
    keys,
    programId: lendingProgram,
    data,
  });
};
