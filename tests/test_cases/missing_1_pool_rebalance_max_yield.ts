import { assert } from "chai";
import { RebalanceModes, StrategyTypes } from "@castlefinance/vault-core";
import { YieldSrcSelector, TestFixture } from "../test_fixture";

describe("Disabled pools", () => {
    const fixture = new TestFixture();

    describe("Rebalance with max yield strategy missing 1 pool", () => {
        const vaultAllocationCap = 76;
        let yieldSrcOptions: YieldSrcSelector = {
            solend: true,
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
                    allocationCapPct: vaultAllocationCap,
                    strategyType: { [StrategyTypes.maxYield]: {} },
                    rebalanceMode: { [RebalanceModes.calculator]: {} },
                },
                yieldSrcOptions
            );
        });

        it("Initialize fewer yield sources", async function () {
            assert.equal(0b011, fixture.vaultClient.getYieldSourceFlags());
        });

        fixture.testRebalanceWithdraw(
            vaultAllocationCap / 100,
            1 - vaultAllocationCap / 100,
            0,
            rebalanceMode,
            true,
            true,
            false
        );
    });
});
