const anchor = require("@project-serum/anchor");
const assert = require("assert");
const tokenLending = require("@dbricks/dbricks-solend");
const { Buffer } = require("buffer")
const { AccountLayout, MintLayout, TOKEN_PROGRAM_ID , Token} = require("@solana/spl-token");
const { PublicKey, SystemProgram, SYSVAR_CLOCK_PUBKEY} = require("@solana/web3.js");

/// TODO convert to ts
/// TODO use SDK instead of raw code
describe("castle-vault", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.CastleLendingAggregator;

  let vaultAuthority;
  let vaultSeed;
  let lpTokenMint;
  let reserveTokenMint;
  let vaultReserveTokenAccount;

  const owner = anchor.web3.Keypair.generate();
  // Can this be a PDA?
  const vaultStateAccount = anchor.web3.Keypair.generate();
  const payer = anchor.web3.Keypair.generate();

  // TODO add private key to fixtures
  const lendingProgram = new PublicKey("BwTGCAdzPncEFqP5JBAeCLRWKE8MDVvbGDVMD7XX2fvu");

  const quoteCurrency = (s) => {
    const buf = Buffer.alloc(32);
    const strBuf = Buffer.from(s);
    strBuf.copy(buf, 0);
    return buf;
  };

  const lendingMarket = anchor.web3.Keypair.generate();
  const collateralMint = anchor.web3.Keypair.generate();
  const reserve = anchor.web3.Keypair.generate();
  const liquiditySupply = anchor.web3.Keypair.generate();

  const pythProduct = new PublicKey("3Mnn2fX6rQyUsyELYms1sBJyChWofzSNRoqYzvgMVz5E");
  const pythPrice = new PublicKey("J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix");
  const switchboardFeed = new PublicKey("GvDMxPzN1sCj7L26YDK2HnMRXEQmQ2aemov8YBtPS7vR");

  let lendingMarketAuthority;
  let _lmaBumpSeed;

  before(async () => {
    const sig  = await provider.connection.requestAirdrop(payer.publicKey, 1000000000);
    await provider.connection.confirmTransaction(sig, "singleGossip");

    const pythProgramId = new PublicKey("gSbePebfvPy7tRqimPoVecS2UsBvYv46ynrzWocc92s");
    const switchboardProgramId = new PublicKey("2TfB33aLaneQb5TNVwyDz3jSZXS6jdW2ARw1Dgf84XCG");
    const balanceNeeded = await provider.connection.getMinimumBalanceForRentExemption(tokenLending.LENDING_MARKET_SIZE);
    const initTx = new anchor.web3.Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: lendingMarket.publicKey,
        lamports: balanceNeeded,
        space: tokenLending.LENDING_MARKET_SIZE,
        programId: lendingProgram,
      })
    ).add(
      tokenLending.initLendingMarketInstruction(
        owner.publicKey,
        quoteCurrency("USD"),
        lendingMarket.publicKey,
        pythProgramId,
        switchboardProgramId,
        lendingProgram
      )
    );
    await provider.send(initTx, [payer, lendingMarket]);

    reserveTokenMint = await Token.createMint(
      provider.connection,
      payer,
      owner.publicKey,
      null,
      2,
      TOKEN_PROGRAM_ID
    );
    const ownerReserveTokenAccount = await reserveTokenMint.createAccount(owner.publicKey);
    const liquidityAmount = 10;
    await reserveTokenMint.mintTo(ownerReserveTokenAccount, owner, [], liquidityAmount);

    const collateralSupply = anchor.web3.Keypair.generate();
    const liquidityFeeReceiver = anchor.web3.Keypair.generate();
    const userCollateral = anchor.web3.Keypair.generate();
    const userTransferAuthority = anchor.web3.Keypair.generate();

    const reserveBalance = await provider.connection.getMinimumBalanceForRentExemption(tokenLending.RESERVE_SIZE);
    const mintBalance = await provider.connection.getMinimumBalanceForRentExemption(MintLayout.span);
    const accountBalance = await provider.connection.getMinimumBalanceForRentExemption(AccountLayout.span);

    const tx1 = new anchor.web3.Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: reserve.publicKey,
        lamports: reserveBalance,
        space: tokenLending.RESERVE_SIZE,
        programId: lendingProgram,
      })
    ).add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: collateralMint.publicKey,
        lamports: mintBalance,
        space: MintLayout.span,
        programId: TOKEN_PROGRAM_ID,
      })
    ).add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: collateralSupply.publicKey,
        lamports: accountBalance,
        space: AccountLayout.span,
        programId: TOKEN_PROGRAM_ID,
      })
    ).add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: userCollateral.publicKey,
        lamports: accountBalance,
        space: AccountLayout.span,
        programId: TOKEN_PROGRAM_ID,
      })
    );
    const tx2 = new anchor.web3.Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: liquiditySupply.publicKey,
        lamports: accountBalance,
        space: AccountLayout.span,
        programId: TOKEN_PROGRAM_ID,
      })
    ).add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: liquidityFeeReceiver.publicKey,
        lamports: accountBalance,
        space: AccountLayout.span,
        programId: TOKEN_PROGRAM_ID,
      })
    );

    [lendingMarketAuthority, _lmaBumpSeed] = await PublicKey.findProgramAddress(
      [lendingMarket.publicKey.toBuffer()],
      lendingProgram,
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
    const tx3 = new anchor.web3.Transaction().add(
      Token.createApproveInstruction(
        TOKEN_PROGRAM_ID,
        ownerReserveTokenAccount,
        userTransferAuthority.publicKey,
        owner.publicKey,
        [],
        liquidityAmount,
      )
    ).add(
      tokenLending.initReserveInstruction(
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
        lendingMarket.publicKey,
        lendingMarketAuthority,
        owner.publicKey,
        userTransferAuthority.publicKey,
        switchboardFeed,
        lendingProgram,
      )
    );
    await provider.sendAll([
        {tx: tx1, signers: [payer, reserve, collateralMint, collateralSupply, userCollateral]},
        {tx: tx2, signers: [payer, liquiditySupply, liquidityFeeReceiver]},
        {tx: tx3, signers: [owner, userTransferAuthority]},
    ]);
  });

  it("Creates vault", async () => {
    [vaultAuthority, vaultSeed] = await PublicKey.findProgramAddress(
      [vaultStateAccount.publicKey.toBuffer()],
      program.programId,
    )
    lpTokenMint = await Token.createMint(
      provider.connection,
      payer,
      vaultAuthority,
      null,
      2,
      TOKEN_PROGRAM_ID
    );
    const ownerLpTokenAccount = await lpTokenMint.createAccount(owner.publicKey);
    
    // TODO change to require initial deposit
    vaultReserveTokenAccount = await reserveTokenMint.createAccount(vaultAuthority);
    await reserveTokenMint.mintTo(vaultReserveTokenAccount, owner, [], 1000);

    await program.rpc.initializePool(
      {
        accounts: {
          authority: vaultAuthority,
          reservePool: vaultStateAccount.publicKey,
          poolMint: lpTokenMint.publicKey,
          token: vaultReserveTokenAccount,
          destination: ownerLpTokenAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [vaultStateAccount],
        instructions: [await program.account.reservePool.createInstruction(vaultStateAccount)]
      }
    );

    const actualPoolAccount = await program.account.reservePool.fetch(vaultStateAccount.publicKey);
    assert(actualPoolAccount.tokenProgramId.equals(TOKEN_PROGRAM_ID));
    assert(actualPoolAccount.tokenAccount.equals(vaultReserveTokenAccount));
    assert(actualPoolAccount.tokenMint.equals(reserveTokenMint.publicKey));
    assert(actualPoolAccount.poolMint.equals(lpTokenMint.publicKey));

    const lpTokenMintInfo = await lpTokenMint.getMintInfo();
    assert.equal(lpTokenMintInfo.supply.toNumber(), 1000000);
  });

  let userLpTokenAccount;
  const depositAmount = 1000;
  it("Deposits to vault reserves", async () => {
    // Create depositor token account
    const userAuthority = anchor.web3.Keypair.generate();
    const userReserveTokenAccount = await reserveTokenMint.createAccount(owner.publicKey);
    await reserveTokenMint.mintTo(userReserveTokenAccount, owner, [], depositAmount);
    await reserveTokenMint.approve(
      userReserveTokenAccount,
      userAuthority.publicKey,
      owner,
      [],
      depositAmount,
    );

    // Create depositor pool LP token account
    userLpTokenAccount = await lpTokenMint.createAccount(owner.publicKey);

    await program.rpc.deposit(
      new anchor.BN(depositAmount),
      {
        accounts: {
          reservePool: vaultStateAccount.publicKey,
          authority: vaultAuthority,
          userAuthority: userAuthority.publicKey,
          source: userReserveTokenAccount,
          destination: userLpTokenAccount,
          token: vaultReserveTokenAccount,
          poolMint: lpTokenMint.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [userAuthority],
      }
    );

    const userTokenAccountInfo = await reserveTokenMint.getAccountInfo(userReserveTokenAccount);
    assert.equal(userTokenAccountInfo.amount.toNumber(), 0);

    const tokenAccountInfo = await reserveTokenMint.getAccountInfo(vaultReserveTokenAccount);
    assert.equal(tokenAccountInfo.amount.toNumber(), 2000);

    const userPoolTokenAccountInfo = await lpTokenMint.getAccountInfo(userLpTokenAccount);
    assert.equal(userPoolTokenAccountInfo.amount.toNumber(), 1000000);

    const lpTokenMintInfo = await lpTokenMint.getMintInfo();
    assert.equal(lpTokenMintInfo.supply.toNumber(), 2000000);
  });

  it("Forwards deposits to lending program", async () => {
    const collateralMintToken = new Token(provider.connection, collateralMint.publicKey, TOKEN_PROGRAM_ID, payer);
    const vaultLpTokenAccount = await collateralMintToken.createAccount(vaultAuthority);

    await program.rpc.rebalance(
      {
        accounts: {
          vaultState: vaultStateAccount.publicKey,
          vaultAuthority: vaultAuthority,
          vaultReserveToken: vaultReserveTokenAccount,
          vaultLpToken: vaultLpTokenAccount,
          solendProgram: lendingProgram,
          solendMarketAuthority: lendingMarketAuthority,
          solendMarket: lendingMarket.publicKey,
          solendReserveStateAccount: reserve.publicKey,
          solendLpMintAccount: collateralMint.publicKey,
          solendDepositTokenAccount: liquiditySupply.publicKey,
          solendPyth: pythPrice,
          solendSwitchboard: switchboardFeed,
          clock: SYSVAR_CLOCK_PUBKEY,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
      }
    );
    const vaultReserveTokenAccountInfo = await reserveTokenMint.getAccountInfo(vaultReserveTokenAccount);
    assert.equal(vaultReserveTokenAccountInfo.amount.toNumber(), 0);

    const vaultLpTokenAccountInfo = await collateralMintToken.getAccountInfo(vaultLpTokenAccount);
    assert.notEqual(vaultLpTokenAccountInfo.amount.toNumber(), 0);

    const liquiditySupplyAccountInfo = await reserveTokenMint.getAccountInfo(liquiditySupply.publicKey);
    assert.equal(liquiditySupplyAccountInfo.amount.toNumber(), 2010);
  });

  it("Rebalances", async () => {
  });

  it("Withdraws from vault", async () => {
    // Pool tokens to withdraw from
    const withdrawAmount = 500000;

    // Create token account to withdraw into
    const userReserveTokenAccount = await reserveTokenMint.createAccount(owner.publicKey);

    // Delegate authority to transfer pool tokens
    const userAuthority = anchor.web3.Keypair.generate();
    await lpTokenMint.approve(
      userLpTokenAccount,
      userAuthority.publicKey,
      owner,
      [],
      withdrawAmount,
    );

    await program.rpc.withdraw(
      new anchor.BN(withdrawAmount),
      {
        accounts: {
          reservePool: vaultStateAccount.publicKey,
          authority: vaultAuthority,
          userAuthority: userAuthority.publicKey,
          source: userLpTokenAccount,
          token: vaultReserveTokenAccount,
          destination: userReserveTokenAccount,
          poolMint: lpTokenMint.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [userAuthority],
      }
    );

    const userReserveTokenAccountInfo = await reserveTokenMint.getAccountInfo(userReserveTokenAccount);
    assert.equal(userReserveTokenAccountInfo.amount.toNumber(), 500);

    const vaultReserveTokenAccountInfo = await reserveTokenMint.getAccountInfo(vaultReserveTokenAccount);
    assert.equal(vaultReserveTokenAccountInfo.amount.toNumber(), 1500);

    const userLpTokenAccountInfo = await lpTokenMint.getAccountInfo(userLpTokenAccount);
    assert.equal(userLpTokenAccountInfo.amount.toNumber(), withdrawAmount);

    const lpTokenMintInfo = await lpTokenMint.getMintInfo();
    assert.equal(lpTokenMintInfo.supply.toNumber(), 1500000);
  });
});