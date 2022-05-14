import { PublicKey } from "@solana/web3.js";
import { Rate, TokenAmount } from "../utils";

export abstract class LendingMarket {
    abstract getApy(): Promise<Rate>;
    abstract getLpTokenAccountValue(address: PublicKey): Promise<TokenAmount>;
    abstract getDepositedAmount(): Promise<TokenAmount>;
    abstract getBorrowedAmount(): Promise<TokenAmount>;
}
