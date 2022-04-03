import { BN } from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";

// TODO change to enum or mapping
export type StrategyType = { equalAllocation: {} } | { maxYield: {} };
export type RebalanceMode = { calculator: {} } | { proofChecker: {} };

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
    rebalanceMode: any;
    owner: PublicKey;
    fees: VaultFees;
}

export interface VaultFees {
    feeReceiver: PublicKey;
    referralFeeReceiver: PublicKey;
    feeCarryBps: number;
    feeMgmtBps: number;
    referralFeePct: number;
}

export interface FeeArgs {
    feeCarryBps: number;
    feeMgmtBps: number;
    referralFeePct: number;
    referralFeeOwner: PublicKey;
}

export interface ProposedWeightsBps {
    solend: number;
    port: number;
    jet: number;
}

export interface RebalanceEvent {
    solend: BN;
    port: BN;
    jet: BN;
}
