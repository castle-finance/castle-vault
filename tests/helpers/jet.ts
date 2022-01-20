import {
  DEX_ID,
  Jet,
  JetClient,
  JetMarket,
  JET_ID,
  ReserveConfig,
} from "@jet-lab/jet-engine";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_CLOCK_PUBKEY,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import { BN, Provider } from "@project-serum/anchor";
import { Token, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { JetAccounts } from "../../sdk/src/types";

export async function createLendingMarket(
  client: JetClient,
  owner: PublicKey,
  quoteCurrencyMint: PublicKey
): Promise<JetMarket> {
  const account = Keypair.generate();

  await client.program.rpc.initMarket(owner, "USD", quoteCurrencyMint, {
    accounts: {
      market: account.publicKey,
    },
    signers: [account],
    instructions: [await client.program.account.market.createInstruction(account)],
  });

  return JetMarket.load(client, account.publicKey);
}

export async function initReserve(
  client: JetClient,
  market: PublicKey,
  marketOwner: PublicKey,
  quoteTokenMint: PublicKey,
  tokenMint: Token,
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
  const [vault, vaultBump] = await findProgramAddress(client.program.programId, [
    "vault",
    reserve,
  ]);
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
  const [marketAuthority] = await findProgramAddress(client.program.programId, [
    market,
  ]);

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
    manageFeeCollectionThreshold: new BN(10),
    loanOriginationFee: 0,
    liquidationSlippage: 300,
    liquidationDexTradeMax: new BN(100),
    reserved0: 0,
    reserved1: Array(24).fill(0),
  };

  await client.program.rpc.initReserve(reserveAccounts.bump, reserveConfig, {
    accounts: toPublicKeys({
      market,
      marketAuthority,
      owner: marketOwner,

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
    signers: [reserveAccounts.accounts.reserve],
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

export async function getMarketAuthority(
  market: PublicKey,
  programId: PublicKey = JET_ID
): Promise<PublicKey> {
  const [marketAuthority] = await findProgramAddress(programId, [market]);
  return marketAuthority;
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
