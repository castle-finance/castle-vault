import * as anchor from "@project-serum/anchor";
import {
  AccountInfo,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  MintInfo,
  Token,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_CLOCK_PUBKEY,
  SYSVAR_RENT_PUBKEY,
  Transaction,
  TransactionInstruction,
  TransactionSignature,
} from "@solana/web3.js";

import { CastleLendingAggregator } from "./castle_lending_aggregator";
import {
  PortReserveAsset,
  SolendReserveAsset,
  JetReserveAsset,
} from "./adapters";
import { StrategyType, Vault } from "./types";

export class VaultClient {
  vaultId: PublicKey;
  vaultState: Vault;
  program: anchor.Program<CastleLendingAggregator>;
  jet: JetReserveAsset;
  solend: SolendReserveAsset;
  port: PortReserveAsset;

  private constructor(
    program: anchor.Program<CastleLendingAggregator>,
    vaultId: PublicKey,
    vault: Vault
  ) {
    this.program = program;
    this.vaultId = vaultId;
    this.vaultState = vault;
  }

  static async load(
    program: anchor.Program<CastleLendingAggregator>,
    vaultId: PublicKey
  ): Promise<VaultClient> {
    const vaultState = await program.account.vault.fetch(vaultId);
    return new VaultClient(program, vaultId, vaultState);
  }

  private async reload() {
    this.vaultState = await this.program.account.vault.fetch(this.vaultId);
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
      [
        vaultId.publicKey.toBuffer(),
        anchor.utils.bytes.utf8.encode("authority"),
      ],
      program.programId
    );

    const [vaultReserveTokenAccount, reserveBump] =
      await PublicKey.findProgramAddress(
        [vaultId.publicKey.toBuffer(), reserveTokenMint.toBuffer()],
        program.programId
      );

    const [vaultSolendLpTokenAccount, solendLpBump] =
      await PublicKey.findProgramAddress(
        [
          vaultId.publicKey.toBuffer(),
          solend.accounts.collateralMint.toBuffer(),
        ],
        program.programId
      );

    const [vaultPortLpTokenAccount, portLpBump] =
      await PublicKey.findProgramAddress(
        [vaultId.publicKey.toBuffer(), port.accounts.collateralMint.toBuffer()],
        program.programId
      );

    const [vaultJetLpTokenAccount, jetLpBump] =
      await PublicKey.findProgramAddress(
        [vaultId.publicKey.toBuffer(), jet.accounts.depositNoteMint.toBuffer()],
        program.programId
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

    return new VaultClient(program, vaultId.publicKey, vaultState);
  }

  // TODO store params as class vars so that caller doesn't have to keep track of them?
  // Adapter pattern?
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

  async deposit(
    wallet: anchor.Wallet,
    amount: number,
    userReserveTokenAccount: PublicKey
  ): Promise<TransactionSignature> {
    let ixs = [this.getRefreshIx()];

    const userLpTokenAccount = await this.getUserLpTokenAccount(wallet);

    // Create account if it does not exist
    const userLpTokenAccountInfo =
      await this.program.provider.connection.getAccountInfo(userLpTokenAccount);
    if (userLpTokenAccountInfo == null) {
      ixs.unshift(
        createAta(wallet, this.vaultState.lpTokenMint, userLpTokenAccount)
      );
    }

    return this.program.rpc.deposit(new anchor.BN(amount), {
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
      signers: [wallet.payer],
      instructions: ixs,
    });
  }

  // Amount is currently denominated in lp tokens. Convert to reserve tokens?
  async withdraw(
    wallet: anchor.Wallet,
    amount: number,
    userLpTokenAccount: PublicKey
  ): Promise<TransactionSignature> {
    let ixs = [this.getRefreshIx()];

    const userReserveTokenAccount = await this.getUserReserveTokenAccount(
      wallet
    );

    // Create account if it does not exist
    const userReserveTokenAccountInfo =
      await this.program.provider.connection.getAccountInfo(
        userReserveTokenAccount
      );
    if (userReserveTokenAccountInfo == null) {
      ixs.unshift(
        createAta(
          wallet,
          this.vaultState.reserveTokenMint,
          userReserveTokenAccount
        )
      );
    }

    // Withdraw from lending markets if not enough reserves in vault
    const vaultReserveTokenAccountInfo = await this.getReserveTokenAccountInfo(
      this.vaultState.vaultReserveToken
    );
    const vaultReserveAmount = vaultReserveTokenAccountInfo.amount.toNumber();
    if (vaultReserveAmount < amount) {
      ixs = [...ixs, ...(await this.getRebalanceAndReconcileIxs(amount))];
    }

    return this.program.rpc.withdraw(new anchor.BN(amount), {
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
      signers: [wallet.payer],
      instructions: ixs,
    });
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
    // Simulate transaction to get new allocations
    const newAllocations = (
      await this.program.simulate.rebalance(
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
          instructions: [this.getRefreshIx()],
        }
      )
    ).events[0].data;

    // Sort ixs in ascending order of outflows
    const diffAndReconcileIxs: [number, TransactionInstruction][] = [
      [
        newAllocations.solend.toNumber() -
          (await this.solend.getLpTokenAccountValue(
            this.vaultState.vaultSolendLpToken
          )),
        this.getReconcileSolendIx(),
      ],
      [
        newAllocations.port.toNumber() -
          (await this.port.getLpTokenAccountValue(
            this.vaultState.vaultPortLpToken
          )),
        this.getReconcilePortIx(),
      ],
      [
        newAllocations.jet.toNumber() -
          (await this.jet.getLpTokenAccountValue(
            this.vaultState.vaultJetLpToken
          )),
        this.getReconcileJetIx(),
      ],
    ];
    const reconcileIxs = diffAndReconcileIxs
      .sort((a, b) => a[0] - b[0])
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

  async getApy(): Promise<number> {
    // Weighted average of APYs by allocation
    const assetApysAndValues: [number, number][] = [
      [
        await this.solend.getApy(),
        await this.solend.getLpTokenAccountValue(
          this.vaultState.vaultSolendLpToken
        ),
      ],
      [
        await this.port.getApy(),
        await this.port.getLpTokenAccountValue(
          this.vaultState.vaultPortLpToken
        ),
      ],
      [
        await this.jet.getApy(),
        await this.jet.getLpTokenAccountValue(this.vaultState.vaultJetLpToken),
      ],
    ];
    const [valueSum, weightSum] = assetApysAndValues.reduce(
      ([valueSum, weightSum], [value, weight]) => [
        valueSum + value * weight,
        weightSum + weight,
      ],
      [0, 0]
    );
    return valueSum / weightSum;
  }

  // Denominated in reserve tokens per LP token
  async getLpExchangeRate(): Promise<number> {
    const totalValue = await this.getTotalValue();
    const lpTokenMintInfo = await this.getLpTokenMintInfo();
    return totalValue / lpTokenMintInfo.supply.toNumber();
  }

  async getTotalValue(): Promise<number> {
    await this.reload();
    return this.vaultState.totalValue.toNumber();
  }

  async getUserValue(wallet: anchor.Wallet): Promise<number> {
    const userLpTokenAccount = await this.getUserLpTokenAccount(wallet);
    const userLpTokenAmount = (
      await this.getLpTokenAccountInfo(userLpTokenAccount)
    ).amount.toNumber();
    return userLpTokenAmount * (await this.getLpExchangeRate());
  }

  async getUserReserveTokenAccount(wallet: anchor.Wallet): Promise<PublicKey> {
    return await Token.getAssociatedTokenAddress(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      this.vaultState.reserveTokenMint,
      wallet.publicKey
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

  async getUserLpTokenAccount(wallet: anchor.Wallet): Promise<PublicKey> {
    return await Token.getAssociatedTokenAddress(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      this.vaultState.lpTokenMint,
      wallet.publicKey
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
