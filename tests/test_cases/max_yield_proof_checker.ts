import { RebalanceModes, StrategyTypes } from "@castlefinance/vault-core";
import { TestFixture } from "../test_fixture";

describe("Max yield proof checker", () => {
    const fixture = new TestFixture();

    describe("Rebalance", () => {
        const rebalanceMode = RebalanceModes.proofChecker;
        before(async function () {
            await fixture.initLendingMarkets();
        });
        before(async function () {
            await fixture.initializeVault({
                allocationCapPct: fixture.vaultAllocationCap,
                strategyType: { [StrategyTypes.maxYield]: {} },
                rebalanceMode: { [RebalanceModes.proofChecker]: {} },
            });
        });
        fixture.testRebalanceWithdraw(
            1 - fixture.vaultAllocationCap / 100,
            0,
            fixture.vaultAllocationCap / 100,
            rebalanceMode
        );
    });
});
