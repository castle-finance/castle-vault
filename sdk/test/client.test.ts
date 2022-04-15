import { assert } from "chai";
import {
    Cluster,
    Connection,
    LAMPORTS_PER_SOL,
    PublicKey,
} from "@solana/web3.js";
import { NATIVE_MINT } from "@solana/spl-token";
import { Wallet, Provider } from "@project-serum/anchor";

import { VaultClient } from "../src";
import Big from "big.js";

describe("VaultClient", () => {
    const cluster: Cluster = "devnet";
    const connection = new Connection(
        "https://psytrbhymqlkfrhudd.dev.genesysgo.net:8899/"
    );
    const wallet = Wallet.local();
    const provider = new Provider(connection, wallet, {
        commitment: "confirmed",
    });

    const depositAmount = LAMPORTS_PER_SOL * 1;

    let vaultClient: VaultClient;

    before(async () => {
        const sig = await connection.requestAirdrop(
            wallet.publicKey,
            LAMPORTS_PER_SOL
        );
        await connection.confirmTransaction(sig, "confirmed");
    });

    it("loads devnet sol vault", async () => {
        const vaultId = new PublicKey(
            //"3PUZJamT1LAwgkjT58PHoY8izM1Y8jRz2A1UwiV4JTkk"
            "FEthCwaa3sGvPTTYV7ZSuYKTm4gaHPGtju4xFxDqv5gJ"
        );
        vaultClient = await VaultClient.load(
            provider,
            cluster,
            NATIVE_MINT,
            vaultId
        );
        assert.isNotNull(vaultClient);

        const lpTokenMintInfo = await vaultClient.getLpTokenMintInfo();
        console.log("LP token supply: ", lpTokenMintInfo.supply.toString());

        console.log(
            "Total value: ",
            (await vaultClient.getTotalValue()).toNumber()
        );

        console.log(
            "LP exchange rate: ",
            (await vaultClient.getLpExchangeRate()).toNumber()
        );

        console.log("Vault APY: ", (await vaultClient.getApy()).toNumber());
        console.log("Jet APY: ", (await vaultClient.jet.getApy()).toNumber());
        console.log("Port APY: ", (await vaultClient.port.getApy()).toNumber());
        console.log(
            "Solend APY: ",
            (await vaultClient.solend.getApy()).toNumber()
        );
    });

    it("deposits", async () => {
        const startUserValue = await vaultClient.getUserValue(wallet.publicKey);
        console.log("start value: ", startUserValue.toNumber());

        const sigs = await vaultClient.deposit(
            wallet,
            depositAmount,
            wallet.publicKey
        );
        await connection.confirmTransaction(sigs[sigs.length - 1], "finalized");

        const endUserValue = await vaultClient.getUserValue(wallet.publicKey);
        console.log("end value: ", endUserValue.toNumber());

        assert.isAtMost(
            Math.abs(
                endUserValue.sub(startUserValue).sub(depositAmount).toNumber()
            ),
            1000000
        );
    });

    it("rebalances", async () => {
        const sigs = await vaultClient.rebalance();
        const result = await connection.confirmTransaction(
            sigs[sigs.length - 1],
            "finalized"
        );
        assert.isNull(result.value.err);
    });

    it("withdraws", async () => {
        const startUserValue = await vaultClient.getUserValue(wallet.publicKey);
        console.log("start value: ", startUserValue.toNumber());

        const exchangeRate = await vaultClient.getLpExchangeRate();
        const withdrawAmount = new Big(depositAmount)
            .div(exchangeRate)
            .toNumber();

        const sigs = await vaultClient.withdraw(wallet, withdrawAmount);
        await connection.confirmTransaction(sigs[sigs.length - 1], "finalized");

        const endUserValue = await vaultClient.getUserValue(wallet.publicKey);
        console.log("end value: ", endUserValue.toNumber());

        assert.isAtMost(
            Math.abs(
                startUserValue.sub(endUserValue).sub(depositAmount).toNumber()
            ),
            1000000
        );
    });
});
