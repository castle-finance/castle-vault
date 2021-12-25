const anchor = require("@project-serum/anchor");
const assert = require("assert");
const tokenLending = require("@solana/spl-token-lending");
const { Buffer } = require("buffer")
const { TOKEN_PROGRAM_ID , Token} = require("@solana/spl-token");
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

  // TODO possible to get dynamically?
  const lendingProgram = new PublicKey("BwTGCAdzPncEFqP5JBAeCLRWKE8MDVvbGDVMD7XX2fvu");

  const quoteCurrency = (s) => {
    const buf = Buffer.alloc(32);
    const strBuf = Buffer.from(s);
    strBuf.copy(buf, 0);
    return buf;
  };

  const lendingMarket = anchor.web3.Keypair.generate();
  //const lendingMarketLpMintAccount;
  //const lendingMarketLpToken = Token(
  //  provider.connection, 
  //  lendingMarketLpMintAccount, 
  //  TOKEN_PROGRAM_ID,
  //  payer,
  //);
  //const splLpTokenAccount = await lendingMarketLpToken.createAccount(authority);
  //const lendingMarketReserveStateAccount;
  //const lendingMarketDepositTokenAccount;
  //const lendingMarketAuthority;

  before(async () => {
    const sig  = await provider.connection.requestAirdrop(payer.publicKey, 1000000000);
    await provider.connection.confirmTransaction(sig, "singleGossip");

    const balanceNeeded = await provider.connection.getMinimumBalanceForRentExemption(258);
    const initTx = new anchor.web3.Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: lendingMarket.publicKey,
        lamports: balanceNeeded,
        space: 258,
        programId: lendingProgram,
      })
    ).add(
      tokenLending.initLendingMarketInstruction(
        owner.publicKey,
        quoteCurrency("USD"),
        lendingMarket.publicKey,
        lendingProgram
      )
    );
    await provider.send(
      initTx,
      [payer, lendingMarket],
      "max"
    );

    // TODO Create lending reserve

  });

  it("Creates reserve pool", async () => {
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
    
    reserveTokenMint = await Token.createMint(
      provider.connection,
      payer,
      owner.publicKey,
      null,
      2,
      TOKEN_PROGRAM_ID
    );
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
  });

  let userLpTokenAccount;

  it("Deposits to reserve pool", async () => {
    const depositAmount = 1000;

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
    await program.rpc.rebalance(
      {
        accounts: {
          reservePool: vaultStateAccount.publicKey,
          authority: vaultAuthority,
          lendingProgram: lendingProgram,
          poolDepositTokenAccount: vaultReserveTokenAccount,
          poolLpTokenAccount: splLpTokenAccount,
          lendingMarketReserveStateAccount: lendingMarketReserveStateAccount,
          lendingMarketLpMintAccount: lendingMarketLpMintAccount,
          lendingMarketDepositTokenAccount: lendingMarketDepositTokenAccount,
          lendingMarket: lendingMarket.publicKey,
          lendingMarketAuthority: lendingMarketAuthority,
          clock: SYSVAR_CLOCK_PUBKEY,
          tokenprogram: TOKEN_PROGRAM_ID,
        },
        signers: [vaultStateAccount],
      }
    );

    assert(true);
  });

  it("Rebalances", async () => {
  });

  it("Withdraws from reserve pool", async () => {
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