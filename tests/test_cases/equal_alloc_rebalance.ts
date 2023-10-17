import { RebalanceModes, StrategyTypes } from "@castlefinance/vault-core";
import { TestFixture } from "../test_fixture";

describe("Equal allocation strategy", () => {
    const fixture = new TestFixture();

    describe("Rebalance", () => {
        before(async function () {
            await fixture.initLendingMarkets();
        });
        before(async function () {
            await fixture.initializeVault({
                strategyType: { [StrategyTypes.equalAllocation]: {} },
                rebalanceMode: { [RebalanceModes.calculator]: {} },
            });
        });
        fixture.testRebalanceWithdraw(1 / 3, 1 / 3, 1 / 3);
    });
});
