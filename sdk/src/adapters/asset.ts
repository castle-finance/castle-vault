import { PublicKey } from "@solana/web3.js";

export abstract class Asset {
  abstract getApy(): Promise<number>;
  abstract getLpTokenAccountValue(address: PublicKey): Promise<number>;
}
