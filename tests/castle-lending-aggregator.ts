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
} from "../sdk/src/index";

describe("castle-vault", () => {
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);
  const wallet = provider.wallet as anchor.Wallet;

  const program = anchor.workspace
    .CastleLendingAggregator as anchor.Program<CastleLendingAggregator>;

  const owner = Keypair.generate();

  const pythProduct = new PublicKey(
    "ALP8SdU9oARYVLgLR7LrqMNCYBnhtnQz1cj6bwgwQmgj"
  );
  const pythPrice = new PublicKey(
    "H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG"
  );
  const switchboardFeed = new PublicKey(
    "AdtRGGhmqvom3Jemp5YNrxd9q9unX36BZk1pujkkXijL"
  );

  const initialReserveAmount = 100;
  const depositAmount = 1000000000;
  const withdrawAmount = 500000000;
  const initialCollateralRatio = 1.0;
  const feeMgmtBps = 10000;
  const feeCarryBps = 10000;

  // TODO auto calculate from above vars
  const feeAmount = 8;

  let reserveToken: Token;

  let jet: JetReserveAsset;
  let solend: SolendReserveAsset;
  let port: PortReserveAsset;

  let vaultClient: VaultClient;

  async function initLendingMarkets() {
    const sig = await provider.connection.requestAirdrop(
      owner.publicKey,
      1000000000
    );
    await provider.connection.confirmTransaction(sig, "singleGossip");

    reserveToken = await Token.createMint(
      provider.connection,
      owner,
      owner.publicKey,
      null,
      2,
      TOKEN_PROGRAM_ID
    );

    const ownerReserveTokenAccount = await reserveToken.createAccount(
      owner.publicKey
    );
    await reserveToken.mintTo(
      ownerReserveTokenAccount,
      owner,
      [],
      3 * initialReserveAmount
    );

    const pythProgram = new PublicKey(
      "FsJ3A3u2vn5cTVofAjvy6y5kwABJAqYWpe4975bi2epH"
    );
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

  function testInit(strategyType: StrategyType): () => Promise<void> {
    return async function () {
      vaultClient = await VaultClient.initialize(
        program,
        provider.wallet as anchor.Wallet,
        reserveToken.publicKey,
        solend,
        port,
        jet,
        strategyType,
        owner.publicKey,
        feeCarryBps,
        feeMgmtBps
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
      await reserveToken.mintTo(
        userReserveTokenAccount,
        owner,
        [],
        depositAmount
      );

      await vaultClient.deposit(wallet, depositAmount, userReserveTokenAccount);

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
    };
  }

  function testWithdraw(
    expectUserLp: number,
    expectUserReserve: number
  ): () => Promise<void> {
    return async function () {
      await vaultClient.withdraw(wallet, withdrawAmount);

      const userReserveTokenAccount =
        await vaultClient.getUserReserveTokenAccount(wallet.publicKey);
      const userReserveTokenAccountInfo =
        await vaultClient.getReserveTokenAccountInfo(userReserveTokenAccount);
      assert.equal(
        userReserveTokenAccountInfo.amount.toNumber(),
        expectUserReserve
      );

      const userLpTokenAccount = await vaultClient.getUserLpTokenAccount(
        wallet.publicKey
      );
      const userLpTokenAccountInfo = await vaultClient.getLpTokenAccountInfo(
        userLpTokenAccount
      );
      assert.equal(userLpTokenAccountInfo.amount.toNumber(), expectUserLp);

      const feeReceiverAccountInfo =
        await vaultClient.getFeeReceiverAccountInfo();
      const feesReceived = feeReceiverAccountInfo.amount.toNumber();
      assert.notEqual(feesReceived, 0);
    };
  }

  function testRebalance(
    expectedSolendAllocation: number,
    expectedPortAllocation: number,
    expectedJetAllocation: number
  ): () => Promise<void> {
    return async function () {
      await vaultClient.rebalance();

      const vaultReserveTokenAccountInfo =
        await vaultClient.getReserveTokenAccountInfo(
          vaultClient.vaultState.vaultReserveToken
        );
      const vaultReserveTokens = vaultReserveTokenAccountInfo.amount.toNumber();
      assert(vaultReserveTokens <= 3);

      const vaultValue = depositAmount - (withdrawAmount - feeAmount);

      const solendCollateralRatio = 1;
      const expectedSolendValue = Math.floor(
        vaultValue * expectedSolendAllocation
      );
      assert.equal(
        (
          await vaultClient.solend.getLpTokenAccountValue(
            vaultClient.vaultState.vaultSolendLpToken
          )
        ).toNumber(),
        expectedSolendValue * solendCollateralRatio
      );
      const solendLiquiditySupplyAccountInfo =
        await reserveToken.getAccountInfo(
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
    };
  }

  describe("equal allocation strategy", () => {
    before(initLendingMarkets);

    it("Creates vault", testInit({ equalAllocation: {} }));

    it("Deposits to vault reserves", testDeposit());

    it(
      "Withdraws from vault reserves",
      testWithdraw(
        depositAmount * initialCollateralRatio - withdrawAmount,
        withdrawAmount - feeAmount
      )
    );

    it(
      "Forwards deposits to lending markets",
      testRebalance(1 / 3, 1 / 3, 1 / 3)
    );

    // This fee amount is higher since it takes longer to execute than the other strategy
    // TODO auto-calculate these based on which slots the txs are confirmed in
    const finalFeeAmount = feeAmount + 55;
    it(
      "Withdraws from lending markets",
      testWithdraw(0, depositAmount - finalFeeAmount)
    );
  });

  describe("max yield strategy", () => {
    before(initLendingMarkets);

    it("Creates vault", testInit({ maxYield: {} }));

    it("Deposits to vault reserves", testDeposit());

    it(
      "Withdraws from vault reserves",
      testWithdraw(
        depositAmount * initialCollateralRatio - withdrawAmount,
        withdrawAmount - feeAmount
      )
    );

    it("Forwards deposits to lending markets", async () => {
      await testRebalance(0, 0, 1)();

      // TODO borrow from solend to increase apy and ensure it switches to that
      // TODO borrow from port to increase apy and ensure it switches to that
    });

    // TODO auto-calculate these based on which slots the txs are confirmed in
    const finalFeeAmount = feeAmount + 43;
    it(
      "Withdraws from lending markets",
      testWithdraw(0, depositAmount - finalFeeAmount)
    );
  });
});
