import { TestFixture } from "../test_fixture";

describe("Max yield calculator", () => {
    const fixture = new TestFixture();

    describe("Rebalance", () => {
        before(async function () {
            await fixture.initLendingMarkets();
        });
        before(async function () {
            await fixture.initializeVault({
                allocationCapPct: fixture.vaultAllocationCap,
            });
        });

        fixture.testRebalanceWithdraw(
            1 - fixture.vaultAllocationCap / 100,
            0,
            fixture.vaultAllocationCap / 100
        );

        // TODO borrow from solend to increase apy and ensure it switches to that
        // TODO borrow from port to increase apy and ensure it switches to that
    });
});
