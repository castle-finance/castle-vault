import { BN } from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";

import { DeploymentEnvs } from "@castlefinance/vault-core";

export type ProgramIdMap = {
    [E in DeploymentEnvs]: PublicKey;
};

export interface Vault {
    version: number;
    owner: PublicKey;
    vaultAuthority: PublicKey;
    authoritySeed: PublicKey;
    authorityBump: number[];
    solendReserve: PublicKey;
    portReserve: PublicKey;
    jetReserve: PublicKey;
    vaultReserveToken: PublicKey;
    vaultSolendLpToken: PublicKey;
    vaultPortLpToken: PublicKey;
    vaultJetLpToken: PublicKey;
    lpTokenMint: PublicKey;
    reserveTokenMint: PublicKey;
    feeReceiver: PublicKey;
    referralFeeReceiver: PublicKey;
    value: SlotTrackedValue;
    allocations: Allocations;
    config: VaultConfig;
}

export interface VaultConfig {
    depositCap: BN;
    feeCarryBps: number;
    feeMgmtBps: number;
    referralFeePct: number;
    allocationCapPct: number;
    rebalanceMode: { [x: string]: {} };
    strategyType: { [x: string]: {} };
}

export interface LastUpdate {
    slot: BN;
    stale: any;
}

export interface SlotTrackedValue {
    value: BN;
    lastUpdate: LastUpdate;
}

export interface Allocations {
    solend: SlotTrackedValue;
    port: SlotTrackedValue;
    jet: SlotTrackedValue;
}

export interface ProposedWeightsBps {
    solend: number;
    port: number;
    jet: number;
}

export interface RebalanceDataEvent {
    solend: BN;
    port: BN;
    jet: BN;
}
