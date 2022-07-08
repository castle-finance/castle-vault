import { PublicKey, TransactionInstruction } from "@solana/web3.js";
import * as anchor from "@project-serum/anchor";

import { CastleVault } from "../idl";
import { Vault } from "../types";
import { Rate, TokenAmount } from "../utils";

export abstract class LendingMarket {
    abstract getApy(): Promise<Rate>;
    abstract getLpTokenAccountValue(vaultState: Vault): Promise<TokenAmount>;
    abstract getDepositedAmount(): Promise<TokenAmount>;
    abstract getBorrowedAmount(): Promise<TokenAmount>;

    abstract getRefreshIx(
        program: anchor.Program<CastleVault>,
        vaultId: PublicKey,
        vaultState: Vault
    ): Promise<TransactionInstruction>;

    abstract getReconcileIx(
        program: anchor.Program<CastleVault>,
        vaultId: PublicKey,
        vaultState: Vault,
        withdrawOption?: anchor.BN
    ): Promise<TransactionInstruction>;

    abstract getInitializeIx(
        program: anchor.Program<CastleVault>,
        vaultId: PublicKey,
        vaultAuthority: PublicKey,
        wallet: PublicKey,
        owner: PublicKey
    ): Promise<TransactionInstruction>;
}
