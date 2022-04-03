import assert from "assert";
import * as anchor from "@project-serum/anchor";
import { TOKEN_PROGRAM_ID, Token, NATIVE_MINT } from "@solana/spl-token";
import { Keypair, PublicKey } from "@solana/web3.js";

import {
  SolendReserveAsset,
  JetReserveAsset,
  PortReserveAsset,
  VaultClient,
  CastleLendingAggregator,
  StrategyType,
  RebalanceMode,
} from "../sdk/src/index";

describe("castle-vault", () => {
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);
  const wallet = provider.wallet as anchor.Wallet;

  const program = anchor.workspace
    .CastleLendingAggregator as anchor.Program<CastleLendingAggregator>;

  const owner = Keypair.generate();

  const pythProduct = new PublicKey("ALP8SdU9oARYVLgLR7LrqMNCYBnhtnQz1cj6bwgwQmgj");
  const pythPrice = new PublicKey("H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG");
  const switchboardFeed = new PublicKey("AdtRGGhmqvom3Jemp5YNrxd9q9unX36BZk1pujkkXijL");

  const slotsPerYear = 63072000;
  const initialReserveAmount = 100;
  const depositAmount = 1 * 10 ** 9;
  const withdrawAmount = 0.5 * 10 ** 9;
  const initialCollateralRatio = 1.0;
  const feeMgmtBps = 10000;
  const feeCarryBps = 10000;
  const referralFeePct = 20;
  const referralFeeOwner = Keypair.generate().publicKey;

  let reserveToken: Token;

  let jet: JetReserveAsset;
  let solend: SolendReserveAsset;
  let port: PortReserveAsset;

  let vaultClient: VaultClient;

  let expectedWithdrawAmount: anchor.BN;
  let lastUpdatedVaultBalance: anchor.BN;
  let totalFees: { primary: anchor.BN; referral: anchor.BN };
  let lastUpdatedSlot: number;

  async function fetchSlots(txs: string[]): Promise<number[]> {
    const slots = (
      await Promise.all(
        txs.map((tx) => {
          return provider.connection.getParsedConfirmedTransaction(tx, "confirmed");
        })
      )
    ).map((res) => res.slot);

    return slots;
  }

  function calcReserveToLp(
    amount: anchor.BN,
    lpSupply: anchor.BN,
    vaultValue: anchor.BN
  ): anchor.BN {
    return lpSupply.mul(amount).div(vaultValue);
  }

  function splitFees(
    total: anchor.BN,
    splitPercentage: number
  ): [anchor.BN, anchor.BN] {
    const primFees = total
      .mul(new anchor.BN(100 - splitPercentage))
      .div(new anchor.BN(100));

    const refFees = total.mul(new anchor.BN(splitPercentage)).div(new anchor.BN(100));

    return [primFees, refFees];
  }

  async function calculateFees(
    vaultBalance: anchor.BN,
    lpMintSupply: anchor.BN,
    currentSlot: number,
    slots: number[]
  ) {
    const bpsWhole = new anchor.BN(10_000);

    let primFees = new anchor.BN(0);
    let refFees = new anchor.BN(0);

    for (const newSlot of slots) {
      // TODO add carry fee calculation
      //const carryFees = newVaultBalance
      //  .sub(vaultBalance)
      //  .mul(new anchor.BN(feeCarryBps))
      //  .div(bpsWhole);

      const mgmtFees = vaultBalance
        .mul(new anchor.BN(feeMgmtBps))
        .div(bpsWhole)
        .div(new anchor.BN(slotsPerYear))
        .div(new anchor.BN(newSlot - currentSlot));

      const tFees = mgmtFees;
      const tLpFees = calcReserveToLp(tFees, lpMintSupply, vaultBalance);

      const [primFee, refFee] = splitFees(tLpFees, referralFeePct);

      primFees = primFees.add(primFee);
      refFees = refFees.add(refFee);

      lpMintSupply = lpMintSupply.add(tLpFees);
      currentSlot = newSlot;
    }

    return { primary: primFees, referral: refFees };
  }

  async function initLendingMarkets() {
    lastUpdatedVaultBalance = new anchor.BN(0);
    expectedWithdrawAmount = new anchor.BN(0);
    totalFees = { primary: new anchor.BN(0), referral: new anchor.BN(0) };
    lastUpdatedSlot = 0;

    const sig = await provider.connection.requestAirdrop(owner.publicKey, 1000000000);

    const supplSig = await provider.connection.requestAirdrop(
      referralFeeOwner,
      1000000000
    );

    await provider.connection.confirmTransaction(sig, "singleGossip");
    await provider.connection.confirmTransaction(supplSig, "singleGossip");

    reserveToken = await Token.createMint(
      provider.connection,
      owner,
      owner.publicKey,
      null,
      2,
      TOKEN_PROGRAM_ID
    );

    const ownerReserveTokenAccount = await reserveToken.createAccount(owner.publicKey);

    await reserveToken.mintTo(
      ownerReserveTokenAccount,
      owner,
      [],
      3 * initialReserveAmount
    );

    const pythProgram = new PublicKey("FsJ3A3u2vn5cTVofAjvy6y5kwABJAqYWpe4975bi2epH");
    const switchboardProgram = new PublicKey(
      "DtmE9D2CSB4L5D6A15mraeEjrGMm6auWVzgaD8hK2tZM"
    );

    solend = await SolendReserveAsset.initialize(
      provider,
      owner,
      wallet,
      reserveToken.publicKey,
      pythProgram,
      switchboardProgram,
      pythProduct,
      pythPrice,
      switchboardFeed,
      ownerReserveTokenAccount,
      initialReserveAmount
    );

    port = await PortReserveAsset.initialize(
      provider,
      owner,
      reserveToken.publicKey,
      pythPrice,
      ownerReserveTokenAccount,
      initialReserveAmount
    );

    jet = await JetReserveAsset.initialize(
      provider,
      wallet,
      owner,
      NATIVE_MINT,
      reserveToken,
      pythPrice,
      pythProduct,
      ownerReserveTokenAccount,
      initialReserveAmount
    );
  }

  function testInit(
    strategyType: StrategyType,
    rebalanceMode: RebalanceMode
  ): () => Promise<void> {
    return async function () {
      vaultClient = await VaultClient.initialize(
        program,
        provider.wallet as anchor.Wallet,
        reserveToken.publicKey,
        solend,
        port,
        jet,
        strategyType,
        rebalanceMode,
        owner.publicKey,
        { feeCarryBps, feeMgmtBps, referralFeeOwner, referralFeePct }
      );
      // TODO add more checks
      assert.notEqual(vaultClient.vaultState, null);
    };
  }

  function testDeposit(): () => Promise<void> {
    return async function () {
      const userReserveTokenAccount = await reserveToken.createAccount(
        wallet.publicKey
      );
      await reserveToken.mintTo(userReserveTokenAccount, owner, [], depositAmount);

      const txs = await vaultClient.deposit(
        wallet,
        depositAmount,
        userReserveTokenAccount
      );
      await provider.connection.confirmTransaction(txs[txs.length - 1], "singleGossip");
      const depositTxSlots = await fetchSlots(txs);

      const userTokenAccountInfo = await reserveToken.getAccountInfo(
        userReserveTokenAccount
      );
      assert.equal(userTokenAccountInfo.amount.toNumber(), 0);

      const tokenAccountInfo = await reserveToken.getAccountInfo(
        vaultClient.vaultState.vaultReserveToken
      );
      assert.equal(tokenAccountInfo.amount.toNumber(), depositAmount);

      const userLpTokenAccount = await vaultClient.getUserLpTokenAccount(
        wallet.publicKey
      );
      const userLpTokenAccountInfo = await vaultClient.getLpTokenAccountInfo(
        userLpTokenAccount
      );
      assert.equal(
        userLpTokenAccountInfo.amount.toNumber(),
        depositAmount * initialCollateralRatio
      );

      const lpTokenMintInfo = await vaultClient.getLpTokenMintInfo();
      assert.equal(
        lpTokenMintInfo.supply.toNumber(),
        depositAmount * initialCollateralRatio
      );

      lastUpdatedSlot = depositTxSlots[depositTxSlots.length - 1];
    };
  }

  function testWithdraw(expectUserLp: number): () => Promise<void> {
    return async function () {
      lastUpdatedVaultBalance = new anchor.BN(
        (await vaultClient.getTotalValue()).toNumber()
      );

      const beforeWithdrawLpSupply = (await vaultClient.getLpTokenMintInfo()).supply;

      const txs = await vaultClient.withdraw(wallet, withdrawAmount);
      await provider.connection.confirmTransaction(txs[txs.length - 1], "singleGossip");

      const withdrawTxSlots = await fetchSlots(txs);

      const lastUpdatedMintSupply = (await vaultClient.getLpTokenMintInfo()).supply.add(
        new anchor.BN(withdrawAmount)
      );

      const feeAmount = await calculateFees(
        lastUpdatedVaultBalance,
        beforeWithdrawLpSupply,
        lastUpdatedSlot,
        withdrawTxSlots
      );

      totalFees = {
        primary: totalFees.primary.add(feeAmount.primary),
        referral: totalFees.referral.add(feeAmount.referral),
      };

      expectedWithdrawAmount = new anchor.BN(withdrawAmount)
        .mul(lastUpdatedVaultBalance)
        .div(lastUpdatedMintSupply)
        .add(expectedWithdrawAmount);

      const userReserveTokenAccount = await vaultClient.getUserReserveTokenAccount(
        wallet.publicKey
      );
      const userReserveTokenAccountInfo = await vaultClient.getReserveTokenAccountInfo(
        userReserveTokenAccount
      );

      const userLpTokenAccount = await vaultClient.getUserLpTokenAccount(
        wallet.publicKey
      );
      const userLpTokenAccountInfo = await vaultClient.getLpTokenAccountInfo(
        userLpTokenAccount
      );

      const feeReceiverAccountInfo = await vaultClient.getFeeReceiverAccountInfo();
      const referralFeeAccountInfo =
        await vaultClient.getReferralFeeReceiverAccountInfo();

      const referralFeesReceived = referralFeeAccountInfo.amount.toNumber();
      const feesReceived = feeReceiverAccountInfo.amount.toNumber();

      assert.equal(userLpTokenAccountInfo.amount.toNumber(), expectUserLp);
      assert.equal(
        userReserveTokenAccountInfo.amount.toNumber(),
        expectedWithdrawAmount.toNumber()
      );

      assert.equal(feesReceived, totalFees.primary.toNumber());
      assert.equal(referralFeesReceived, totalFees.referral.toNumber());

      lastUpdatedSlot = withdrawTxSlots[withdrawTxSlots.length - 1];
    };
  }

  function testRebalance(
    expectedSolendAllocation: number,
    expectedPortAllocation: number,
    expectedJetAllocation: number
  ): () => Promise<void> {
    return async function () {
      const beforeRebalanceMintSupply = (await vaultClient.getLpTokenMintInfo()).supply;

      const txs = await vaultClient.rebalance();
      await provider.connection.confirmTransaction(txs[txs.length - 1], "singleGossip");

      const rebalanceTxSlots = await fetchSlots(txs);

      const vaultValue = (await vaultClient.getTotalValue()).toNumber();

      const feeAmount = await calculateFees(
        new anchor.BN(vaultValue),
        beforeRebalanceMintSupply,
        lastUpdatedSlot,
        rebalanceTxSlots
      );

      totalFees = {
        primary: totalFees.primary.add(feeAmount.primary),
        referral: totalFees.referral.add(feeAmount.referral),
      };

      const vaultReserveTokenAccountInfo = await vaultClient.getReserveTokenAccountInfo(
        vaultClient.vaultState.vaultReserveToken
      );
      const vaultReserveTokens = vaultReserveTokenAccountInfo.amount.toNumber();
      assert(vaultReserveTokens <= 3);

      const solendCollateralRatio = 1;
      const expectedSolendValue = Math.floor(vaultValue * expectedSolendAllocation);
      assert.equal(
        (
          await vaultClient.solend.getLpTokenAccountValue(
            vaultClient.vaultState.vaultSolendLpToken
          )
        ).toNumber(),
        expectedSolendValue * solendCollateralRatio
      );
      const solendLiquiditySupplyAccountInfo = await reserveToken.getAccountInfo(
        vaultClient.solend.accounts.liquiditySupply
      );
      assert.equal(
        solendLiquiditySupplyAccountInfo.amount.toNumber(),
        expectedSolendValue + initialReserveAmount
      );

      const portCollateralRatio = 1;
      const expectedPortValue = Math.floor(vaultValue * expectedPortAllocation);
      assert.equal(
        (
          await vaultClient.port.getLpTokenAccountValue(
            vaultClient.vaultState.vaultPortLpToken
          )
        ).toNumber(),
        expectedPortValue * portCollateralRatio
      );
      const portLiquiditySupplyAccountInfo = await reserveToken.getAccountInfo(
        vaultClient.port.accounts.liquiditySupply
      );
      assert.equal(
        portLiquiditySupplyAccountInfo.amount.toNumber(),
        expectedPortValue + initialReserveAmount
      );

      const jetCollateralRatio = 1;
      const expectedJetValue = Math.floor(vaultValue * expectedJetAllocation);
      assert.equal(
        (
          await vaultClient.jet.getLpTokenAccountValue(
            vaultClient.vaultState.vaultJetLpToken
          )
        ).toNumber(),
        expectedJetValue * jetCollateralRatio
      );

      const jetLiquiditySupplyAccountInfo = await reserveToken.getAccountInfo(
        vaultClient.jet.accounts.liquiditySupply
      );
      assert.equal(
        jetLiquiditySupplyAccountInfo.amount.toNumber(),
        expectedJetValue + initialReserveAmount
      );

      lastUpdatedSlot = rebalanceTxSlots[rebalanceTxSlots.length - 1];
    };
  }

  describe("equal allocation strategy", () => {
    before(initLendingMarkets);

    it("Creates vault", testInit({ equalAllocation: {} }, { calculator: {} }));

    it("Deposits to vault reserves", testDeposit());

    it(
      "Withdraws from vault reserves",
      testWithdraw(depositAmount * initialCollateralRatio - withdrawAmount)
    );

    it("Forwards deposits to lending markets", testRebalance(1 / 3, 1 / 3, 1 / 3));

    it("Withdraws from lending markets", testWithdraw(0));
  });

  describe("max yield strategy", () => {
    before(initLendingMarkets);

    it("Creates vault", testInit({ maxYield: {} }, { calculator: {} }));

    it("Deposits to vault reserves", testDeposit());

    it(
      "Withdraws from vault reserves",
      testWithdraw(depositAmount * initialCollateralRatio - withdrawAmount)
    );

    it("Forwards deposits to lending markets", async () => {
      await testRebalance(0, 0, 1)();

      // TODO borrow from solend to increase apy and ensure it switches to that
      // TODO borrow from port to increase apy and ensure it switches to that
    });

    it("Withdraws from lending markets", testWithdraw(0));
  });
});
