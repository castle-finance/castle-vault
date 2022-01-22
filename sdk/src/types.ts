import { BN } from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";

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
}
