import { PublicKey } from "@solana/web3.js";
import { ProgramIdMap } from "./types";

import {
    Clusters,
    DeploymentEnvs,
    DeploymentEnvToClusters,
} from "@castlefinance/vault-core";

export * from "./client";
export * from "./types";
export * from "./adapters";

export { CastleVault } from "./idl";

export const PROGRAM_IDS: ProgramIdMap = {
    [DeploymentEnvs.devnetStaging]: new PublicKey(
        "4tSMVfVbnwZcDwZB1M1j27dx9hdjL72VR9GM8AykpAvK"
    ),
    [DeploymentEnvs.devnetParity]: new PublicKey(
        "Cast1eoVj8hwfKKRPji4cqX7WFgcnYz3um7TTgnaJKFn"
    ),
    [DeploymentEnvs.mainnet]: new PublicKey(
        "Cast1eoVj8hwfKKRPji4cqX7WFgcnYz3um7TTgnaJKFn"
    ),
};

export const CLUSTER_MAP: DeploymentEnvToClusters = {
    [DeploymentEnvs.devnetStaging]: Clusters.devnet,
    [DeploymentEnvs.devnetParity]: Clusters.devnet,
    [DeploymentEnvs.mainnet]: Clusters.mainnetBeta,
};
