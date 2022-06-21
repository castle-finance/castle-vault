export type CastleVault = {
    version: "0.0.0";
    name: "castle_vault";
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
                    name: "reserveTokenMint";
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
                    name: "config";
                    type: {
                        defined: "VaultConfigArg";
                    };
                }
            ];
        },
        {
            name: "initializeRewardAccount";
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
                    name: "vaultPortObligation";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultPortStakeAccount";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultPortRewardToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "portRewardTokenMint";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "portStakingPool";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "portStakeProgram";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "portLendProgram";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "portLendingMarket";
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
                    name: "clock";
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
                    name: "obligationBump";
                    type: "u8";
                },
                {
                    name: "stakeBump";
                    type: "u8";
                },
                {
                    name: "rewardBump";
                    type: "u8";
                }
            ];
        },
        {
            name: "initializeJet";
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
                    name: "vaultJetLpToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "jetLpTokenMint";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "jetReserve";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "owner";
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: "payer";
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: "tokenProgram";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "systemProgram";
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
                    name: "bump";
                    type: "u8";
                }
            ];
        },
        {
            name: "initializePort";
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
                    name: "vaultPortLpToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "portLpTokenMint";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "portReserve";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "owner";
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: "payer";
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: "tokenProgram";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "systemProgram";
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
                    name: "bump";
                    type: "u8";
                }
            ];
        },
        {
            name: "initializeSolend";
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
                    name: "vaultSolendLpToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "solendLpTokenMint";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "solendReserve";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "owner";
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: "payer";
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: "tokenProgram";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "systemProgram";
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
                    name: "bump";
                    type: "u8";
                }
            ];
        },
        {
            name: "updateHaltFlags";
            accounts: [
                {
                    name: "vault";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "owner";
                    isMut: false;
                    isSigner: true;
                }
            ];
            args: [
                {
                    name: "flags";
                    type: "u16";
                }
            ];
        },
        {
            name: "updateYieldSourceFlags";
            accounts: [
                {
                    name: "vault";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "owner";
                    isMut: false;
                    isSigner: true;
                }
            ];
            args: [
                {
                    name: "flags";
                    type: "u16";
                }
            ];
        },
        {
            name: "updateConfig";
            accounts: [
                {
                    name: "vault";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "owner";
                    isMut: false;
                    isSigner: true;
                }
            ];
            args: [
                {
                    name: "newConfig";
                    type: {
                        defined: "VaultConfigArg";
                    };
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
            args: [
                {
                    name: "proposedWeights";
                    type: {
                        defined: "StrategyWeightsArg";
                    };
                }
            ];
        },
        {
            name: "refreshSolend";
            accounts: [
                {
                    name: "vault";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultSolendLpToken";
                    isMut: false;
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
                    name: "clock";
                    isMut: false;
                    isSigner: false;
                }
            ];
            args: [];
        },
        {
            name: "refreshPort";
            accounts: [
                {
                    name: "vault";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultPortLpToken";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "portLendProgram";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "portReserve";
                    isMut: true;
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
            name: "refreshJet";
            accounts: [
                {
                    name: "vault";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultJetLpToken";
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
            name: "consolidateRefresh";
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
                    name: "vaultSolendLpToken";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "lpTokenMint";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "tokenProgram";
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
                    name: "vaultPortObligation";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultPortStakeAccount";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "vaultPortRewardToken";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "portStakingPool";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "portLendProgram";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "portStakeProgram";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "portStakingRewardPool";
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: "portStakingAuthority";
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: "portLpTokenAccount";
                    isMut: true;
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
                        name: "version";
                        type: {
                            array: ["u8", 3];
                        };
                    },
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
                        name: "feeReceiver";
                        type: "publicKey";
                    },
                    {
                        name: "referralFeeReceiver";
                        type: "publicKey";
                    },
                    {
                        name: "haltFlags";
                        type: "u16";
                    },
                    {
                        name: "yieldSourceFlags";
                        type: "u16";
                    },
                    {
                        name: "value";
                        type: {
                            defined: "SlotTrackedValue";
                        };
                    },
                    {
                        name: "targetAllocations";
                        type: {
                            defined: "Allocations";
                        };
                    },
                    {
                        name: "config";
                        type: {
                            defined: "VaultConfig";
                        };
                    },
                    {
                        name: "actualAllocations";
                        type: {
                            defined: "Allocations";
                        };
                    },
                    {
                        name: "vaultPortStakeAccount";
                        type: "publicKey";
                    },
                    {
                        name: "vaultPortRewardToken";
                        type: "publicKey";
                    },
                    {
                        name: "vaultPortObligation";
                        type: "publicKey";
                    },
                    {
                        name: "reserved";
                        type: {
                            array: ["u32", 4];
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
                    }
                ];
            };
        },
        {
            name: "VaultConfigArg";
            type: {
                kind: "struct";
                fields: [
                    {
                        name: "depositCap";
                        type: "u64";
                    },
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
                        name: "allocationCapPct";
                        type: "u8";
                    },
                    {
                        name: "rebalanceMode";
                        type: {
                            defined: "RebalanceMode";
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
        },
        {
            name: "StrategyWeightsArg";
            type: {
                kind: "struct";
                fields: [
                    {
                        name: "solend";
                        type: "u16";
                    },
                    {
                        name: "port";
                        type: "u16";
                    },
                    {
                        name: "jet";
                        type: "u16";
                    }
                ];
            };
        },
        {
            name: "VaultConfig";
            type: {
                kind: "struct";
                fields: [
                    {
                        name: "depositCap";
                        type: "u64";
                    },
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
                        name: "allocationCapPct";
                        type: "u8";
                    },
                    {
                        name: "rebalanceMode";
                        type: {
                            defined: "RebalanceMode";
                        };
                    },
                    {
                        name: "strategyType";
                        type: {
                            defined: "StrategyType";
                        };
                    },
                    {
                        name: "padding";
                        type: {
                            array: ["u32", 3];
                        };
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
                            defined: "SlotTrackedValue";
                        };
                    },
                    {
                        name: "port";
                        type: {
                            defined: "SlotTrackedValue";
                        };
                    },
                    {
                        name: "jet";
                        type: {
                            defined: "SlotTrackedValue";
                        };
                    }
                ];
            };
        },
        {
            name: "SlotTrackedValue";
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
                    },
                    {
                        name: "padding";
                        type: {
                            array: ["u8", 7];
                        };
                    }
                ];
            };
        },
        {
            name: "RebalanceMode";
            type: {
                kind: "enum";
                variants: [
                    {
                        name: "Calculator";
                    },
                    {
                        name: "ProofChecker";
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
        }
    ];
    events: [
        {
            name: "DepositEvent";
            fields: [
                {
                    name: "vault";
                    type: "publicKey";
                    index: false;
                },
                {
                    name: "user";
                    type: "publicKey";
                    index: false;
                },
                {
                    name: "amount";
                    type: "u64";
                    index: false;
                }
            ];
        },
        {
            name: "RebalanceEvent";
            fields: [
                {
                    name: "vault";
                    type: "publicKey";
                    index: false;
                }
            ];
        },
        {
            name: "RebalanceDataEvent";
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
        },
        {
            name: "WithdrawEvent";
            fields: [
                {
                    name: "vault";
                    type: "publicKey";
                    index: false;
                },
                {
                    name: "user";
                    type: "publicKey";
                    index: false;
                },
                {
                    name: "amount";
                    type: "u64";
                    index: false;
                }
            ];
        }
    ];
    errors: [
        {
            code: 300;
            name: "MathError";
            msg: "failed to perform some math operation safely";
        },
        {
            code: 301;
            name: "StrategyError";
            msg: "Failed to run the strategy";
        },
        {
            code: 302;
            name: "VaultIsNotRefreshed";
            msg: "Vault is not refreshed";
        },
        {
            code: 303;
            name: "AllocationIsNotUpdated";
            msg: "Allocation is not updated";
        },
        {
            code: 304;
            name: "TryFromReserveError";
            msg: "Failed to convert from Reserve";
        },
        {
            code: 305;
            name: "OverflowError";
            msg: "Failed to perform a math operation without an overflow";
        },
        {
            code: 306;
            name: "InvalidReferralFeeConfig";
            msg: "Referral fee split cannot set to be over 50%";
        },
        {
            code: 307;
            name: "InvalidFeeConfig";
            msg: "Fees cannot be set to over 100%";
        },
        {
            code: 308;
            name: "InvalidProposedWeights";
            msg: "Proposed weights do not meet the required constraints";
        },
        {
            code: 309;
            name: "RebalanceProofCheckFailed";
            msg: "Proposed weights failed proof check";
        },
        {
            code: 310;
            name: "DepositCapError";
            msg: "Vault size limit is reached";
        },
        {
            code: 311;
            name: "InvalidAccount";
            msg: "Account passed in is not valid";
        },
        {
            code: 312;
            name: "InsufficientAccounts";
            msg: "Insufficient number of accounts for a given operation";
        },
        {
            code: 313;
            name: "InvalidAllocationCap";
            msg: "Allocation cap cannot set to under 1/(number of assets) or over 100%";
        },
        {
            code: 314;
            name: "InvalidVaultFlags";
            msg: "Bits passed in do not result in valid vault flags";
        },
        {
            code: 315;
            name: "HaltedVault";
            msg: "Vault is halted";
        }
    ];
};

export const IDL: CastleVault = {
    version: "0.0.0",
    name: "castle_vault",
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
                    name: "reserveTokenMint",
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
                    name: "config",
                    type: {
                        defined: "VaultConfigArg",
                    },
                },
            ],
        },
        {
            name: "initializeRewardAccount",
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
                    name: "vaultPortObligation",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultPortStakeAccount",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultPortRewardToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "portRewardTokenMint",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "portStakingPool",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "portStakeProgram",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "portLendProgram",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "portLendingMarket",
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
                    name: "clock",
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
                    name: "obligationBump",
                    type: "u8",
                },
                {
                    name: "stakeBump",
                    type: "u8",
                },
                {
                    name: "rewardBump",
                    type: "u8",
                },
            ],
        },
        {
            name: "initializeJet",
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
                    name: "vaultJetLpToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "jetLpTokenMint",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "jetReserve",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "owner",
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: "payer",
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: "tokenProgram",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "systemProgram",
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
                    name: "bump",
                    type: "u8",
                },
            ],
        },
        {
            name: "initializePort",
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
                    name: "vaultPortLpToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "portLpTokenMint",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "portReserve",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "owner",
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: "payer",
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: "tokenProgram",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "systemProgram",
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
                    name: "bump",
                    type: "u8",
                },
            ],
        },
        {
            name: "initializeSolend",
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
                    name: "vaultSolendLpToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "solendLpTokenMint",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "solendReserve",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "owner",
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: "payer",
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: "tokenProgram",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "systemProgram",
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
                    name: "bump",
                    type: "u8",
                },
            ],
        },
        {
            name: "updateHaltFlags",
            accounts: [
                {
                    name: "vault",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "owner",
                    isMut: false,
                    isSigner: true,
                },
            ],
            args: [
                {
                    name: "flags",
                    type: "u16",
                },
            ],
        },
        {
            name: "updateYieldSourceFlags",
            accounts: [
                {
                    name: "vault",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "owner",
                    isMut: false,
                    isSigner: true,
                },
            ],
            args: [
                {
                    name: "flags",
                    type: "u16",
                },
            ],
        },
        {
            name: "updateConfig",
            accounts: [
                {
                    name: "vault",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "owner",
                    isMut: false,
                    isSigner: true,
                },
            ],
            args: [
                {
                    name: "newConfig",
                    type: {
                        defined: "VaultConfigArg",
                    },
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
            args: [
                {
                    name: "proposedWeights",
                    type: {
                        defined: "StrategyWeightsArg",
                    },
                },
            ],
        },
        {
            name: "refreshSolend",
            accounts: [
                {
                    name: "vault",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultSolendLpToken",
                    isMut: false,
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
                    name: "clock",
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [],
        },
        {
            name: "refreshPort",
            accounts: [
                {
                    name: "vault",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultPortLpToken",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "portLendProgram",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "portReserve",
                    isMut: true,
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
            name: "refreshJet",
            accounts: [
                {
                    name: "vault",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultJetLpToken",
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
            name: "consolidateRefresh",
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
                    name: "vaultSolendLpToken",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "lpTokenMint",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "tokenProgram",
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
                    name: "vaultPortObligation",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultPortStakeAccount",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "vaultPortRewardToken",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "portStakingPool",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "portLendProgram",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "portStakeProgram",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "portStakingRewardPool",
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: "portStakingAuthority",
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: "portLpTokenAccount",
                    isMut: true,
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
                        name: "version",
                        type: {
                            array: ["u8", 3],
                        },
                    },
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
                        name: "feeReceiver",
                        type: "publicKey",
                    },
                    {
                        name: "referralFeeReceiver",
                        type: "publicKey",
                    },
                    {
                        name: "haltFlags",
                        type: "u16",
                    },
                    {
                        name: "yieldSourceFlags",
                        type: "u16",
                    },
                    {
                        name: "value",
                        type: {
                            defined: "SlotTrackedValue",
                        },
                    },
                    {
                        name: "targetAllocations",
                        type: {
                            defined: "Allocations",
                        },
                    },
                    {
                        name: "config",
                        type: {
                            defined: "VaultConfig",
                        },
                    },
                    {
                        name: "actualAllocations",
                        type: {
                            defined: "Allocations",
                        },
                    },
                    {
                        name: "vaultPortStakeAccount",
                        type: "publicKey",
                    },
                    {
                        name: "vaultPortRewardToken",
                        type: "publicKey",
                    },
                    {
                        name: "vaultPortObligation",
                        type: "publicKey",
                    },
                    {
                        name: "reserved",
                        type: {
                            array: ["u32", 4],
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
                ],
            },
        },
        {
            name: "VaultConfigArg",
            type: {
                kind: "struct",
                fields: [
                    {
                        name: "depositCap",
                        type: "u64",
                    },
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
                        name: "allocationCapPct",
                        type: "u8",
                    },
                    {
                        name: "rebalanceMode",
                        type: {
                            defined: "RebalanceMode",
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
        {
            name: "StrategyWeightsArg",
            type: {
                kind: "struct",
                fields: [
                    {
                        name: "solend",
                        type: "u16",
                    },
                    {
                        name: "port",
                        type: "u16",
                    },
                    {
                        name: "jet",
                        type: "u16",
                    },
                ],
            },
        },
        {
            name: "VaultConfig",
            type: {
                kind: "struct",
                fields: [
                    {
                        name: "depositCap",
                        type: "u64",
                    },
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
                        name: "allocationCapPct",
                        type: "u8",
                    },
                    {
                        name: "rebalanceMode",
                        type: {
                            defined: "RebalanceMode",
                        },
                    },
                    {
                        name: "strategyType",
                        type: {
                            defined: "StrategyType",
                        },
                    },
                    {
                        name: "padding",
                        type: {
                            array: ["u32", 3],
                        },
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
                            defined: "SlotTrackedValue",
                        },
                    },
                    {
                        name: "port",
                        type: {
                            defined: "SlotTrackedValue",
                        },
                    },
                    {
                        name: "jet",
                        type: {
                            defined: "SlotTrackedValue",
                        },
                    },
                ],
            },
        },
        {
            name: "SlotTrackedValue",
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
                    {
                        name: "padding",
                        type: {
                            array: ["u8", 7],
                        },
                    },
                ],
            },
        },
        {
            name: "RebalanceMode",
            type: {
                kind: "enum",
                variants: [
                    {
                        name: "Calculator",
                    },
                    {
                        name: "ProofChecker",
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
    ],
    events: [
        {
            name: "DepositEvent",
            fields: [
                {
                    name: "vault",
                    type: "publicKey",
                    index: false,
                },
                {
                    name: "user",
                    type: "publicKey",
                    index: false,
                },
                {
                    name: "amount",
                    type: "u64",
                    index: false,
                },
            ],
        },
        {
            name: "RebalanceEvent",
            fields: [
                {
                    name: "vault",
                    type: "publicKey",
                    index: false,
                },
            ],
        },
        {
            name: "RebalanceDataEvent",
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
        {
            name: "WithdrawEvent",
            fields: [
                {
                    name: "vault",
                    type: "publicKey",
                    index: false,
                },
                {
                    name: "user",
                    type: "publicKey",
                    index: false,
                },
                {
                    name: "amount",
                    type: "u64",
                    index: false,
                },
            ],
        },
    ],
    errors: [
        {
            code: 300,
            name: "MathError",
            msg: "failed to perform some math operation safely",
        },
        {
            code: 301,
            name: "StrategyError",
            msg: "Failed to run the strategy",
        },
        {
            code: 302,
            name: "VaultIsNotRefreshed",
            msg: "Vault is not refreshed",
        },
        {
            code: 303,
            name: "AllocationIsNotUpdated",
            msg: "Allocation is not updated",
        },
        {
            code: 304,
            name: "TryFromReserveError",
            msg: "Failed to convert from Reserve",
        },
        {
            code: 305,
            name: "OverflowError",
            msg: "Failed to perform a math operation without an overflow",
        },
        {
            code: 306,
            name: "InvalidReferralFeeConfig",
            msg: "Referral fee split cannot set to be over 50%",
        },
        {
            code: 307,
            name: "InvalidFeeConfig",
            msg: "Fees cannot be set to over 100%",
        },
        {
            code: 308,
            name: "InvalidProposedWeights",
            msg: "Proposed weights do not meet the required constraints",
        },
        {
            code: 309,
            name: "RebalanceProofCheckFailed",
            msg: "Proposed weights failed proof check",
        },
        {
            code: 310,
            name: "DepositCapError",
            msg: "Vault size limit is reached",
        },
        {
            code: 311,
            name: "InvalidAccount",
            msg: "Account passed in is not valid",
        },
        {
            code: 312,
            name: "InsufficientAccounts",
            msg: "Insufficient number of accounts for a given operation",
        },
        {
            code: 313,
            name: "InvalidAllocationCap",
            msg: "Allocation cap cannot set to under 1/(number of assets) or over 100%",
        },
        {
            code: 314,
            name: "InvalidVaultFlags",
            msg: "Bits passed in do not result in valid vault flags",
        },
        {
            code: 315,
            name: "HaltedVault",
            msg: "Vault is halted",
        },
    ],
};
