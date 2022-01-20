import * as solend from "@dbricks/dbricks-solend";
import * as anchor from "@project-serum/anchor";
import { AccountLayout, MintLayout, Token, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";

const PROGRAM_ID = new PublicKey("ALend7Ketfx5bxh6ghsCDXAoDrhvEmsXT3cynB6aPLgx");

export async function initLendingMarket(
  provider: anchor.Provider,
  owner: PublicKey,
  payer: Keypair,
  pythProgramId: PublicKey,
  switchboardProgramId: PublicKey
) {
  const lendingMarketAccount = anchor.web3.Keypair.generate();
  const balanceNeeded = await provider.connection.getMinimumBalanceForRentExemption(
    solend.LENDING_MARKET_SIZE
  );

  const initTx = new anchor.web3.Transaction()
    .add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: lendingMarketAccount.publicKey,
        lamports: balanceNeeded,
        space: solend.LENDING_MARKET_SIZE,
        programId: PROGRAM_ID,
      })
    )
    .add(
      solend.initLendingMarketInstruction(
        owner,
        quoteCurrency("USD"),
        lendingMarketAccount.publicKey,
        pythProgramId,
        switchboardProgramId,
        PROGRAM_ID
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
  reserveTokenMint: Token,
  pythProduct: PublicKey,
  pythPrice: PublicKey,
  switchboardFeed: PublicKey,
  lendingMarket: PublicKey
) {
  const collateralMint = anchor.web3.Keypair.generate();
  const [lendingMarketAuthority] = await PublicKey.findProgramAddress(
    [lendingMarket.toBuffer()],
    PROGRAM_ID
  );

  const reserve = anchor.web3.Keypair.generate();
  const liquiditySupply = anchor.web3.Keypair.generate();
  const collateralSupply = anchor.web3.Keypair.generate();
  const liquidityFeeReceiver = anchor.web3.Keypair.generate();
  const userCollateral = anchor.web3.Keypair.generate();
  const userTransferAuthority = anchor.web3.Keypair.generate();

  const reserveBalance = await provider.connection.getMinimumBalanceForRentExemption(
    solend.RESERVE_SIZE
  );
  const mintBalance = await provider.connection.getMinimumBalanceForRentExemption(
    MintLayout.span
  );
  const accountBalance = await provider.connection.getMinimumBalanceForRentExemption(
    AccountLayout.span
  );

  const tx1 = new anchor.web3.Transaction()
    .add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: reserve.publicKey,
        lamports: reserveBalance,
        space: solend.RESERVE_SIZE,
        programId: PROGRAM_ID,
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
      borrowFeeWad: 100_000_000_000n,
      /// 0.3% (Aave flash loan fee)
      flashLoanFeeWad: 3_000_000_000_000_000n,
      hostFeePercentage: 20,
    },
    depositLimit: 100_000_000n,
    borrowLimit: 100_000_000n,
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
      solend.initReserveInstruction(
        liquidityAmount,
        reserveConfig,
        ownerReserveTokenAccount,
        userCollateral.publicKey,
        reserve.publicKey,
        reserveTokenMint.publicKey,
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
        PROGRAM_ID
      )
    );
  await provider.sendAll([
    {
      tx: tx1,
      signers: [payer, reserve, collateralMint, collateralSupply, userCollateral],
    },
    { tx: tx2, signers: [payer, liquiditySupply, liquidityFeeReceiver] },
    { tx: tx3, signers: [owner, userTransferAuthority] },
  ]);
  return {
    program: PROGRAM_ID,
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
