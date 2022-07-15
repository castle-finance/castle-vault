import * as anchor from "@project-serum/anchor";
import { RebalanceModes, StrategyTypes } from "@castlefinance/vault-core";
import { TestFixture } from "../test_fixture";

describe("Equal allocation strategy", () => {
    const fixture = new TestFixture();

    describe("Deposit and withdrawal", () => {
        before(async function () {
            await fixture.initLendingMarkets();
        });
        before(async function () {
            await fixture.initializeVault({
                depositCap: new anchor.BN(fixture.vaultDepositCap),
                strategyType: { [StrategyTypes.equalAllocation]: {} },
                rebalanceMode: { [RebalanceModes.calculator]: {} },
            });
        });
        fixture.testDepositAndWithdrawal();
    });
});
