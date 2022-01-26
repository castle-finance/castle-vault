import Big from "big.js";
import {
  Cluster,
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_CLOCK_PUBKEY,
  SYSVAR_RENT_PUBKEY,
  Transaction,
  TransactionInstruction,
  TransactionSignature,
} from "@solana/web3.js";
import {
  AccountInfo,
  AccountLayout,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  MintInfo,
  NATIVE_MINT,
  Token,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import * as anchor from "@project-serum/anchor";
import { SendTxRequest } from "@project-serum/anchor/dist/cjs/provider";

import { PROGRAM_ID } from ".";
import { CastleLendingAggregator } from "./castle_lending_aggregator";
import { PortReserveAsset, SolendReserveAsset, JetReserveAsset } from "./adapters";
import { StrategyType, Vault } from "./types";

export class VaultClient {
  private constructor(
    public program: anchor.Program<CastleLendingAggregator>,
    public vaultId: PublicKey,
    public vaultState: Vault,
    public solend: SolendReserveAsset,
    public port: PortReserveAsset,
    public jet: JetReserveAsset
  ) {}

  // TODO add function to change wallet

  static async load(
    provider: anchor.Provider,
    cluster: Cluster,
    reserveMint: PublicKey,
    vaultId: PublicKey
  ): Promise<VaultClient> {
    const program = (await anchor.Program.at(
      PROGRAM_ID,
      provider
    )) as anchor.Program<CastleLendingAggregator>;
    const vaultState = await program.account.vault.fetch(vaultId);

    const solend = await SolendReserveAsset.load(provider, cluster, reserveMint);
    const port = await PortReserveAsset.load(provider, cluster, reserveMint);
    const jet = await JetReserveAsset.load(provider, cluster, reserveMint);

    return new VaultClient(program, vaultId, vaultState, solend, port, jet);
  }

  private async reload() {
    this.vaultState = await this.program.account.vault.fetch(this.vaultId);
    // TODO reload underlying asset data also?
  }

  static async initialize(
    program: anchor.Program<CastleLendingAggregator>,
    wallet: anchor.Wallet,
    reserveTokenMint: PublicKey,
    solend: SolendReserveAsset,
    port: PortReserveAsset,
    jet: JetReserveAsset,
    strategyType: StrategyType
  ): Promise<VaultClient> {
    const vaultId = Keypair.generate();

    const [vaultAuthority, authorityBump] = await PublicKey.findProgramAddress(
      [vaultId.publicKey.toBuffer(), anchor.utils.bytes.utf8.encode("authority")],
      program.programId
    );
    // send sol to vault authority to pay for jet deposit account init
    const amount = await program.provider.connection.getMinimumBalanceForRentExemption(
      AccountLayout.span
    );
    const tx = new Transaction().add(
      SystemProgram.transfer({
        fromPubkey: wallet.publicKey,
        toPubkey: vaultAuthority,
        lamports: amount,
      })
    );
    await program.provider.send(tx, [wallet.payer]);

    const [vaultReserveTokenAccount, reserveBump] = await PublicKey.findProgramAddress(
      [vaultId.publicKey.toBuffer(), reserveTokenMint.toBuffer()],
      program.programId
    );

    const [vaultSolendLpTokenAccount, solendLpBump] =
      await PublicKey.findProgramAddress(
        [vaultId.publicKey.toBuffer(), solend.accounts.collateralMint.toBuffer()],
        program.programId
      );

    const [vaultPortLpTokenAccount, portLpBump] = await PublicKey.findProgramAddress(
      [vaultId.publicKey.toBuffer(), port.accounts.collateralMint.toBuffer()],
      program.programId
    );

    const [vaultJetLpTokenAccount, jetLpBump] = await PublicKey.findProgramAddress(
      [
        anchor.utils.bytes.utf8.encode("deposits"),
        jet.accounts.reserve.toBuffer(),
        vaultAuthority.toBuffer(),
      ],
      jet.accounts.program
    );

    const [lpTokenMint, lpTokenMintBump] = await PublicKey.findProgramAddress(
      [vaultId.publicKey.toBuffer(), anchor.utils.bytes.utf8.encode("lp_mint")],
      program.programId
    );

    await program.rpc.initialize(
      {
        authority: authorityBump,
        reserve: reserveBump,
        lpMint: lpTokenMintBump,
        solendLp: solendLpBump,
        portLp: portLpBump,
        jetLp: jetLpBump,
      },
      strategyType,
      {
        accounts: {
          vault: vaultId.publicKey,
          vaultAuthority: vaultAuthority,
          lpTokenMint: lpTokenMint,
          vaultReserveToken: vaultReserveTokenAccount,
          vaultSolendLpToken: vaultSolendLpTokenAccount,
          vaultPortLpToken: vaultPortLpTokenAccount,
          vaultJetLpToken: vaultJetLpTokenAccount,
          jetProgram: jet.accounts.program,
          jetMarket: jet.accounts.market,
          jetMarketAuthority: jet.accounts.marketAuthority,
          jetReserveState: jet.accounts.reserve,
          reserveTokenMint: reserveTokenMint,
          solendLpTokenMint: solend.accounts.collateralMint,
          portLpTokenMint: port.accounts.collateralMint,
          jetLpTokenMint: jet.accounts.depositNoteMint,
          payer: wallet.payer.publicKey,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY,
          clock: SYSVAR_CLOCK_PUBKEY,
        },
        signers: [vaultId, wallet.payer],
        instructions: [await program.account.vault.createInstruction(vaultId)],
      }
    );
    const vaultState = await program.account.vault.fetch(vaultId.publicKey);

    return new VaultClient(program, vaultId.publicKey, vaultState, solend, port, jet);
  }

  private getRefreshIx(): TransactionInstruction {
    return this.program.instruction.refresh({
      accounts: {
        vault: this.vaultId,
        vaultReserveToken: this.vaultState.vaultReserveToken,
        vaultSolendLpToken: this.vaultState.vaultSolendLpToken,
        vaultPortLpToken: this.vaultState.vaultPortLpToken,
        vaultJetLpToken: this.vaultState.vaultJetLpToken,
        solendProgram: this.solend.accounts.program,
        solendReserveState: this.solend.accounts.reserve,
        solendPyth: this.solend.accounts.pythPrice,
        solendSwitchboard: this.solend.accounts.switchboardFeed,
        portProgram: this.port.accounts.program,
        portReserveState: this.port.accounts.reserve,
        portOracle: this.port.accounts.oracle,
        jetProgram: this.jet.accounts.program,
        jetMarket: this.jet.accounts.market,
        jetMarketAuthority: this.jet.accounts.marketAuthority,
        jetReserveState: this.jet.accounts.reserve,
        jetFeeNoteVault: this.jet.accounts.feeNoteVault,
        jetDepositNoteMint: this.jet.accounts.depositNoteMint,
        jetPyth: this.jet.accounts.pythPrice,
        tokenProgram: TOKEN_PROGRAM_ID,
        clock: SYSVAR_CLOCK_PUBKEY,
      },
    });
  }

  /**
   *
   * @param wallet
   * @param lamports amount depositing
   * @returns
   */
  private async getWrappedSolIxs(
    wallet: anchor.Wallet,
    lamports: number = 0
  ): Promise<WrapSolIxResponse> {
    const userReserveKeypair = Keypair.generate();
    const userReserveTokenAccount = userReserveKeypair.publicKey;

    const rent = await Token.getMinBalanceRentForExemptAccount(
      this.program.provider.connection
    );
    return {
      openIxs: [
        SystemProgram.createAccount({
          fromPubkey: wallet.publicKey,
          newAccountPubkey: userReserveTokenAccount,
          programId: TOKEN_PROGRAM_ID,
          space: AccountLayout.span,
          lamports: lamports + rent,
        }),
        Token.createInitAccountInstruction(
          TOKEN_PROGRAM_ID,
          NATIVE_MINT,
          userReserveTokenAccount,
          wallet.publicKey
        ),
      ],
      closeIx: Token.createCloseAccountInstruction(
        TOKEN_PROGRAM_ID,
        userReserveTokenAccount,
        wallet.publicKey,
        wallet.publicKey,
        []
      ),
      keyPair: userReserveKeypair,
    };
  }

  /**
   *
   * TODO refactor to be more clear
   *
   * @param wallet
   * @param amount
   * @param userReserveTokenAccount
   * @returns
   */
  async deposit(
    wallet: anchor.Wallet,
    amount: number,
    userReserveTokenAccount: PublicKey
  ): Promise<TransactionSignature[]> {
    const depositTx = new Transaction();

    let wrappedSolIxResponse: WrapSolIxResponse;
    if (this.vaultState.reserveTokenMint.equals(NATIVE_MINT)) {
      wrappedSolIxResponse = await this.getWrappedSolIxs(wallet, amount);
      depositTx.add(...wrappedSolIxResponse.openIxs);
      userReserveTokenAccount = wrappedSolIxResponse.keyPair.publicKey;
    }

    const userLpTokenAccount = await this.getUserLpTokenAccount(wallet.publicKey);
    const userLpTokenAccountInfo =
      await this.program.provider.connection.getAccountInfo(userLpTokenAccount);

    // Create account if it does not exist
    let createLpAcctTx: Transaction;
    if (userLpTokenAccountInfo == null) {
      createLpAcctTx = new Transaction().add(
        createAta(wallet, this.vaultState.lpTokenMint, userLpTokenAccount)
      );
    }

    depositTx.add(this.getRefreshIx());
    depositTx.add(
      this.program.instruction.deposit(new anchor.BN(amount), {
        accounts: {
          vault: this.vaultId,
          vaultAuthority: this.vaultState.vaultAuthority,
          vaultReserveToken: this.vaultState.vaultReserveToken,
          lpTokenMint: this.vaultState.lpTokenMint,
          userReserveToken: userReserveTokenAccount,
          userLpToken: userLpTokenAccount,
          userAuthority: wallet.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
      })
    );

    let txs: SendTxRequest[] = [];
    if (createLpAcctTx != null) {
      txs.push({ tx: createLpAcctTx, signers: [] });
    }

    if (wrappedSolIxResponse != null) {
      depositTx.add(wrappedSolIxResponse.closeIx);
      txs.push({ tx: depositTx, signers: [wrappedSolIxResponse.keyPair] });
    } else {
      txs.push({ tx: depositTx, signers: [] });
    }

    return await this.program.provider.sendAll(txs);
  }

  // TODO derive lp token account from wallet?
  async withdraw(
    wallet: anchor.Wallet,
    amount: number,
    userLpTokenAccount: PublicKey
  ): Promise<TransactionSignature[]> {
    //console.debug("Withdrawing %d reserve tokens", amount);

    let txs: SendTxRequest[] = [];

    // Withdraw from lending markets if not enough reserves in vault
    // Has to be 2 transactions because of size limits
    const vaultReserveTokenAccountInfo = await this.getReserveTokenAccountInfo(
      this.vaultState.vaultReserveToken
    );
    const vaultReserveAmount = vaultReserveTokenAccountInfo.amount.toNumber();

    //console.debug("Reserve tokens in vault: %d", vaultReserveAmount);

    if (vaultReserveAmount < amount) {
      const rrTx = new Transaction();
      rrTx.add(this.getRefreshIx());
      for (let ix of await this.getRebalanceAndReconcileIxs(amount)) {
        rrTx.add(ix);
      }
      txs.push({ tx: rrTx, signers: [] });
    }

    const withdrawTx = new Transaction();
    let userReserveTokenAccount: PublicKey;
    let wrappedSolIxResponse: WrapSolIxResponse;
    if (this.vaultState.reserveTokenMint.equals(NATIVE_MINT)) {
      wrappedSolIxResponse = await this.getWrappedSolIxs(wallet);
      withdrawTx.add(...wrappedSolIxResponse.openIxs);
      userReserveTokenAccount = wrappedSolIxResponse.keyPair.publicKey;
    } else {
      userReserveTokenAccount = await this.getUserReserveTokenAccount(wallet.publicKey);
      // Create reserve token account to withdraw into if it does not exist
      const userReserveTokenAccountInfo =
        await this.program.provider.connection.getAccountInfo(userReserveTokenAccount);
      if (userReserveTokenAccountInfo == null) {
        withdrawTx.add(
          createAta(wallet, this.vaultState.reserveTokenMint, userReserveTokenAccount)
        );
      }
    }

    withdrawTx.add(this.getRefreshIx());
    // Convert from reserve tokens to LP tokens
    const exchangeRate = await this.getLpExchangeRate();
    const convertedAmount = amount / exchangeRate;

    //console.debug("Converted to %d lp tokens", convertedAmount);

    withdrawTx.add(
      this.program.instruction.withdraw(new anchor.BN(convertedAmount), {
        accounts: {
          vault: this.vaultId,
          vaultAuthority: this.vaultState.vaultAuthority,
          userAuthority: wallet.publicKey,
          userLpToken: userLpTokenAccount,
          userReserveToken: userReserveTokenAccount,
          vaultReserveToken: this.vaultState.vaultReserveToken,
          vaultLpMint: this.vaultState.lpTokenMint,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
      })
    );

    if (wrappedSolIxResponse != null) {
      withdrawTx.add(wrappedSolIxResponse.closeIx);
      txs.push({ tx: withdrawTx, signers: [wrappedSolIxResponse.keyPair] });
    } else {
      txs.push({ tx: withdrawTx, signers: [] });
    }
    return this.program.provider.sendAll(txs);
  }

  async rebalance(): Promise<TransactionSignature> {
    const tx = new Transaction();
    tx.add(this.getRefreshIx());
    for (let ix of await this.getRebalanceAndReconcileIxs()) {
      tx.add(ix);
    }
    return this.program.provider.send(tx);
  }

  private async getRebalanceAndReconcileIxs(
    withdrawAmountOption: number = 0
  ): Promise<TransactionInstruction[]> {
    // TODO split off into its own public function so that heartbeat can use it to figure out when to actually send txs
    // Simulate transaction to get new allocations
    const newAllocations = (
      await this.program.simulate.rebalance(new anchor.BN(withdrawAmountOption), {
        accounts: {
          vault: this.vaultId,
          vaultReserveToken: this.vaultState.vaultReserveToken,
          vaultSolendLpToken: this.vaultState.vaultSolendLpToken,
          vaultPortLpToken: this.vaultState.vaultPortLpToken,
          vaultJetLpToken: this.vaultState.vaultJetLpToken,
          solendReserveState: this.solend.accounts.reserve,
          portReserveState: this.port.accounts.reserve,
          jetReserveState: this.jet.accounts.reserve,
        },
        instructions: [this.getRefreshIx()],
      })
    ).events[0].data;

    // Sort ixs in ascending order of outflows
    const diffAndReconcileIxs: [Big, TransactionInstruction][] = [
      //[
      //  new Big(newAllocations.solend.toString()).sub(
      //    await this.solend.getLpTokenAccountValue(this.vaultState.vaultSolendLpToken)
      //  ),
      //  this.getReconcileSolendIx(),
      //],
      [
        new Big(newAllocations.port.toString()).sub(
          await this.port.getLpTokenAccountValue(this.vaultState.vaultPortLpToken)
        ),
        this.getReconcilePortIx(),
      ],
      [
        new Big(newAllocations.jet.toNumber()).sub(
          await this.jet.getLpTokenAccountValue(this.vaultState.vaultJetLpToken)
        ),
        this.getReconcileJetIx(),
      ],
    ];
    const reconcileIxs = diffAndReconcileIxs
      .sort((a, b) => a[0].sub(b[0]).toNumber())
      .map((val, _) => val[1]);

    const rebalanceIx = this.program.instruction.rebalance(
      new anchor.BN(withdrawAmountOption),
      {
        accounts: {
          vault: this.vaultId,
          vaultReserveToken: this.vaultState.vaultReserveToken,
          vaultSolendLpToken: this.vaultState.vaultSolendLpToken,
          vaultPortLpToken: this.vaultState.vaultPortLpToken,
          vaultJetLpToken: this.vaultState.vaultJetLpToken,
          solendReserveState: this.solend.accounts.reserve,
          portReserveState: this.port.accounts.reserve,
          jetReserveState: this.jet.accounts.reserve,
        },
      }
    );
    return [rebalanceIx, ...reconcileIxs];
  }

  private getReconcilePortIx(): TransactionInstruction {
    return this.program.instruction.reconcilePort({
      accounts: {
        vault: this.vaultId,
        vaultAuthority: this.vaultState.vaultAuthority,
        vaultReserveToken: this.vaultState.vaultReserveToken,
        vaultPortLpToken: this.vaultState.vaultPortLpToken,
        portProgram: this.port.accounts.program,
        portMarketAuthority: this.port.accounts.marketAuthority,
        portMarket: this.port.accounts.market,
        portReserveState: this.port.accounts.reserve,
        portLpMint: this.port.accounts.collateralMint,
        portReserveToken: this.port.accounts.liquiditySupply,
        clock: SYSVAR_CLOCK_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });
  }

  private getReconcileJetIx(): TransactionInstruction {
    return this.program.instruction.reconcileJet({
      accounts: {
        vault: this.vaultId,
        vaultAuthority: this.vaultState.vaultAuthority,
        vaultReserveToken: this.vaultState.vaultReserveToken,
        vaultJetLpToken: this.vaultState.vaultJetLpToken,
        jetProgram: this.jet.accounts.program,
        jetMarket: this.jet.accounts.market,
        jetMarketAuthority: this.jet.accounts.marketAuthority,
        jetReserveState: this.jet.accounts.reserve,
        jetReserveToken: this.jet.accounts.liquiditySupply,
        jetLpMint: this.jet.accounts.depositNoteMint,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });
  }

  private getReconcileSolendIx(): TransactionInstruction {
    return this.program.instruction.reconcileSolend({
      accounts: {
        vault: this.vaultId,
        vaultAuthority: this.vaultState.vaultAuthority,
        vaultReserveToken: this.vaultState.vaultReserveToken,
        vaultSolendLpToken: this.vaultState.vaultSolendLpToken,
        solendProgram: this.solend.accounts.program,
        solendMarketAuthority: this.solend.accounts.marketAuthority,
        solendMarket: this.solend.accounts.market,
        solendReserveState: this.solend.accounts.reserve,
        solendLpMint: this.solend.accounts.collateralMint,
        solendReserveToken: this.solend.accounts.liquiditySupply,
        clock: SYSVAR_CLOCK_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });
  }

  async getApy(): Promise<Big> {
    // Weighted average of APYs by allocation
    const assetApysAndValues: [Big, Big][] = [
      [
        await this.solend.getApy(),
        await this.solend.getLpTokenAccountValue(this.vaultState.vaultSolendLpToken),
      ],
      [
        await this.port.getApy(),
        await this.port.getLpTokenAccountValue(this.vaultState.vaultPortLpToken),
      ],
      [
        await this.jet.getApy(),
        await this.jet.getLpTokenAccountValue(this.vaultState.vaultJetLpToken),
      ],
    ];
    const [valueSum, weightSum] = assetApysAndValues.reduce(
      ([valueSum, weightSum], [value, weight]) => [
        weight.mul(value).add(valueSum),
        weightSum.add(weight),
      ],
      [new Big(0), new Big(0)]
    );
    if (weightSum.eq(new Big(0))) {
      return new Big(0);
    } else {
      return valueSum.div(weightSum);
    }
  }

  // Denominated in reserve tokens per LP token
  async getLpExchangeRate(): Promise<number> {
    const totalValue = await this.getTotalValue();
    //console.debug("total vault value: %d", totalValue);
    const lpTokenMintInfo = await this.getLpTokenMintInfo();
    const lpTokenSupply = lpTokenMintInfo.supply.toNumber();
    //console.debug("lp token supply: %d", lpTokenSupply);
    if (lpTokenSupply == 0 || totalValue == 0) {
      return 1;
    } else {
      return totalValue / lpTokenSupply;
    }
  }

  /**
   * Gets the total value stored in the vault, denominated in reserve tokens
   *
   * Note: this assumes that the total value in the vault state is up to date
   * May need to calculate from ts client instead
   *
   * @returns
   */
  async getTotalValue(): Promise<number> {
    await this.reload();
    return this.vaultState.totalValue.toNumber();
  }

  async getUserValue(address: PublicKey): Promise<number> {
    const userLpTokenAccount = await this.getUserLpTokenAccount(address);
    const userLpTokenAccountInfo = await this.getLpTokenAccountInfo(userLpTokenAccount);
    if (userLpTokenAccountInfo == null) {
      return 0;
    } else {
      const userLpTokenAmount = userLpTokenAccountInfo.amount.toNumber();
      const exchangeRate = await this.getLpExchangeRate();
      return userLpTokenAmount * exchangeRate;
    }
  }

  async getUserReserveTokenAccount(address: PublicKey): Promise<PublicKey> {
    return await Token.getAssociatedTokenAddress(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      this.vaultState.reserveTokenMint,
      address
    );
  }

  async getReserveTokenAccountInfo(address: PublicKey): Promise<AccountInfo> {
    const reserveToken = new Token(
      this.program.provider.connection,
      this.vaultState.reserveTokenMint,
      TOKEN_PROGRAM_ID,
      Keypair.generate() // dummy since we don't need to send txs
    );
    return reserveToken.getAccountInfo(address);
  }

  async getUserLpTokenAccount(address: PublicKey): Promise<PublicKey> {
    return await Token.getAssociatedTokenAddress(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      this.vaultState.lpTokenMint,
      address
    );
  }

  async getLpTokenAccountInfo(address: PublicKey): Promise<AccountInfo> {
    const lpToken = new Token(
      this.program.provider.connection,
      this.vaultState.lpTokenMint,
      TOKEN_PROGRAM_ID,
      Keypair.generate() // dummy since we don't need to send txs
    );
    return lpToken.getAccountInfo(address);
  }

  async getLpTokenMintInfo(): Promise<MintInfo> {
    const lpToken = new Token(
      this.program.provider.connection,
      this.vaultState.lpTokenMint,
      TOKEN_PROGRAM_ID,
      Keypair.generate() // dummy since we don't need to send txs
    );
    return lpToken.getMintInfo();
  }
}

const createAta = (
  wallet: anchor.Wallet,
  mint: PublicKey,
  address: PublicKey
): TransactionInstruction => {
  return Token.createAssociatedTokenAccountInstruction(
    ASSOCIATED_TOKEN_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    mint,
    address,
    wallet.publicKey,
    wallet.publicKey
  );
};

interface WrapSolIxResponse {
  openIxs: [TransactionInstruction, TransactionInstruction];
  closeIx: TransactionInstruction;
  keyPair: Keypair;
}
