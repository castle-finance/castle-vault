import { BN } from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";

/**
 * Canonical source-of-truth of supported strategies
 */
export enum StrategyTypes {
    maxYield = "maxYield",
    equalAllocation = "equalAllocation",
}

/**
 * Define a type-union based on the enum using TypeScript Template Literal Type
 * Use when typing a function that accepts a StrategyType
 * parameter without needing to use the enum when invoking it
 * ENHANCEMENT - rename this to `StrategyType` when the `StrategyType` defined below is renamed
 */
export type StrategyTypeLiteral = `${StrategyTypes}`;

// ENHANCEMENT - change to be a type-safe mapping of { [key in StrategyTypes]: { ... } }
export type StrategyType = { equalAllocation: {} } | { maxYield: {} };

export interface LastUpdate {
    slot: BN;
    stale: any;
}

export interface Allocation {
    value: BN;
    lastUpdate: LastUpdate;
}

export interface Allocations {
    solend: Allocation;
    port: Allocation;
    jet: Allocation;
}

export interface Vault {
    authorityBump: number[];
    authoritySeed: PublicKey;
    lastUpdate: LastUpdate;
    lpTokenMint: PublicKey;
    reserveTokenMint: PublicKey;
    totalValue: BN;
    vaultAuthority: PublicKey;
    vaultJetLpToken: PublicKey;
    vaultPortLpToken: PublicKey;
    vaultReserveToken: PublicKey;
    vaultSolendLpToken: PublicKey;
    allocations: Allocations;
    strategyType: any;
    owner: PublicKey;
    feeReceiver: PublicKey;
    feeCarryBps: number;
    feeMgmtBps: number;
}

export interface RebalanceEvent {
    solend: BN;
    port: BN;
    jet: BN;
}
