import { assert } from "chai";
import { Transaction } from "@solana/web3.js";
import { RebalanceModes, StrategyTypes } from "@castlefinance/vault-core";
import { TestFixture } from "../test_fixture";

describe("Refresh Check", () => {
    const fixture = new TestFixture();
    const vaultAllocationCap = 76;

    const rebalanceMode = RebalanceModes.calculator;
    before(async function () {
        await fixture.initLendingMarkets();
    });
    before(async function () {
        await fixture.initializeVault({
            allocationCapPct: vaultAllocationCap,
            strategyType: { [StrategyTypes.equalAllocation]: {} },
            rebalanceMode: { [RebalanceModes.calculator]: {} },
        });
    });

    it("Incomplete refresh after rebalance should not succeed", async function () {
        const depositQty = 3000000;
        await fixture.mintReserveToken(
            fixture.userReserveTokenAccount,
            depositQty
        );
        await fixture.depositToVault(depositQty);

        await fixture.performRebalance({
            solend: (1 / 3) * 10000,
            port: (1 / 3) * 10000,
            jet: (1 / 3) * 10000,
        });

        const maxDiffAllowed = 1;

        // Deleberately craft an incomplete refresh
        const refreshTx = new Transaction();
        refreshTx.add(
            fixture.vaultClient
                .getJet()
                .getRefreshIx(
                    fixture.program,
                    fixture.vaultClient.vaultId,
                    fixture.vaultClient.getVaultState()
                )
        );
        refreshTx.add(fixture.vaultClient.getConsolidateRefreshIx());
        
        const expectedErrorCode = "0x12f";
        fixture.suppressLogs();
        try {
            const sig = await fixture.program.provider.send(refreshTx);
            await fixture.program.provider.connection.confirmTransaction(
                sig,
                "singleGossip"
            );
        } catch (err) {
            assert.isTrue(
                err.message.includes(expectedErrorCode),
                `Error code ${expectedErrorCode} not included in error message: ${err}`
            );
        }
        fixture.restoreLogs();

        await fixture.vaultClient.reload();
        const vaultValutStoredOnChain = fixture.vaultClient
            .getVaultState()
            .value.value.toNumber();
        assert.isAtMost(
            Math.abs(vaultValutStoredOnChain - Math.floor(depositQty)),
            maxDiffAllowed
        );
    });
});
