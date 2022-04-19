import { PublicKey } from "@solana/web3.js";

export * from "./client";
export * from "./types";
export * from "./adapters";

export { CastleLendingAggregator } from "./castle_lending_aggregator";

// TODO separate into dev and mainnet
export const PROGRAM_ID = new PublicKey(
    "Cast1eoVj8hwfKKRPji4cqX7WFgcnYz3um7TTgnaJKFn"
    // "4tSMVfVbnwZcDwZB1M1j27dx9hdjL72VR9GM8AykpAvK"
    //"E5xEvrNhrknmgGbRv8iDDqHsgqG1xeMEdfPjL8i4YEVo"
);
