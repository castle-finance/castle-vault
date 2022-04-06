export type CastleLendingAggregator = {
    version: "1.4.0";
    name: "castle_lending_aggregator";
    instructions: [
        {
            name: "initialize";
            accounts: [
                {
                    name: "vault";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultAuthority";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "lpTokenMint";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultReserveToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultSolendLpToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultPortLpToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultJetLpToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "reserveTokenMint";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "solendLpTokenMint";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "portLpTokenMint";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "jetLpTokenMint";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "solendReserve";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "portReserve";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "jetReserve";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "feeReceiver";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "referralFeeReceiver";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "referralFeeOwner";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "payer";
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: "owner";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "systemProgram";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "tokenProgram";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "associatedTokenProgram";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "rent";
                    isMut: false;
                    isSigner: false;
                }
            ];
            args: [
                {
                    name: "bumps";
                    type: {
                        defined: "InitBumpSeeds";
                    };
                },
                {
                    name: "strategyType";
                    type: {
                        defined: "StrategyType";
                    };
                },
                {
                    name: "fees";
                    type: {
                        defined: "FeeArgs";
                    };
                },
                {
                    name: "poolSizeLimit";
                    type: "u64";
                }
            ];
        },
        {
            name: "deposit";
            accounts: [
                {
                    name: "vault";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultAuthority";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "vaultReserveToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "lpTokenMint";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "userReserveToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "userLpToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "userAuthority";
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: "tokenProgram";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "clock";
                    isMut: false;
                    isSigner: false;
                }
            ];
            args: [
                {
                    name: "reserveTokenAmount";
                    type: "u64";
                }
            ];
        },
        {
            name: "withdraw";
            accounts: [
                {
                    name: "vault";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultAuthority";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "vaultReserveToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "lpTokenMint";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "userLpToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "userReserveToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "userAuthority";
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: "tokenProgram";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "clock";
                    isMut: false;
                    isSigner: false;
                }
            ];
            args: [
                {
                    name: "lpTokenAmount";
                    type: "u64";
                }
            ];
        },
        {
            name: "rebalance";
            accounts: [
                {
                    name: "vault";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "solendReserve";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "portReserve";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "jetReserve";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "clock";
                    isMut: false;
                    isSigner: false;
                }
            ];
            args: [];
        },
        {
            name: "refresh";
            accounts: [
                {
                    name: "vault";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultAuthority";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "vaultReserveToken";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "vaultSolendLpToken";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "vaultPortLpToken";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "vaultJetLpToken";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "lpTokenMint";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "solendProgram";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "solendReserve";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "solendPyth";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "solendSwitchboard";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "portProgram";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "portReserve";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "portOracle";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "jetProgram";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "jetMarket";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "jetMarketAuthority";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "jetReserve";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "jetFeeNoteVault";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "jetDepositNoteMint";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "jetPyth";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "feeReceiver";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "referralFeeReceiver";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "tokenProgram";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "clock";
                    isMut: false;
                    isSigner: false;
                }
            ];
            args: [];
        },
        {
            name: "reconcileSolend";
            accounts: [
                {
                    name: "vault";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultAuthority";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "vaultReserveToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultSolendLpToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "solendProgram";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "solendMarketAuthority";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "solendMarket";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "solendReserve";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "solendLpMint";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "solendReserveToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "clock";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "tokenProgram";
                    isMut: false;
                    isSigner: false;
                }
            ];
            args: [
                {
                    name: "withdrawOption";
                    type: "u64";
                }
            ];
        },
        {
            name: "reconcilePort";
            accounts: [
                {
                    name: "vault";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultAuthority";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "vaultReserveToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultPortLpToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "portProgram";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "portMarketAuthority";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "portMarket";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "portReserve";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "portLpMint";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "portReserveToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "clock";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "tokenProgram";
                    isMut: false;
                    isSigner: false;
                }
            ];
            args: [
                {
                    name: "withdrawOption";
                    type: "u64";
                }
            ];
        },
        {
            name: "reconcileJet";
            accounts: [
                {
                    name: "vault";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultAuthority";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "vaultReserveToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultJetLpToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "jetProgram";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "jetMarket";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "jetMarketAuthority";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "jetReserve";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "jetReserveToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "jetLpMint";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "tokenProgram";
                    isMut: false;
                    isSigner: false;
                }
            ];
            args: [
                {
                    name: "withdrawOption";
                    type: "u64";
                }
            ];
        }
    ];
    accounts: [
        {
            name: "vault";
            type: {
                kind: "struct";
                fields: [
                    {
                        name: "owner";
                        type: "publicKey";
                    },
                    {
                        name: "vaultAuthority";
                        type: "publicKey";
                    },
                    {
                        name: "authoritySeed";
                        type: "publicKey";
                    },
                    {
                        name: "authorityBump";
                        type: {
                            array: ["u8", 1];
                        };
                    },
                    {
                        name: "solendReserve";
                        type: "publicKey";
                    },
                    {
                        name: "portReserve";
                        type: "publicKey";
                    },
                    {
                        name: "jetReserve";
                        type: "publicKey";
                    },
                    {
                        name: "vaultReserveToken";
                        type: "publicKey";
                    },
                    {
                        name: "vaultSolendLpToken";
                        type: "publicKey";
                    },
                    {
                        name: "vaultPortLpToken";
                        type: "publicKey";
                    },
                    {
                        name: "vaultJetLpToken";
                        type: "publicKey";
                    },
                    {
                        name: "lpTokenMint";
                        type: "publicKey";
                    },
                    {
                        name: "reserveTokenMint";
                        type: "publicKey";
                    },
                    {
                        name: "fees";
                        type: {
                            defined: "VaultFees";
                        };
                    },
                    {
                        name: "lastUpdate";
                        type: {
                            defined: "LastUpdate";
                        };
                    },
                    {
                        name: "totalValue";
                        type: "u64";
                    },
                    {
                        name: "poolSizeLimit";
                        type: "u64";
                    },
                    {
                        name: "allocations";
                        type: {
                            defined: "Allocations";
                        };
                    },
                    {
                        name: "strategyType";
                        type: {
                            defined: "StrategyType";
                        };
                    }
                ];
            };
        }
    ];
    types: [
        {
            name: "InitBumpSeeds";
            type: {
                kind: "struct";
                fields: [
                    {
                        name: "authority";
                        type: "u8";
                    },
                    {
                        name: "reserve";
                        type: "u8";
                    },
                    {
                        name: "lpMint";
                        type: "u8";
                    },
                    {
                        name: "solendLp";
                        type: "u8";
                    },
                    {
                        name: "portLp";
                        type: "u8";
                    },
                    {
                        name: "jetLp";
                        type: "u8";
                    }
                ];
            };
        },
        {
            name: "FeeArgs";
            type: {
                kind: "struct";
                fields: [
                    {
                        name: "feeCarryBps";
                        type: "u32";
                    },
                    {
                        name: "feeMgmtBps";
                        type: "u32";
                    },
                    {
                        name: "referralFeePct";
                        type: "u8";
                    }
                ];
            };
        },
        {
            name: "VaultFees";
            type: {
                kind: "struct";
                fields: [
                    {
                        name: "feeCarryBps";
                        type: "u32";
                    },
                    {
                        name: "feeMgmtBps";
                        type: "u32";
                    },
                    {
                        name: "referralFeePct";
                        type: "u8";
                    },
                    {
                        name: "feeReceiver";
                        type: "publicKey";
                    },
                    {
                        name: "referralFeeReceiver";
                        type: "publicKey";
                    }
                ];
            };
        },
        {
            name: "Allocations";
            type: {
                kind: "struct";
                fields: [
                    {
                        name: "solend";
                        type: {
                            defined: "Allocation";
                        };
                    },
                    {
                        name: "port";
                        type: {
                            defined: "Allocation";
                        };
                    },
                    {
                        name: "jet";
                        type: {
                            defined: "Allocation";
                        };
                    }
                ];
            };
        },
        {
            name: "Allocation";
            type: {
                kind: "struct";
                fields: [
                    {
                        name: "value";
                        type: "u64";
                    },
                    {
                        name: "lastUpdate";
                        type: {
                            defined: "LastUpdate";
                        };
                    }
                ];
            };
        },
        {
            name: "LastUpdate";
            type: {
                kind: "struct";
                fields: [
                    {
                        name: "slot";
                        type: "u64";
                    },
                    {
                        name: "stale";
                        type: "bool";
                    }
                ];
            };
        },
        {
            name: "StrategyType";
            type: {
                kind: "enum";
                variants: [
                    {
                        name: "MaxYield";
                    },
                    {
                        name: "EqualAllocation";
                    }
                ];
            };
        },
        {
            name: "Provider";
            type: {
                kind: "enum";
                variants: [
                    {
                        name: "Solend";
                    },
                    {
                        name: "Port";
                    },
                    {
                        name: "Jet";
                    }
                ];
            };
        }
    ];
    events: [
        {
            name: "RebalanceEvent";
            fields: [
                {
                    name: "solend";
                    type: "u64";
                    index: false;
                },
                {
                    name: "port";
                    type: "u64";
                    index: false;
                },
                {
                    name: "jet";
                    type: "u64";
                    index: false;
                }
            ];
        }
    ];
    errors: [
        {
            code: 6000;
            name: "MathError";
            msg: "failed to perform some math operation safely";
        },
        {
            code: 6001;
            name: "StrategyError";
            msg: "Failed to run the strategy";
        },
        {
            code: 6002;
            name: "VaultIsNotRefreshed";
            msg: "Vault is not refreshed";
        },
        {
            code: 6003;
            name: "AllocationIsNotUpdated";
            msg: "Allocation is not updated";
        },
        {
            code: 6004;
            name: "TryFromReserveError";
            msg: "Failed to convert from Reserve";
        },
        {
            code: 6005;
            name: "OverflowError";
            msg: "Failed to perform a math operation without an overflow";
        },
        {
            code: 6006;
            name: "ReferralFeeError";
            msg: "Failed to set referral fee share which is greater than 50%";
        },
        {
            code: 6007;
            name: "FeeBpsError";
            msg: "Failed to set fee BPS which is greater than 10000";
        },
        {
            code: 6008;
            name: "DepositCapError";
            msg: "Staking pool size limit is reached";
        }
    ];
};

export const IDL: CastleLendingAggregator = {
    version: "1.4.0",
    name: "castle_lending_aggregator",
    instructions: [
        {
            name: "initialize",
            accounts: [
                {
                    name: "vault",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultAuthority",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "lpTokenMint",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultReserveToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultSolendLpToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultPortLpToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultJetLpToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "reserveTokenMint",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "solendLpTokenMint",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "portLpTokenMint",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "jetLpTokenMint",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "solendReserve",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "portReserve",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "jetReserve",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "feeReceiver",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "referralFeeReceiver",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "referralFeeOwner",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "payer",
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: "owner",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "systemProgram",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "tokenProgram",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "associatedTokenProgram",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "rent",
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: "bumps",
                    type: {
                        defined: "InitBumpSeeds",
                    },
                },
                {
                    name: "strategyType",
                    type: {
                        defined: "StrategyType",
                    },
                },
                {
                    name: "fees",
                    type: {
                        defined: "FeeArgs",
                    },
                },
                {
                    name: "poolSizeLimit",
                    type: "u64",
                },
            ],
        },
        {
            name: "deposit",
            accounts: [
                {
                    name: "vault",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultAuthority",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "vaultReserveToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "lpTokenMint",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "userReserveToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "userLpToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "userAuthority",
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: "tokenProgram",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "clock",
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: "reserveTokenAmount",
                    type: "u64",
                },
            ],
        },
        {
            name: "withdraw",
            accounts: [
                {
                    name: "vault",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultAuthority",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "vaultReserveToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "lpTokenMint",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "userLpToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "userReserveToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "userAuthority",
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: "tokenProgram",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "clock",
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: "lpTokenAmount",
                    type: "u64",
                },
            ],
        },
        {
            name: "rebalance",
            accounts: [
                {
                    name: "vault",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "solendReserve",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "portReserve",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "jetReserve",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "clock",
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [],
        },
        {
            name: "refresh",
            accounts: [
                {
                    name: "vault",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultAuthority",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "vaultReserveToken",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "vaultSolendLpToken",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "vaultPortLpToken",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "vaultJetLpToken",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "lpTokenMint",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "solendProgram",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "solendReserve",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "solendPyth",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "solendSwitchboard",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "portProgram",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "portReserve",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "portOracle",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "jetProgram",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "jetMarket",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "jetMarketAuthority",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "jetReserve",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "jetFeeNoteVault",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "jetDepositNoteMint",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "jetPyth",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "feeReceiver",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "referralFeeReceiver",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "tokenProgram",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "clock",
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [],
        },
        {
            name: "reconcileSolend",
            accounts: [
                {
                    name: "vault",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultAuthority",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "vaultReserveToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultSolendLpToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "solendProgram",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "solendMarketAuthority",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "solendMarket",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "solendReserve",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "solendLpMint",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "solendReserveToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "clock",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "tokenProgram",
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: "withdrawOption",
                    type: "u64",
                },
            ],
        },
        {
            name: "reconcilePort",
            accounts: [
                {
                    name: "vault",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultAuthority",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "vaultReserveToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultPortLpToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "portProgram",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "portMarketAuthority",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "portMarket",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "portReserve",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "portLpMint",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "portReserveToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "clock",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "tokenProgram",
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: "withdrawOption",
                    type: "u64",
                },
            ],
        },
        {
            name: "reconcileJet",
            accounts: [
                {
                    name: "vault",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultAuthority",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "vaultReserveToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultJetLpToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "jetProgram",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "jetMarket",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "jetMarketAuthority",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "jetReserve",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "jetReserveToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "jetLpMint",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "tokenProgram",
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: "withdrawOption",
                    type: "u64",
                },
            ],
        },
    ],
    accounts: [
        {
            name: "vault",
            type: {
                kind: "struct",
                fields: [
                    {
                        name: "owner",
                        type: "publicKey",
                    },
                    {
                        name: "vaultAuthority",
                        type: "publicKey",
                    },
                    {
                        name: "authoritySeed",
                        type: "publicKey",
                    },
                    {
                        name: "authorityBump",
                        type: {
                            array: ["u8", 1],
                        },
                    },
                    {
                        name: "solendReserve",
                        type: "publicKey",
                    },
                    {
                        name: "portReserve",
                        type: "publicKey",
                    },
                    {
                        name: "jetReserve",
                        type: "publicKey",
                    },
                    {
                        name: "vaultReserveToken",
                        type: "publicKey",
                    },
                    {
                        name: "vaultSolendLpToken",
                        type: "publicKey",
                    },
                    {
                        name: "vaultPortLpToken",
                        type: "publicKey",
                    },
                    {
                        name: "vaultJetLpToken",
                        type: "publicKey",
                    },
                    {
                        name: "lpTokenMint",
                        type: "publicKey",
                    },
                    {
                        name: "reserveTokenMint",
                        type: "publicKey",
                    },
                    {
                        name: "fees",
                        type: {
                            defined: "VaultFees",
                        },
                    },
                    {
                        name: "lastUpdate",
                        type: {
                            defined: "LastUpdate",
                        },
                    },
                    {
                        name: "totalValue",
                        type: "u64",
                    },
                    {
                        name: "poolSizeLimit",
                        type: "u64",
                    },
                    {
                        name: "allocations",
                        type: {
                            defined: "Allocations",
                        },
                    },
                    {
                        name: "strategyType",
                        type: {
                            defined: "StrategyType",
                        },
                    },
                ],
            },
        },
    ],
    types: [
        {
            name: "InitBumpSeeds",
            type: {
                kind: "struct",
                fields: [
                    {
                        name: "authority",
                        type: "u8",
                    },
                    {
                        name: "reserve",
                        type: "u8",
                    },
                    {
                        name: "lpMint",
                        type: "u8",
                    },
                    {
                        name: "solendLp",
                        type: "u8",
                    },
                    {
                        name: "portLp",
                        type: "u8",
                    },
                    {
                        name: "jetLp",
                        type: "u8",
                    },
                ],
            },
        },
        {
            name: "FeeArgs",
            type: {
                kind: "struct",
                fields: [
                    {
                        name: "feeCarryBps",
                        type: "u32",
                    },
                    {
                        name: "feeMgmtBps",
                        type: "u32",
                    },
                    {
                        name: "referralFeePct",
                        type: "u8",
                    },
                ],
            },
        },
        {
            name: "VaultFees",
            type: {
                kind: "struct",
                fields: [
                    {
                        name: "feeCarryBps",
                        type: "u32",
                    },
                    {
                        name: "feeMgmtBps",
                        type: "u32",
                    },
                    {
                        name: "referralFeePct",
                        type: "u8",
                    },
                    {
                        name: "feeReceiver",
                        type: "publicKey",
                    },
                    {
                        name: "referralFeeReceiver",
                        type: "publicKey",
                    },
                ],
            },
        },
        {
            name: "Allocations",
            type: {
                kind: "struct",
                fields: [
                    {
                        name: "solend",
                        type: {
                            defined: "Allocation",
                        },
                    },
                    {
                        name: "port",
                        type: {
                            defined: "Allocation",
                        },
                    },
                    {
                        name: "jet",
                        type: {
                            defined: "Allocation",
                        },
                    },
                ],
            },
        },
        {
            name: "Allocation",
            type: {
                kind: "struct",
                fields: [
                    {
                        name: "value",
                        type: "u64",
                    },
                    {
                        name: "lastUpdate",
                        type: {
                            defined: "LastUpdate",
                        },
                    },
                ],
            },
        },
        {
            name: "LastUpdate",
            type: {
                kind: "struct",
                fields: [
                    {
                        name: "slot",
                        type: "u64",
                    },
                    {
                        name: "stale",
                        type: "bool",
                    },
                ],
            },
        },
        {
            name: "StrategyType",
            type: {
                kind: "enum",
                variants: [
                    {
                        name: "MaxYield",
                    },
                    {
                        name: "EqualAllocation",
                    },
                ],
            },
        },
        {
            name: "Provider",
            type: {
                kind: "enum",
                variants: [
                    {
                        name: "Solend",
                    },
                    {
                        name: "Port",
                    },
                    {
                        name: "Jet",
                    },
                ],
            },
        },
    ],
    events: [
        {
            name: "RebalanceEvent",
            fields: [
                {
                    name: "solend",
                    type: "u64",
                    index: false,
                },
                {
                    name: "port",
                    type: "u64",
                    index: false,
                },
                {
                    name: "jet",
                    type: "u64",
                    index: false,
                },
            ],
        },
    ],
    errors: [
        {
            code: 6000,
            name: "MathError",
            msg: "failed to perform some math operation safely",
        },
        {
            code: 6001,
            name: "StrategyError",
            msg: "Failed to run the strategy",
        },
        {
            code: 6002,
            name: "VaultIsNotRefreshed",
            msg: "Vault is not refreshed",
        },
        {
            code: 6003,
            name: "AllocationIsNotUpdated",
            msg: "Allocation is not updated",
        },
        {
            code: 6004,
            name: "TryFromReserveError",
            msg: "Failed to convert from Reserve",
        },
        {
            code: 6005,
            name: "OverflowError",
            msg: "Failed to perform a math operation without an overflow",
        },
        {
            code: 6006,
            name: "ReferralFeeError",
            msg: "Failed to set referral fee share which is greater than 50%",
        },
        {
            code: 6007,
            name: "FeeBpsError",
            msg: "Failed to set fee BPS which is greater than 10000",
        },
        {
            code: 6008,
            name: "DepositCapError",
            msg: "Staking pool size limit is reached",
        },
    ],
};
