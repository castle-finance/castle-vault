import { Cluster, clusterApiUrl, PublicKey } from "@solana/web3.js";
import { ClusterMap, Envs, ProgramIdMap } from "./types";

export * from "./client";
export * from "./types";
export * from "./adapters";

export { CastleLendingAggregator } from "./castle_lending_aggregator";

export const PROGRAM_IDS: ProgramIdMap = {
    [Envs.devnetStaging]: new PublicKey(
        "4tSMVfVbnwZcDwZB1M1j27dx9hdjL72VR9GM8AykpAvK"
    ),
    [Envs.devnetParity]: new PublicKey(
        "Cast1eoVj8hwfKKRPji4cqX7WFgcnYz3um7TTgnaJKFn"
    ),
    [Envs.mainnet]: new PublicKey(
        "Cast1eoVj8hwfKKRPji4cqX7WFgcnYz3um7TTgnaJKFn"
    ),
};

export const CLUSTER_MAP: ClusterMap = {
    [Envs.devnetStaging]: "devnet" as Cluster,
    [Envs.devnetParity]: "devnet" as Cluster,
    [Envs.mainnet]: "mainnet-beta" as Cluster,
};
