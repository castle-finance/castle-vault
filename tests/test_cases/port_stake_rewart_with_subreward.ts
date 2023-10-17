import { RebalanceModes, StrategyTypes } from "@castlefinance/vault-core";
import { YieldSrcSelector, TestFixture } from "../test_fixture";

describe("Stake Port LP token and claim reward", () => {
    const fixture = new TestFixture();

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

        fixture.testPortRewardClaiming(true);
    });
});
