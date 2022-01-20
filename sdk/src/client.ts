import * as anchor from "@project-serum/anchor";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_CLOCK_PUBKEY,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import { CastleLendingAggregator } from "./castle_lending_aggregator";
import { StrategyType, VaultState } from "./types";

export class VaultClient {
  vaultId: PublicKey;
  vaultState: VaultState;
  program: anchor.Program;

  /**
   * Create a new Castle Vault client object
   * @param vaultId
   * @returns
   */
  constructor(program: anchor.Program, vaultId: PublicKey) {
    this.vaultId = vaultId;
    return;
  }

  static async initialize(
    program: anchor.Program<CastleLendingAggregator>,
    wallet: anchor.Wallet,
    reserveTokenMint: PublicKey,
    solendCollateralMint: PublicKey,
    portCollateralMint: PublicKey,
    jetCollateralMint: PublicKey,
    strategyType: StrategyType
  ): Promise<[PublicKey, VaultState]> {
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

    return [vaultId.publicKey, vaultState];
  }

  deposit(amount: number): string {
    return amount.toString();
  }
}
