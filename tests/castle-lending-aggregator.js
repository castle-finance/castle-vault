const anchor = require("@project-serum/anchor");
const assert = require("assert");
const { TOKEN_PROGRAM_ID , Token} = require("@solana/spl-token");
const { PublicKey } = require("@solana/web3.js");


describe("reserve-pool", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.CastleLendingAggregator;

  let authority;
  let bumpSeed;
  let poolTokenMint;
  let poolTokenAccount;
  let tokenMint;
  let tokenAccount;

  const owner = anchor.web3.Keypair.generate();
  const poolAccount = anchor.web3.Keypair.generate();
  const payer = anchor.web3.Keypair.generate();

  it("Creates reserve pool", async () => {
    const sig  = await provider.connection.requestAirdrop(payer.publicKey, 1000000000);
    await provider.connection.confirmTransaction(sig, "singleGossip");

    [authority, bumpSeed] = await PublicKey.findProgramAddress(
      [poolAccount.publicKey.toBuffer()],
      program.programId,
    )
    // Create pool mint
    poolTokenMint = await Token.createMint(
      provider.connection,
      payer,
      authority,
      null,
      2,
      TOKEN_PROGRAM_ID
    );
    // Create pool account
    poolTokenAccount = await poolTokenMint.createAccount(owner.publicKey);
    
    tokenMint = await Token.createMint(
      provider.connection,
      payer,
      owner.publicKey,
      null,
      2,
      TOKEN_PROGRAM_ID
    );
    tokenAccount = await tokenMint.createAccount(authority);

    await tokenMint.mintTo(tokenAccount, owner, [], 1000);

    await program.rpc.initializePool(
      {
        accounts: {
          authority: authority,
          reservePool: poolAccount.publicKey,
          poolMint: poolTokenMint.publicKey,
          token: tokenAccount,
          destination: poolTokenAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [poolAccount],
        instructions: [await program.account.reservePool.createInstruction(poolAccount)]
      }
    );

    const actualPoolAccount = await program.account.reservePool.fetch(poolAccount.publicKey);
    assert(actualPoolAccount.tokenProgramId.equals(TOKEN_PROGRAM_ID));
    assert(actualPoolAccount.tokenAccount.equals(tokenAccount));
    assert(actualPoolAccount.tokenMint.equals(tokenMint.publicKey));
    assert(actualPoolAccount.poolMint.equals(poolTokenMint.publicKey));
  });

  it("Deposits to reserve pool", async () => {

  });

  it("Withdraws from reserve pool", async () => {

  });
});