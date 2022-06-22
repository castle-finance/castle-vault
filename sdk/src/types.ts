import { BN } from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";

import { DeploymentEnvs } from "@castlefinance/vault-core";

export type ProgramIdMap = {
    [E in DeploymentEnvs]: PublicKey;
};

export interface Vault {
    version: number[];
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
    haltFlags: number;
    yieldSourceFlags: number;
    value: SlotTrackedValue;
    targetAllocations: Allocations;
    config: VaultConfig;
    actualAllocations: Allocations;
    vaultPortStakeAccount: PublicKey;
    vaultPortRewardToken: PublicKey;
    vaultPortObligation: PublicKey;
    lpTokenSupply: BN;
}

export interface VaultConfig {
    depositCap?: BN;
    feeCarryBps?: number;
    feeMgmtBps?: number;
    referralFeePct?: number;
    allocationCapPct?: number;
    rebalanceMode?: { [x: string]: {} };
    strategyType?: { [x: string]: {} };
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

export enum VaultFlags {
    HaltReconciles = 1 << 0,
    HaltRefreshes = 1 << 1,
    HaltDepositsWithdraws = 1 << 2,
    HaltAll = HaltReconciles | HaltRefreshes | HaltDepositsWithdraws,
}

export enum YieldSourceFlags {
    Solend = 1 << 0,
    Port = 1 << 1,
    Jet = 1 << 2,
}
