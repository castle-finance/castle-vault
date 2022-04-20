import { assert } from "chai";
import {
    Cluster,
    Connection,
    clusterApiUrl,
    LAMPORTS_PER_SOL,
    PublicKey,
} from "@solana/web3.js";
import { NATIVE_MINT } from "@solana/spl-token";
import { Wallet, Provider } from "@project-serum/anchor";

import {
    JetReserveAsset,
    PortReserveAsset,
    SolendReserveAsset,
    VaultClient,
} from "../src";
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
    let jet: JetReserveAsset;
    let solend: SolendReserveAsset;
    let port: PortReserveAsset;

    before(async () => {
        //const sig = await connection.requestAirdrop(wallet.publicKey, LAMPORTS_PER_SOL);
        //await connection.confirmTransaction(sig, "confirmed");
    });

    it("loads devnet sol vault", async () => {
        const vaultId = new PublicKey(
            "FEthCwaa3sGvPTTYV7ZSuYKTm4gaHPGtju4xFxDqv5gJ"
        );
        vaultClient = await VaultClient.load(
            provider,
            cluster,
            NATIVE_MINT,
            vaultId
        );
        assert.isNotNull(vaultClient);

        console.log(
            "Total value: ",
            (await vaultClient.getTotalValue()).toString()
        );
        console.log((await vaultClient.getApy()).toString());

        jet = vaultClient.getJet();
        solend = vaultClient.getSolend();
        port = vaultClient.getPort();

        console.log("Jet");
        console.log((await jet.getApy()).toString());
        console.log((await jet.getBorrowedAmount()).toString());
        console.log((await jet.getDepositedAmount()).toString());

        console.log("Solend");
        console.log((await solend.getApy()).toString());
        console.log((await solend.getBorrowedAmount()).toString());
        console.log((await solend.getDepositedAmount()).toString());

        console.log("Port");
        console.log((await port.getApy()).toString());
        console.log((await port.getBorrowedAmount()).toString());
        console.log((await port.getDepositedAmount()).toString());
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
        try {
            const sigs = await vaultClient.withdraw(wallet, withdrawAmount);
            await connection.confirmTransaction(
                sigs[sigs.length - 1],
                "finalized"
            );
        } catch (e) {
            console.log(e);
            console.log(Object.keys(e));
        }

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
