import { PublicKey, TransactionInstruction } from "@solana/web3.js";
import Big from "big.js";
import Vault from "../types";

export abstract class Asset {
    abstract getApy(): Promise<Big>;
    abstract getLpTokenAccountValue(address: PublicKey): Promise<Big>;
    abstract getDepositedAmount(): Promise<Big>;
    abstract getBorrowedAmount(): Promise<Big>;

    abstract getRefreshIx(
        program: anchor.Program<CastleVault>,
        vaultId: PublicKey,
        vaultState: Vault
    ): TransactionInstruction;
}
