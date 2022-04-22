import { assert } from "chai";
import {
    Cluster,
    Connection,
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

const USDC_MINT = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

describe("VaultClient", () => {
    const PROGRAM_ID = new PublicKey(
        "Cast1eoVj8hwfKKRPji4cqX7WFgcnYz3um7TTgnaJKFn"
        //"4tSMVfVbnwZcDwZB1M1j27dx9hdjL72VR9GM8AykpAvK"
    );
    //const cluster: Cluster = "devnet";
    const cluster: Cluster = "mainnet-beta";
    const connection = new Connection(
        "https://solana-api.syndica.io/access-token/PBhwkfVgRLe1MEpLI5VbMDcfzXThjLKDHroc31shR5e7qrPqQi9TAUoV6aD3t0pg/rpc"
        //"https://psytrbhymqlkfrhudd.dev.genesysgo.net:8899/"
    );
    const wallet = Wallet.local();
    const provider = new Provider(connection, wallet, {
        commitment: "confirmed",
    });

    const depositAmount = (0.1 * LAMPORTS_PER_SOL) / 1000;

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
            //"3PUZJamT1LAwgkjT58PHoY8izM1Y8jRz2A1UwiV4JTkk"
            //"FEthCwaa3sGvPTTYV7ZSuYKTm4gaHPGtju4xFxDqv5gJ"
            //"9n6ekjHHgkPB9fVuWHzH6iNuxBxN22hEBryZXYFg6cNk" // old devnet-parity vault
            "EDtqJFHksXpdXLDuxgoYpjpxg3LjBpmW4jh3fkz4SX32" // old mainnet vault
            //"HKnAJ5wX3w7b52wFr4ZFf7fAtiHS3oaFngnkViXGCusf" // new devnet-parity vault
            //"5zwJzQbw8PzNT2SwkhwrYfriVsLshytWk1UQkkudQv6G" // new devnet-staging vault
            //"5VsCBvW7CswQfYe5rQdP9W5tSWb2rEZBQZ2C8wU7qrnL" // new mainnet vault
        );
        vaultClient = await VaultClient.load(
            provider,
            cluster,
            USDC_MINT,
            //NATIVE_MINT,
            vaultId,
            PROGRAM_ID
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
        console.log(
            (await vaultClient.getVaultJetLpTokenAccountValue()).toString()
        );
        console.log((await jet.getApy()).toString());
        console.log((await jet.getBorrowedAmount()).toString());
        console.log((await jet.getDepositedAmount()).toString());

        console.log("Solend");
        console.log(
            (await vaultClient.getVaultSolendLpTokenAccountValue()).toString()
        );
        console.log((await solend.getApy()).toString());
        console.log((await solend.getBorrowedAmount()).toString());
        console.log((await solend.getDepositedAmount()).toString());

        console.log("Port");
        console.log(
            (await vaultClient.getVaultPortLpTokenAccountValue()).toString()
        );
        console.log((await port.getApy()).toString());
        console.log((await port.getBorrowedAmount()).toString());
        console.log((await port.getDepositedAmount()).toString());
    });

    it("deposits", async () => {
        const startUserValue = await vaultClient.getUserValue(wallet.publicKey);
        console.log("start value: ", startUserValue.toNumber());

        //const userReserveTokenAccount = wallet.publicKey;
        const userReserveTokenAccount =
            await vaultClient.getUserReserveTokenAccount(wallet.publicKey);
        try {
            const sigs = await vaultClient.deposit(
                wallet,
                depositAmount,
                userReserveTokenAccount
            );
            await connection.confirmTransaction(
                sigs[sigs.length - 1],
                "finalized"
            );
        } catch (e) {
            console.log(e);
            throw e;
        }

        const endUserValue = await vaultClient.getUserValue(wallet.publicKey);
        console.log("end value: ", endUserValue.toNumber());

        assert.isAtMost(
            Math.abs(
                endUserValue.sub(startUserValue).sub(depositAmount).toNumber()
            ),
            1000
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

    // TODO sleep to avoid AlreadyProcessed error

    it("withdraws", async () => {
        const startUserValue = await vaultClient.getUserValue(wallet.publicKey);
        console.log("start value: ", startUserValue.toNumber());

        const exchangeRate = await vaultClient.getLpExchangeRate();
        const withdrawAmount = new Big(depositAmount)
            .div(exchangeRate)
            .round(0, Big.roundDown)
            .toNumber();

        try {
            const sigs = await vaultClient.withdraw(wallet, withdrawAmount);
            await connection.confirmTransaction(
                sigs[sigs.length - 1],
                "finalized"
            );
        } catch (e) {
            console.log(e);
            throw e;
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
