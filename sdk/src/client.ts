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
  TransactionInstruction,
} from "@solana/web3.js";
import { CastleLendingAggregator } from "./castle_lending_aggregator";
import {
  JetAccounts,
  PortAccounts,
  SolendAccounts,
  StrategyType,
  VaultState,
} from "./types";

export class VaultClient {
  vaultId: PublicKey;
  vaultState: VaultState;
  program: anchor.Program<CastleLendingAggregator>;

  private constructor(
    program: anchor.Program<CastleLendingAggregator>,
    vaultId: PublicKey,
    vaultState: VaultState
  ) {
    this.program = program;
    this.vaultId = vaultId;
    this.vaultState = vaultState;
  }

  static async load(
    program: anchor.Program<CastleLendingAggregator>,
    vaultId: PublicKey
  ): Promise<VaultClient> {
    const vaultState = await program.account.vault.fetch(vaultId);
    return new VaultClient(program, vaultId, vaultState);
  }

  // TODO split into get Tx and send?
  static async initialize(
    program: anchor.Program<CastleLendingAggregator>,
    wallet: anchor.Wallet,
    reserveTokenMint: PublicKey,
    solendCollateralMint: PublicKey,
    portCollateralMint: PublicKey,
    jetCollateralMint: PublicKey,
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
        [vaultId.publicKey.toBuffer(), solendCollateralMint.toBuffer()],
        program.programId
      );

    const [vaultPortLpTokenAccount, portLpBump] =
      await PublicKey.findProgramAddress(
        [vaultId.publicKey.toBuffer(), portCollateralMint.toBuffer()],
        program.programId
      );

    const [vaultJetLpTokenAccount, jetLpBump] =
      await PublicKey.findProgramAddress(
        [vaultId.publicKey.toBuffer(), jetCollateralMint.toBuffer()],
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
          solendLpTokenMint: solendCollateralMint,
          portLpTokenMint: portCollateralMint,
          jetLpTokenMint: jetCollateralMint,
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
  getRefreshIx(
    solendAccounts: SolendAccounts,
    portAccounts: PortAccounts,
    jetAccounts: JetAccounts
  ): TransactionInstruction {
    return this.program.instruction.refresh({
      accounts: {
        vault: this.vaultId,
        vaultReserveToken: this.vaultState.vaultReserveToken,
        vaultSolendLpToken: this.vaultState.vaultSolendLpToken,
        vaultPortLpToken: this.vaultState.vaultPortLpToken,
        vaultJetLpToken: this.vaultState.vaultJetLpToken,
        solendProgram: solendAccounts.program,
        solendReserveState: solendAccounts.reserve,
        solendPyth: solendAccounts.pythPrice,
        solendSwitchboard: solendAccounts.switchboardFeed,
        portProgram: portAccounts.program,
        portReserveState: portAccounts.reserve,
        portOracle: portAccounts.oracle,
        jetProgram: jetAccounts.program,
        jetMarket: jetAccounts.market,
        jetMarketAuthority: jetAccounts.marketAuthority,
        jetReserveState: jetAccounts.reserve,
        jetFeeNoteVault: jetAccounts.feeNoteVault,
        jetDepositNoteMint: jetAccounts.depositNoteMint,
        jetPyth: jetAccounts.pythPrice,
        tokenProgram: TOKEN_PROGRAM_ID,
        clock: SYSVAR_CLOCK_PUBKEY,
      },
    });
  }

  async deposit(
    wallet: anchor.Wallet,
    amount: number,
    userReserveTokenAccount: PublicKey,
    solendAccounts: SolendAccounts,
    portAccounts: PortAccounts,
    jetAccounts: JetAccounts
  ): Promise<string> {
    let ixs = [this.getRefreshIx(solendAccounts, portAccounts, jetAccounts)];

    const userLpTokenAccount = await this.getUserLpTokenAccount(wallet);

    // Create account if it does not exist
    const userLpTokenAccountInfo =
      await this.program.provider.connection.getAccountInfo(userLpTokenAccount);
    if (userLpTokenAccountInfo == null) {
      ixs.unshift(
        createAta(wallet, this.vaultState.lpTokenMint, userLpTokenAccount)
      );
    }

    return await this.program.rpc.deposit(new anchor.BN(amount), {
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
      instructions: ixs,
    });
  }

  // Amount is currently denominated in lp tokens. Convert to reserve tokens?
  async withdraw(
    wallet: anchor.Wallet,
    amount: number,
    userLpTokenAccount: PublicKey,
    solendAccounts: SolendAccounts,
    portAccounts: PortAccounts,
    jetAccounts: JetAccounts
  ): Promise<string> {
    let ixs = [this.getRefreshIx(solendAccounts, portAccounts, jetAccounts)];

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
    return await this.program.rpc.withdraw(new anchor.BN(amount), {
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
      instructions: ixs,
    });
  }

  // TODO delete / consolidate these 4 fns?
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
