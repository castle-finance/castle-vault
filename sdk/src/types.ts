import * as anchor from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";

export type StrategyType = { equalAllocation: {} } | { maxYield: {} };

export interface LastUpdate {
  slot: anchor.BN;
  stale: any;
}

export interface Allocation {
  value: anchor.BN;
  lastUpdate: LastUpdate;
}

export interface Allocations {
  solend: Allocation;
  port: Allocation;
  jet: Allocation;
}

export interface VaultState {
  authorityBump: number[];
  authoritySeed: PublicKey;
  lastUpdate: LastUpdate;
  lpTokenMint: PublicKey;
  reserveTokenMint: PublicKey;
  totalValue: anchor.BN;
  vaultAuthority: PublicKey;
  vaultJetLpToken: PublicKey;
  vaultPortLpToken: PublicKey;
  vaultReserveToken: PublicKey;
  vaultSolendLpToken: PublicKey;
  allocations: Allocations;
  strategyType: any;
}
