import { assert } from "chai";
import { Keypair, PublicKey } from "@solana/web3.js";
import { StakeAccount } from "@castlefinance/port-sdk";
import { TOKEN_PROGRAM_ID, Token } from "@solana/spl-token";
import { RebalanceModes, StrategyTypes } from "@castlefinance/vault-core";

import { YieldSrcSelector, TestFixture } from "./test_fixture";

describe("Stake Port LP token and claim reward", () => {
    const fixture = new TestFixture();

    function testPortRewardClaiming(subReward: boolean) {
        const depositQty = 1024502;

        before(async () => {
            await fixture.depositToVault(depositQty);
        });

        it("Stake port LP token when rebalancing", async () => {
            await fixture.performRebalance({
                solend: 0,
                port: 10000,
                jet: 0,
            });

            const maxDiffAllowed = 1;
            const totalValue = await fixture.getVaultTotalValue();
            assert.isAtMost(
                Math.abs(totalValue - Math.floor(depositQty)),
                maxDiffAllowed
            );
            const portValue = (
                await fixture.vaultClient.getVaultPortLpTokenAccountValue()
            ).lamports.toNumber();
            assert.isAtMost(
                Math.abs(portValue - Math.floor(depositQty)),
                maxDiffAllowed
            );

            const stakingAccountRaw =
                await fixture.provider.connection.getAccountInfo(
                    new PublicKey(fixture.port.accounts.vaultPortStakeAccount)
                );
            const stakingAccount = StakeAccount.fromRaw({
                pubkey: fixture.port.accounts.vaultPortStakeAccount,
                account: stakingAccountRaw,
            });
            assert.equal(
                stakingAccount.getDepositAmount().toU64().toNumber(),
                depositQty
            );
        });

        it("Withdraws", async function () {
            const oldVaultValue = await fixture.getVaultTotalValue();
            const oldLpTokenSupply = await fixture.getLpTokenSupply();

            const withdrawQty = 922051;
            await fixture.withdrawFromVault(withdrawQty);

            const newUserReserveBalance =
                await fixture.getUserReserveTokenBalance();
            const newVaultValue = await fixture.getVaultTotalValue();
            const newLpTokenSupply = await fixture.getLpTokenSupply();

            const stakingAccountRaw =
                await fixture.provider.connection.getAccountInfo(
                    new PublicKey(fixture.port.accounts.vaultPortStakeAccount)
                );
            const stakingAccount = StakeAccount.fromRaw({
                pubkey: fixture.port.accounts.vaultPortStakeAccount,
                account: stakingAccountRaw,
            });
            assert.equal(
                stakingAccount.getDepositAmount().toU64().toNumber(),
                depositQty - withdrawQty
            );

            // Allow max different of 1 token because of rounding error.
            const actualWithdrawAmount = oldVaultValue - newVaultValue;
            const maxDiffAllowed = 1;
            assert.isAtMost(
                Math.abs(actualWithdrawAmount - withdrawQty),
                maxDiffAllowed
            );
            // Actual should <= requested because we rounds down.
            assert.isAtMost(actualWithdrawAmount, withdrawQty);
            assert.equal(oldLpTokenSupply - newLpTokenSupply, withdrawQty);
            assert.equal(newUserReserveBalance, actualWithdrawAmount);
        });

        it("Claim reward", async function () {
            const accumulatedRewardAmount =
                await fixture.port.getUnclaimedStakingRewards(fixture.program);
            assert.isAtLeast(accumulatedRewardAmount, 1);

            await fixture.vaultClient.claimPortReward();

            const stakingAccountRaw2 =
                await fixture.provider.connection.getAccountInfo(
                    new PublicKey(fixture.port.accounts.vaultPortStakeAccount)
                );
            const stakingAccount2 = StakeAccount.fromRaw({
                pubkey: fixture.port.accounts.vaultPortStakeAccount,
                account: stakingAccountRaw2,
            });
            const rewardAmountAfterClaiming =
                await fixture.port.getUnclaimedStakingRewards(fixture.program);
            assert.equal(rewardAmountAfterClaiming, 0);

            const mint = fixture.port.accounts.stakingRewardTokenMint;
            const rewardToken = new Token(
                fixture.program.provider.connection,
                mint,
                TOKEN_PROGRAM_ID,
                Keypair.generate()
            );
            const claimedRewardAmount = (
                await rewardToken.getAccountInfo(
                    fixture.port.accounts.vaultPortRewardToken
                )
            ).amount.toNumber();
            assert.isAtLeast(claimedRewardAmount, accumulatedRewardAmount);

            if (subReward) {
                const subRewardMint =
                    fixture.port.accounts.stakingSubRewardTokenMint;
                const subRewardToken = new Token(
                    fixture.program.provider.connection,
                    subRewardMint,
                    TOKEN_PROGRAM_ID,
                    Keypair.generate()
                );
                const claimedSubRewardAmount = (
                    await subRewardToken.getAccountInfo(
                        fixture.port.accounts.vaultPortSubRewardToken
                    )
                ).amount.toNumber();

                assert.isAtLeast(claimedSubRewardAmount, 1);
            }
        });
    }

    describe("Sub-reward enabled", () => {
        let yieldSrcOptions: YieldSrcSelector = {
            solend: false,
            port: true,
            jet: false,
        };

        const rebalanceMode = RebalanceModes.calculator;
        before(async function () {
            await fixture.initLendingMarkets(yieldSrcOptions, true);
        });
        before(async function () {
            await fixture.initializeVault(
                {
                    allocationCapPct: 100,
                    strategyType: { [StrategyTypes.equalAllocation]: {} },
                    rebalanceMode: { [RebalanceModes.calculator]: {} },
                },
                yieldSrcOptions
            );
        });

        testPortRewardClaiming(true);
    });

    describe("Sub-reward disabled", () => {
        let yieldSrcOptions: YieldSrcSelector = {
            solend: false,
            port: true,
            jet: false,
        };

        const rebalanceMode = RebalanceModes.calculator;
        before(async function () {
            await fixture.initLendingMarkets(yieldSrcOptions, false);
        });
        before(async function () {
            await fixture.initializeVault(
                {
                    allocationCapPct: 100,
                    strategyType: { [StrategyTypes.equalAllocation]: {} },
                    rebalanceMode: { [RebalanceModes.calculator]: {} },
                },
                yieldSrcOptions
            );
        });

        testPortRewardClaiming(false);
    });
});
