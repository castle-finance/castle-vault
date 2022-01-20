import assert from "assert";
import * as anchor from "@project-serum/anchor";
import { TOKEN_PROGRAM_ID, Token } from "@solana/spl-token";
import {
  Keypair,
  PublicKey,
  SYSVAR_CLOCK_PUBKEY,
  TransactionInstruction,
} from "@solana/web3.js";

import * as jet from "./helpers/jet";
import * as port from "./helpers/port";
import { Solend } from "./helpers/solend";
import { CastleLendingAggregator } from "../target/types/castle_lending_aggregator";
import { Amount, JetClient, JetMarket, JetReserve, JetUser } from "@jet-lab/jet-engine";
import { VaultClient } from "../sdk/src/client";
import { VaultState } from "../sdk/src/types";

// TODO use provider.wallet instead of owner
describe("castle-vault", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace
    .CastleLendingAggregator as anchor.Program<CastleLendingAggregator>;

  const owner = anchor.web3.Keypair.generate();
  const payer = anchor.web3.Keypair.generate();

  const solendCollateralMint = anchor.web3.Keypair.generate();
  const solendReserve = anchor.web3.Keypair.generate();
  const solendLiquiditySupply = anchor.web3.Keypair.generate();

  // TODO change to devnet version
  const solendProgramId = new PublicKey("ALend7Ketfx5bxh6ghsCDXAoDrhvEmsXT3cynB6aPLgx");
  const solendProgram = new Solend(provider, solendProgramId);

  const pythProduct = new PublicKey("3Mnn2fX6rQyUsyELYms1sBJyChWofzSNRoqYzvgMVz5E");
  const pythPrice = new PublicKey("J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix");
  const switchboardFeed = new PublicKey("GvDMxPzN1sCj7L26YDK2HnMRXEQmQ2aemov8YBtPS7vR");

  const jetProgram = new PublicKey("JPv1rCqrhagNNmJVM5J1he7msQ5ybtvE1nNuHpDHMNU");

  const initialReserveAmount = 100;

  let portReserveState: port.ReserveState;
  let jetReserveAccounts: jet.ReserveAccounts;

  let reserveTokenMint: Token;
  let quoteTokenMint: Token;

  let solendMarket: Keypair;
  let solendMarketAuthority: PublicKey;
  let portMarket: Keypair;
  let portMarketAuthority: PublicKey;
  let jetMarket: JetMarket;
  let jetMarketAuthority: PublicKey;

  let vaultId: PublicKey;
  let vaultState: VaultState;

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

    solendMarket = await solendProgram.initLendingMarket(
      owner.publicKey,
      payer,
      new PublicKey("gSbePebfvPy7tRqimPoVecS2UsBvYv46ynrzWocc92s"),
      new PublicKey("2TfB33aLaneQb5TNVwyDz3jSZXS6jdW2ARw1Dgf84XCG")
    );

    [solendMarketAuthority] = await PublicKey.findProgramAddress(
      [solendMarket.publicKey.toBuffer()],
      solendProgramId
    );

    await solendProgram.addReserve(
      initialReserveAmount,
      ownerReserveTokenAccount,
      owner,
      payer,
      reserveTokenMint,
      solendReserve,
      solendCollateralMint,
      solendLiquiditySupply,
      pythProduct,
      pythPrice,
      switchboardFeed,
      solendMarket.publicKey,
      solendMarketAuthority
    );

    console.log("Initialized Solend");

    portMarket = await port.createLendingMarket(provider);

    [portMarketAuthority] = await PublicKey.findProgramAddress(
      [portMarket.publicKey.toBuffer()],
      port.PORT_LENDING
    );

    portReserveState = await port.createDefaultReserve(
      provider,
      initialReserveAmount,
      ownerReserveTokenAccount,
      portMarket.publicKey,
      pythPrice,
      owner
    );

    console.log("Initialized Port");

    const jetClient = await JetClient.connect(provider, true);
    jetMarket = await jet.createLendingMarket(
      jetClient,
      provider.wallet.publicKey,
      quoteTokenMint.publicKey
    );
    jetMarketAuthority = await jet.getMarketAuthority(jetMarket.address);
    jetReserveAccounts = await jet.initReserve(
      jetClient,
      jetMarket.address,
      provider.wallet.publicKey,
      quoteTokenMint.publicKey,
      reserveTokenMint,
      TOKEN_PROGRAM_ID, // dummy dex market addr
      pythPrice,
      pythProduct
    );
    const jetReserve = await JetReserve.load(
      jetClient,
      jetReserveAccounts.accounts.reserve.publicKey
    );
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

    console.log("Initialized Jet");
  });

  // TODO create test vaults for each strategy
  it("Creates vault", async () => {
    [vaultId, vaultState] = await VaultClient.initialize(
      program,
      provider.wallet as anchor.Wallet,
      reserveTokenMint.publicKey,
      solendCollateralMint.publicKey,
      portReserveState.collateralMintAccount,
      jetReserveAccounts.accounts.depositNoteMint,
      { equalAllocation: {} }
    );

    // TODO add more checks
    assert.notEqual(vaultState, null);
  });

  let lpToken: Token;
  let userLpTokenAccount: PublicKey;
  let refreshIx: anchor.web3.TransactionInstruction;

  const depositAmount = 1000;
  const initialCollateralRatio = 1.0;

  it("Deposits to vault reserves", async () => {
    // Create depositor token account
    const userAuthority = anchor.web3.Keypair.generate();
    const userReserveTokenAccount = await reserveTokenMint.createAccount(
      owner.publicKey
    );
    await reserveTokenMint.mintTo(userReserveTokenAccount, owner, [], depositAmount);
    await reserveTokenMint.approve(
      userReserveTokenAccount,
      userAuthority.publicKey,
      owner,
      [],
      depositAmount
    );

    lpToken = new Token(
      provider.connection,
      vaultState.lpTokenMint,
      TOKEN_PROGRAM_ID,
      payer
    );
    userLpTokenAccount = await lpToken.createAccount(owner.publicKey);

    refreshIx = program.instruction.refresh({
      accounts: {
        vault: vaultId,
        vaultReserveToken: vaultState.vaultReserveToken,
        vaultSolendLpToken: vaultState.vaultSolendLpToken,
        vaultPortLpToken: vaultState.vaultPortLpToken,
        vaultJetLpToken: vaultState.vaultJetLpToken,
        solendProgram: solendProgramId,
        solendReserveState: solendReserve.publicKey,
        solendPyth: pythPrice,
        solendSwitchboard: switchboardFeed,
        portProgram: port.PORT_LENDING,
        portReserveState: portReserveState.address,
        portOracle: portReserveState.oracle,
        jetProgram: jetProgram,
        jetMarket: jetMarket.address,
        jetMarketAuthority: jetMarket.marketAuthority,
        jetReserveState: jetReserveAccounts.accounts.reserve.publicKey,
        jetFeeNoteVault: jetReserveAccounts.accounts.feeNoteVault,
        jetDepositNoteMint: jetReserveAccounts.accounts.depositNoteMint,
        jetPyth: jetReserveAccounts.accounts.pythPrice,
        tokenProgram: TOKEN_PROGRAM_ID,
        clock: SYSVAR_CLOCK_PUBKEY,
      },
    });

    await program.rpc.deposit(new anchor.BN(depositAmount), {
      accounts: {
        vault: vaultId,
        vaultAuthority: vaultState.vaultAuthority,
        vaultReserveToken: vaultState.vaultReserveToken,
        lpTokenMint: vaultState.lpTokenMint,
        userReserveToken: userReserveTokenAccount,
        userLpToken: userLpTokenAccount,
        userAuthority: userAuthority.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
      signers: [userAuthority],
      instructions: [refreshIx],
    });

    const userTokenAccountInfo = await reserveTokenMint.getAccountInfo(
      userReserveTokenAccount
    );
    assert.equal(userTokenAccountInfo.amount.toNumber(), 0);

    const tokenAccountInfo = await reserveTokenMint.getAccountInfo(
      vaultState.vaultReserveToken
    );
    assert.equal(tokenAccountInfo.amount.toNumber(), depositAmount);

    const userPoolTokenAccountInfo = await lpToken.getAccountInfo(userLpTokenAccount);
    assert.equal(
      userPoolTokenAccountInfo.amount.toNumber(),
      depositAmount * initialCollateralRatio
    );

    const lpTokenMintInfo = await lpToken.getMintInfo();
    assert.equal(
      lpTokenMintInfo.supply.toNumber(),
      depositAmount * initialCollateralRatio
    );
  });

  const withdrawAmount = 500;
  it("Withdraws from vault reserves", async () => {
    // Create token account to withdraw into
    const userReserveTokenAccount = await reserveTokenMint.createAccount(
      owner.publicKey
    );

    // Delegate authority to transfer pool tokens
    const userAuthority = anchor.web3.Keypair.generate();
    await lpToken.approve(
      userLpTokenAccount,
      userAuthority.publicKey,
      owner,
      [],
      withdrawAmount
    );

    await program.rpc.withdraw(new anchor.BN(withdrawAmount), {
      accounts: {
        vault: vaultId,
        vaultAuthority: vaultState.vaultAuthority,
        userAuthority: userAuthority.publicKey,
        userLpToken: userLpTokenAccount,
        userReserveToken: userReserveTokenAccount,
        vaultReserveToken: vaultState.vaultReserveToken,
        vaultLpMint: vaultState.lpTokenMint,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
      signers: [userAuthority],
      instructions: [refreshIx],
    });

    const userReserveTokenAccountInfo = await reserveTokenMint.getAccountInfo(
      userReserveTokenAccount
    );
    assert.equal(userReserveTokenAccountInfo.amount.toNumber(), withdrawAmount);

    const userLpTokenAccountInfo = await lpToken.getAccountInfo(userLpTokenAccount);
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
          vault: vaultId,
          vaultReserveToken: vaultState.vaultReserveToken,
          vaultSolendLpToken: vaultState.vaultSolendLpToken,
          vaultPortLpToken: vaultState.vaultPortLpToken,
          vaultJetLpToken: vaultState.vaultJetLpToken,
          solendReserveState: solendReserve.publicKey,
          portReserveState: portReserveState.address,
          jetReserveState: jetReserveAccounts.accounts.reserve.publicKey,
        },
      })
    );
    reconcileSolendIx = program.instruction.reconcileSolend({
      accounts: {
        vault: vaultId,
        vaultAuthority: vaultState.vaultAuthority,
        vaultReserveToken: vaultState.vaultReserveToken,
        vaultSolendLpToken: vaultState.vaultSolendLpToken,
        solendProgram: solendProgramId,
        solendMarketAuthority: solendMarketAuthority,
        solendMarket: solendMarket.publicKey,
        solendReserveState: solendReserve.publicKey,
        solendLpMint: solendCollateralMint.publicKey,
        solendReserveToken: solendLiquiditySupply.publicKey,
        clock: SYSVAR_CLOCK_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });
    tx.add(reconcileSolendIx);

    reconcilePortIx = program.instruction.reconcilePort({
      accounts: {
        vault: vaultId,
        vaultAuthority: vaultState.vaultAuthority,
        vaultReserveToken: vaultState.vaultReserveToken,
        vaultPortLpToken: vaultState.vaultPortLpToken,
        portProgram: port.PORT_LENDING,
        portMarketAuthority: portMarketAuthority,
        portMarket: portMarket.publicKey,
        portReserveState: portReserveState.address,
        portLpMint: portReserveState.collateralMintAccount,
        portReserveToken: portReserveState.liquiditySupplyPubkey,
        clock: SYSVAR_CLOCK_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });
    tx.add(reconcilePortIx);

    reconcileJetIx = program.instruction.reconcileJet({
      accounts: {
        vault: vaultId,
        vaultAuthority: vaultState.vaultAuthority,
        vaultReserveToken: vaultState.vaultReserveToken,
        vaultJetLpToken: vaultState.vaultJetLpToken,
        jetProgram: jet.PROGRAM_ID,
        jetMarket: jetMarket.address,
        jetMarketAuthority: jetMarketAuthority,
        jetReserveState: jetReserveAccounts.accounts.reserve.publicKey,
        jetReserveToken: jetReserveAccounts.accounts.vault,
        jetLpMint: jetReserveAccounts.accounts.depositNoteMint,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });
    tx.add(reconcileJetIx);

    await provider.send(tx);

    const vaultReserveTokenAccountInfo = await reserveTokenMint.getAccountInfo(
      vaultState.vaultReserveToken
    );
    assert.equal(vaultReserveTokenAccountInfo.amount.toNumber(), 2);

    const solendCollateralRatio = 1;
    const solendAllocation = 0.332;
    const solendCollateralToken = new Token(
      provider.connection,
      solendCollateralMint.publicKey,
      TOKEN_PROGRAM_ID,
      payer
    );
    const vaultSolendLpTokenAccountInfo = await solendCollateralToken.getAccountInfo(
      vaultState.vaultSolendLpToken
    );
    assert.equal(
      vaultSolendLpTokenAccountInfo.amount.toNumber(),
      (depositAmount - withdrawAmount) * solendAllocation * solendCollateralRatio
    );
    const solendLiquiditySupplyAccountInfo = await reserveTokenMint.getAccountInfo(
      solendLiquiditySupply.publicKey
    );
    assert.equal(
      solendLiquiditySupplyAccountInfo.amount.toNumber(),
      (depositAmount - withdrawAmount) * solendAllocation + initialReserveAmount
    );

    const portCollateralRatio = 1;
    const portAllocation = 0.332;
    const portCollateralToken = new Token(
      provider.connection,
      portReserveState.collateralMintAccount,
      TOKEN_PROGRAM_ID,
      payer
    );
    const vaultPortLpTokenAccountInfo = await portCollateralToken.getAccountInfo(
      vaultState.vaultPortLpToken
    );
    assert.equal(
      vaultPortLpTokenAccountInfo.amount.toNumber(),
      (depositAmount - withdrawAmount) * portAllocation * portCollateralRatio
    );
    const portLiquiditySupplyAccountInfo = await reserveTokenMint.getAccountInfo(
      portReserveState.liquiditySupplyPubkey
    );
    assert.equal(
      portLiquiditySupplyAccountInfo.amount.toNumber(),
      (depositAmount - withdrawAmount) * portAllocation + initialReserveAmount
    );

    const jetCollateralRatio = 1;
    const jetAllocation = 0.332;
    const jetCollateralToken = new Token(
      provider.connection,
      jetReserveAccounts.accounts.depositNoteMint,
      TOKEN_PROGRAM_ID,
      payer
    );
    const vaultJetLpTokenAccountInfo = await jetCollateralToken.getAccountInfo(
      vaultState.vaultJetLpToken
    );
    assert.equal(
      vaultJetLpTokenAccountInfo.amount.toNumber(),
      (depositAmount - withdrawAmount) * jetAllocation * jetCollateralRatio
    );

    const jetLiquiditySupplyAccountInfo = await reserveTokenMint.getAccountInfo(
      jetReserveAccounts.accounts.vault
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
