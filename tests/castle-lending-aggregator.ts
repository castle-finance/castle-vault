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
} from "../sdk/src/index";
//} from "@castlefinance/vault-sdk";

describe("castle-vault", () => {
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);
  const wallet = provider.wallet as anchor.Wallet;

  const program = anchor.workspace
    .CastleLendingAggregator as anchor.Program<CastleLendingAggregator>;

  const owner = Keypair.generate();

  const pythProduct = new PublicKey("3Mnn2fX6rQyUsyELYms1sBJyChWofzSNRoqYzvgMVz5E");
  const pythPrice = new PublicKey("J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix");
  const switchboardFeed = new PublicKey("AdtRGGhmqvom3Jemp5YNrxd9q9unX36BZk1pujkkXijL");

  const initialReserveAmount = 100;

  let reserveToken: Token;

  let jet: JetReserveAsset;
  let solend: SolendReserveAsset;
  let port: PortReserveAsset;

  before("Initialize lending markets", async () => {
    const sig = await provider.connection.requestAirdrop(owner.publicKey, 1000000000);
    await provider.connection.confirmTransaction(sig, "singleGossip");

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

    const pythProgram = new PublicKey("gSbePebfvPy7tRqimPoVecS2UsBvYv46ynrzWocc92s");
    const switchboardProgram = new PublicKey(
      "7azgmy1pFXHikv36q1zZASvFq5vFa39TT9NweVugKKTU"
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
  });

  let vaultClient: VaultClient;

  // TODO create test vaults for each strategy
  it("Creates vault", async () => {
    vaultClient = await VaultClient.initialize(
      program,
      provider.wallet as anchor.Wallet,
      reserveToken.publicKey,
      solend,
      port,
      jet,
      { equalAllocation: {} }
    );
    // TODO add more checks
    assert.notEqual(vaultClient.vaultState, null);
  });

  let userLpTokenAccount: PublicKey;

  const depositAmount = 1000;
  const initialCollateralRatio = 1.0;

  it("Deposits to vault reserves", async () => {
    // Create depositor token account
    const userReserveTokenAccount = await reserveToken.createAccount(wallet.publicKey);
    await reserveToken.mintTo(userReserveTokenAccount, owner, [], depositAmount);
    userLpTokenAccount = await vaultClient.getUserLpTokenAccount(wallet.publicKey);

    await vaultClient.deposit(wallet, depositAmount, userReserveTokenAccount);

    const userTokenAccountInfo = await reserveToken.getAccountInfo(
      userReserveTokenAccount
    );
    assert.equal(userTokenAccountInfo.amount.toNumber(), 0);

    const tokenAccountInfo = await reserveToken.getAccountInfo(
      vaultClient.vaultState.vaultReserveToken
    );
    assert.equal(tokenAccountInfo.amount.toNumber(), depositAmount);

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
  });

  const withdrawAmount = 500;
  it("Withdraws from vault reserves", async () => {
    await vaultClient.withdraw(wallet, withdrawAmount, userLpTokenAccount);

    const userReserveTokenAccount = await vaultClient.getUserReserveTokenAccount(
      wallet.publicKey
    );
    const userReserveTokenAccountInfo = await vaultClient.getReserveTokenAccountInfo(
      userReserveTokenAccount
    );
    assert.equal(userReserveTokenAccountInfo.amount.toNumber(), withdrawAmount);

    const userLpTokenAccountInfo = await vaultClient.getLpTokenAccountInfo(
      userLpTokenAccount
    );
    assert.equal(
      userLpTokenAccountInfo.amount.toNumber(),
      depositAmount * initialCollateralRatio - withdrawAmount
    );
  });

  it("Forwards deposits to lending markets", async () => {
    await vaultClient.rebalance();

    const vaultReserveTokenAccountInfo = await vaultClient.getReserveTokenAccountInfo(
      vaultClient.vaultState.vaultReserveToken
    );
    assert.equal(vaultReserveTokenAccountInfo.amount.toNumber(), 2);

    const solendCollateralRatio = 1;
    const solendAllocation = 0.332;
    assert.equal(
      await vaultClient.solend.getLpTokenAccountValue(
        vaultClient.vaultState.vaultSolendLpToken
      ),
      (depositAmount - withdrawAmount) * solendAllocation * solendCollateralRatio
    );
    const solendLiquiditySupplyAccountInfo = await reserveToken.getAccountInfo(
      vaultClient.solend.accounts.liquiditySupply
    );
    assert.equal(
      solendLiquiditySupplyAccountInfo.amount.toNumber(),
      (depositAmount - withdrawAmount) * solendAllocation + initialReserveAmount
    );

    const portCollateralRatio = 1;
    const portAllocation = 0.332;
    assert.equal(
      await vaultClient.port.getLpTokenAccountValue(
        vaultClient.vaultState.vaultPortLpToken
      ),
      (depositAmount - withdrawAmount) * portAllocation * portCollateralRatio
    );
    const portLiquiditySupplyAccountInfo = await reserveToken.getAccountInfo(
      vaultClient.port.accounts.liquiditySupply
    );
    assert.equal(
      portLiquiditySupplyAccountInfo.amount.toNumber(),
      (depositAmount - withdrawAmount) * portAllocation + initialReserveAmount
    );

    const jetCollateralRatio = 1;
    const jetAllocation = 0.332;
    assert.equal(
      await vaultClient.jet.getLpTokenAccountValue(
        vaultClient.vaultState.vaultJetLpToken
      ),
      (depositAmount - withdrawAmount) * jetAllocation * jetCollateralRatio
    );

    const jetLiquiditySupplyAccountInfo = await reserveToken.getAccountInfo(
      vaultClient.jet.accounts.liquiditySupply
    );
    assert.equal(
      jetLiquiditySupplyAccountInfo.amount.toNumber(),
      (depositAmount - withdrawAmount) * jetAllocation + initialReserveAmount
    );
  });

  it("Withdraws from lending programs", async () => {
    await vaultClient.withdraw(wallet, withdrawAmount, userLpTokenAccount);

    const userReserveTokenAccount = await vaultClient.getUserReserveTokenAccount(
      wallet.publicKey
    );
    const userReserveTokenAccountInfo = await vaultClient.getReserveTokenAccountInfo(
      userReserveTokenAccount
    );
    assert.equal(userReserveTokenAccountInfo.amount.toNumber(), withdrawAmount * 2);

    const userLpTokenAccountInfo = await vaultClient.getLpTokenAccountInfo(
      userLpTokenAccount
    );
    assert.equal(userLpTokenAccountInfo.amount.toNumber(), 0);
  });
});
