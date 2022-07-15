import { assert } from "chai";
import { RebalanceModes, StrategyTypes } from "@castlefinance/vault-core";
import { YieldSrcSelector, TestFixture } from "../test_fixture";

describe("Disabled pools", () => {
    const fixture = new TestFixture();

    describe("Rebalance with equal allocation strategy missing 2 pools", () => {
        const vaultAllocationCap = 76;
        let yieldSrcOptions: YieldSrcSelector = {
            solend: true,
            port: false,
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
                    strategyType: { [StrategyTypes.equalAllocation]: {} },
                    rebalanceMode: { [RebalanceModes.calculator]: {} },
                },
                yieldSrcOptions
            );
        });

        it("Initialize fewer yield sources", async function () {
            assert.equal(0b001, fixture.vaultClient.getYieldSourceFlags());
        });

        fixture.testRebalanceWithdraw(1, 0, 0, rebalanceMode, true, false, false);
    });
});
