import * as anchor from "@project-serum/anchor";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
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

  deposit(amount: number): string {
    return amount.toString();
  }
}
