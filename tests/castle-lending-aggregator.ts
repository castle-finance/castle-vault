import assert from "assert";
import { Amount, JetClient, JetReserve, JetUser } from "@jet-lab/jet-engine";
import * as anchor from "@project-serum/anchor";
import { TOKEN_PROGRAM_ID, Token } from "@solana/spl-token";
import {
  Keypair,
  PublicKey,
  SYSVAR_CLOCK_PUBKEY,
  TransactionInstruction,
} from "@solana/web3.js";

import {
  JetAccounts,
  PortAccounts,
  SolendAccounts,
  VaultClient,
  CastleLendingAggregator,
} from "@castlefinance/vault-sdk";

import * as jet from "./helpers/jet";
import * as port from "./helpers/port";
import * as solend from "./helpers/solend";

// TODO use provider.wallet instead of owner
describe("castle-vault", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);
  const wallet = provider.wallet as anchor.Wallet;

  const program = anchor.workspace
    .CastleLendingAggregator as anchor.Program<CastleLendingAggregator>;

  const owner = Keypair.generate();
  const payer = Keypair.generate();

  const pythProduct = new PublicKey("3Mnn2fX6rQyUsyELYms1sBJyChWofzSNRoqYzvgMVz5E");
  const pythPrice = new PublicKey("J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix");
  const switchboardFeed = new PublicKey("GvDMxPzN1sCj7L26YDK2HnMRXEQmQ2aemov8YBtPS7vR");

  const initialReserveAmount = 100;

  let reserveTokenMint: Token;
  let quoteTokenMint: Token;

  let solendAccounts: SolendAccounts;
  let portAccounts: PortAccounts;
  let jetAccounts: JetAccounts;

  before("Initialize lending markets", async () => {
    const sig = await provider.connection.requestAirdrop(payer.publicKey, 1000000000);
    await provider.connection.confirmTransaction(sig, "singleGossip");

    const sig2 = await provider.connection.requestAirdrop(owner.publicKey, 1000000000);
    await provider.connection.confirmTransaction(sig2, "singleGossip");

    quoteTokenMint = await Token.createMint(
      provider.connection,
      payer,
      owner.publicKey,
      null,
      2,
      TOKEN_PROGRAM_ID
    );

    reserveTokenMint = await Token.createMint(
      provider.connection,
      payer,
      owner.publicKey,
      null,
      2,
      TOKEN_PROGRAM_ID
    );

    const ownerReserveTokenAccount = await reserveTokenMint.createAccount(
      owner.publicKey
    );
    await reserveTokenMint.mintTo(
      ownerReserveTokenAccount,
      owner,
      [],
      3 * initialReserveAmount
    );

    const solendMarket = await solend.initLendingMarket(
      provider,
      owner.publicKey,
      payer,
      new PublicKey("gSbePebfvPy7tRqimPoVecS2UsBvYv46ynrzWocc92s"),
      new PublicKey("2TfB33aLaneQb5TNVwyDz3jSZXS6jdW2ARw1Dgf84XCG")
    );
    solendAccounts = await solend.addReserve(
      provider,
      initialReserveAmount,
      ownerReserveTokenAccount,
      owner,
      payer,
      reserveTokenMint,
      pythProduct,
      pythPrice,
      switchboardFeed,
      solendMarket.publicKey
    );

    const portMarket = await port.createLendingMarket(provider);
    portAccounts = await port.createDefaultReserve(
      provider,
      initialReserveAmount,
      ownerReserveTokenAccount,
      portMarket.publicKey,
      pythPrice,
      owner
    );

    const jetClient = await JetClient.connect(provider, true);
    const jetMarket = await jet.createLendingMarket(
      jetClient,
      provider.wallet.publicKey,
      quoteTokenMint.publicKey
    );
    jetAccounts = await jet.initReserve(
      jetClient,
      jetMarket.address,
      provider.wallet.publicKey,
      quoteTokenMint.publicKey,
      reserveTokenMint,
      TOKEN_PROGRAM_ID, // dummy dex market addr
      pythPrice,
      pythProduct
    );
    const jetReserve = await JetReserve.load(jetClient, jetAccounts.reserve);
    const jetUser = await JetUser.load(
      jetClient,
      jetMarket,
      [jetReserve],
      owner.publicKey
    );
    const depositTx = await jetUser.makeDepositTx(
      jetReserve,
      ownerReserveTokenAccount,
      Amount.tokens(initialReserveAmount)
    );
    await provider.send(depositTx, [owner]);
  });

  let vaultClient: VaultClient;
  let refreshIx: TransactionInstruction;

  // TODO create test vaults for each strategy
  it("Creates vault", async () => {
    vaultClient = await VaultClient.initialize(
      program,
      provider.wallet as anchor.Wallet,
      reserveTokenMint.publicKey,
      solendAccounts.collateralMint,
      portAccounts.collateralMint,
      jetAccounts.depositNoteMint,
      { equalAllocation: {} }
    );
    // TODO add more checks
    assert.notEqual(vaultClient.vaultState, null);

    refreshIx = vaultClient.getRefreshIx(solendAccounts, portAccounts, jetAccounts);
  });

  let userLpTokenAccount: PublicKey;

  const depositAmount = 1000;
  const initialCollateralRatio = 1.0;

  it("Deposits to vault reserves", async () => {
    // Create depositor token account
    const userReserveTokenAccount = await reserveTokenMint.createAccount(
      wallet.publicKey
    );
    await reserveTokenMint.mintTo(userReserveTokenAccount, owner, [], depositAmount);
    userLpTokenAccount = await vaultClient.getUserLpTokenAccount(wallet);

    await vaultClient.deposit(
      wallet,
      depositAmount,
      userReserveTokenAccount,
      solendAccounts,
      portAccounts,
      jetAccounts
    );

    const userTokenAccountInfo = await reserveTokenMint.getAccountInfo(
      userReserveTokenAccount
    );
    assert.equal(userTokenAccountInfo.amount.toNumber(), 0);

    const tokenAccountInfo = await reserveTokenMint.getAccountInfo(
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
    await vaultClient.withdraw(
      wallet,
      withdrawAmount,
      userLpTokenAccount,
      solendAccounts,
      portAccounts,
      jetAccounts
    );

    const userReserveTokenAccount = await vaultClient.getUserReserveTokenAccount(
      wallet
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

  let reconcileSolendIx: TransactionInstruction;
  let reconcilePortIx: TransactionInstruction;
  let reconcileJetIx: TransactionInstruction;
  it("Forwards deposits to lending markets", async () => {
    const tx = new anchor.web3.Transaction();
    tx.add(refreshIx);
    tx.add(
      program.instruction.rebalance(new anchor.BN(0), {
        accounts: {
          vault: vaultClient.vaultId,
          vaultReserveToken: vaultClient.vaultState.vaultReserveToken,
          vaultSolendLpToken: vaultClient.vaultState.vaultSolendLpToken,
          vaultPortLpToken: vaultClient.vaultState.vaultPortLpToken,
          vaultJetLpToken: vaultClient.vaultState.vaultJetLpToken,
          solendReserveState: solendAccounts.reserve,
          portReserveState: portAccounts.reserve,
          jetReserveState: jetAccounts.reserve,
        },
      })
    );
    reconcileSolendIx = program.instruction.reconcileSolend({
      accounts: {
        vault: vaultClient.vaultId,
        vaultAuthority: vaultClient.vaultState.vaultAuthority,
        vaultReserveToken: vaultClient.vaultState.vaultReserveToken,
        vaultSolendLpToken: vaultClient.vaultState.vaultSolendLpToken,
        solendProgram: solendAccounts.program,
        solendMarketAuthority: solendAccounts.marketAuthority,
        solendMarket: solendAccounts.market,
        solendReserveState: solendAccounts.reserve,
        solendLpMint: solendAccounts.collateralMint,
        solendReserveToken: solendAccounts.liquiditySupply,
        clock: SYSVAR_CLOCK_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });
    tx.add(reconcileSolendIx);

    reconcilePortIx = program.instruction.reconcilePort({
      accounts: {
        vault: vaultClient.vaultId,
        vaultAuthority: vaultClient.vaultState.vaultAuthority,
        vaultReserveToken: vaultClient.vaultState.vaultReserveToken,
        vaultPortLpToken: vaultClient.vaultState.vaultPortLpToken,
        portProgram: port.PORT_LENDING,
        portMarketAuthority: portAccounts.marketAuthority,
        portMarket: portAccounts.market,
        portReserveState: portAccounts.reserve,
        portLpMint: portAccounts.collateralMint,
        portReserveToken: portAccounts.liquiditySupply,
        clock: SYSVAR_CLOCK_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });
    tx.add(reconcilePortIx);

    reconcileJetIx = program.instruction.reconcileJet({
      accounts: {
        vault: vaultClient.vaultId,
        vaultAuthority: vaultClient.vaultState.vaultAuthority,
        vaultReserveToken: vaultClient.vaultState.vaultReserveToken,
        vaultJetLpToken: vaultClient.vaultState.vaultJetLpToken,
        jetProgram: jetAccounts.program,
        jetMarket: jetAccounts.market,
        jetMarketAuthority: jetAccounts.marketAuthority,
        jetReserveState: jetAccounts.reserve,
        jetReserveToken: jetAccounts.liquiditySupply,
        jetLpMint: jetAccounts.depositNoteMint,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });
    tx.add(reconcileJetIx);

    await provider.send(tx);

    const vaultReserveTokenAccountInfo = await reserveTokenMint.getAccountInfo(
      vaultClient.vaultState.vaultReserveToken
    );
    assert.equal(vaultReserveTokenAccountInfo.amount.toNumber(), 2);

    const solendCollateralRatio = 1;
    const solendAllocation = 0.332;
    const solendCollateralToken = new Token(
      provider.connection,
      solendAccounts.collateralMint,
      TOKEN_PROGRAM_ID,
      payer
    );
    const vaultSolendLpTokenAccountInfo = await solendCollateralToken.getAccountInfo(
      vaultClient.vaultState.vaultSolendLpToken
    );
    assert.equal(
      vaultSolendLpTokenAccountInfo.amount.toNumber(),
      (depositAmount - withdrawAmount) * solendAllocation * solendCollateralRatio
    );
    const solendLiquiditySupplyAccountInfo = await reserveTokenMint.getAccountInfo(
      solendAccounts.liquiditySupply
    );
    assert.equal(
      solendLiquiditySupplyAccountInfo.amount.toNumber(),
      (depositAmount - withdrawAmount) * solendAllocation + initialReserveAmount
    );

    const portCollateralRatio = 1;
    const portAllocation = 0.332;
    const portCollateralToken = new Token(
      provider.connection,
      portAccounts.collateralMint,
      TOKEN_PROGRAM_ID,
      payer
    );
    const vaultPortLpTokenAccountInfo = await portCollateralToken.getAccountInfo(
      vaultClient.vaultState.vaultPortLpToken
    );
    assert.equal(
      vaultPortLpTokenAccountInfo.amount.toNumber(),
      (depositAmount - withdrawAmount) * portAllocation * portCollateralRatio
    );
    const portLiquiditySupplyAccountInfo = await reserveTokenMint.getAccountInfo(
      portAccounts.liquiditySupply
    );
    assert.equal(
      portLiquiditySupplyAccountInfo.amount.toNumber(),
      (depositAmount - withdrawAmount) * portAllocation + initialReserveAmount
    );

    const jetCollateralRatio = 1;
    const jetAllocation = 0.332;
    const jetCollateralToken = new Token(
      provider.connection,
      jetAccounts.depositNoteMint,
      TOKEN_PROGRAM_ID,
      payer
    );
    const vaultJetLpTokenAccountInfo = await jetCollateralToken.getAccountInfo(
      vaultClient.vaultState.vaultJetLpToken
    );
    assert.equal(
      vaultJetLpTokenAccountInfo.amount.toNumber(),
      (depositAmount - withdrawAmount) * jetAllocation * jetCollateralRatio
    );

    const jetLiquiditySupplyAccountInfo = await reserveTokenMint.getAccountInfo(
      jetAccounts.liquiditySupply
    );
    assert.equal(
      jetLiquiditySupplyAccountInfo.amount.toNumber(),
      (depositAmount - withdrawAmount) * jetAllocation + initialReserveAmount
    );
  });

  it("Rebalances", async () => {
    // TODO
  });

  it("Withdraws from lending programs", async () => {
    // TODO
  });
});
