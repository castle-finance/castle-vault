import { assert } from "chai";
import { RebalanceModes, StrategyTypes } from "@castlefinance/vault-core";
import { YieldSrcSelector, TestFixture } from "../test_fixture";

describe("Disabled pools", () => {
    const fixture = new TestFixture();

    describe("Rebalance with equal allocation strategy missing 1 pool", () => {
        const vaultAllocationCap = 76;
        let yieldSrcOptions: YieldSrcSelector = {
            solend: true,
            port: false,
            jet: true,
        };

        const rebalanceMode = RebalanceModes.calculator;
        before(async function () {
            await fixture.initLendingMarkets(yieldSrcOptions, true);
        });
        before(async function () {
            await fixture.initializeVault(
                {
                    allocationCapPct: vaultAllocationCap,
                    strategyType: { [StrategyTypes.equalAllocation]: {} },
                    rebalanceMode: { [RebalanceModes.calculator]: {} },
                },
                yieldSrcOptions
            );
        });

        it("Initialize fewer yield sources", async function () {
            assert.equal(0b101, fixture.vaultClient.getYieldSourceFlags());
        });

        it("Update and adjust allocation cap", async function () {
            const oldConfig = fixture.vaultClient.getVaultConfig();
            const newConfig = {
                ...oldConfig,
                allocationCapPct: 40,
            };
            const tx = await fixture.vaultClient.updateConfig(
                fixture.owner,
                newConfig
            );
            await fixture.provider.connection.confirmTransaction(
                tx,
                "singleGossip"
            );
            await fixture.vaultClient.reload();
            assert.equal(
                fixture.vaultClient.getVaultConfig().allocationCapPct,
                51
            );
        });

        fixture.testRebalanceWithdraw(
            1 / 2,
            0,
            1 / 2,
            rebalanceMode,
            true,
            false,
            true
        );
    });
});
