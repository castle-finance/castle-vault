import { PublicKey } from "@solana/web3.js";

export { VaultClient } from "./client";
export { MainnetConfig, DevnetConfig, LocalConfig } from "./config";

export const PROGRAM_ID = new PublicKey(
  "6hSKFKsZvksTb4M7828LqWsquWnyatoRwgZbcpeyfWRb"
);
