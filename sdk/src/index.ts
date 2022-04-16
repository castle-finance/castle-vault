import { PublicKey } from "@solana/web3.js";

export * from "./client";
export * from "./types";
export * from "./adapters";

export { CastleLendingAggregator } from "./castle_lending_aggregator";

// TODO separate into dev and mainnet
export const PROGRAM_ID = new PublicKey(
    "4tSMVfVbnwZcDwZB1M1j27dx9hdjL72VR9GM8AykpAvK"
);
