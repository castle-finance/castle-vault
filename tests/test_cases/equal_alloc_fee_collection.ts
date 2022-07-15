import { RebalanceModes, StrategyTypes } from "@castlefinance/vault-core";
import { TestFixture } from "../test_fixture";

describe("Equal allocation strategy", () => {
    const fixture = new TestFixture();

    describe("Fees", () => {
        const feeMgmtBps = 10000;
        const feeCarryBps = 10000;
        const referralFeePct = 20;
        before(async function () {
            await fixture.initLendingMarkets();
        });
        before(async function () {
            await fixture.initializeVault({
                feeCarryBps,
                feeMgmtBps,
                referralFeePct,
                strategyType: { [StrategyTypes.equalAllocation]: {} },
                rebalanceMode: { [RebalanceModes.calculator]: {} },
            });
        });
        fixture.testFees(feeCarryBps, feeMgmtBps, referralFeePct);
    });
});
