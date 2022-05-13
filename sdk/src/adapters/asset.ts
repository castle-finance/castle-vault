import { PublicKey, TransactionInstruction } from "@solana/web3.js";
import Big from "big.js";
import { Vault } from "../types";
import { CastleVault } from "../idl";
import * as anchor from "@project-serum/anchor";

export abstract class Asset {
    abstract getApy(): Promise<Big>;
    abstract getLpTokenAccountValue(vaultState: Vault): Promise<Big>;
    abstract getDepositedAmount(): Promise<Big>;
    abstract getBorrowedAmount(): Promise<Big>;

    abstract getRefreshIx(
        program: anchor.Program<CastleVault>,
        vaultId: PublicKey,
        vaultState: Vault
    ): TransactionInstruction;

    abstract getReconcileIx(
        program: anchor.Program<CastleVault>,
        vaultId: PublicKey,
        vaultState: Vault,
        withdrawOption?: anchor.BN
    ): TransactionInstruction;
}
