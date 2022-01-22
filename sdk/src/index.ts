import { PublicKey } from "@solana/web3.js";

export * from "./client";
export * from "./config";
export * from "./types";
export * from "./adapters";

export { CastleLendingAggregator } from "./castle_lending_aggregator";

export const PROGRAM_ID = new PublicKey(
  "6hSKFKsZvksTb4M7828LqWsquWnyatoRwgZbcpeyfWRb"
);
