import { assert } from "chai";
import { Connection, LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import { NATIVE_MINT } from "@solana/spl-token";
import { Wallet, Provider } from "@project-serum/anchor";

import {
    JetReserveAsset,
    PortReserveAsset,
    SolendReserveAsset,
    VaultClient,
} from "../src";
import Big from "big.js";
import { DeploymentEnvs } from "@castlefinance/vault-core";
import { TokenAmount } from "../src";

describe("VaultClient", () => {
    const connection = new Connection(
        "https://solana-api.syndica.io/access-token/PBhwkfVgRLe1MEpLI5VbMDcfzXThjLKDHroc31shR5e7qrPqQi9TAUoV6aD3t0pg/rpc"
        //"https://psytrbhymqlkfrhudd.dev.genesysgo.net:8899/"
    );
    const wallet = Wallet.local();
    const provider = new Provider(connection, wallet, {
        commitment: "confirmed",
    });

    const depositAmount = new TokenAmount(Big(0.05), 9);

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
            //"Bv4d2wWb7myxpjWudHnEMjdJstxjkiWqX61xLhPBrBx" //devnet-staging
            "3tBqjyYtf9Utb1NNsx4o7AV1qtzHoxsMXgkmat3rZ3y6" //mainnet
        );
        vaultClient = await VaultClient.load(
            provider,
            vaultId,
            DeploymentEnvs.mainnet
        );
        assert.isNotNull(vaultClient);
        console.log("Initialized client");

        console.log(
            "Total value: ",
            (await vaultClient.getTotalValue()).getAmount()
        );
        console.log("APY: ", (await vaultClient.getApy()).toNumber());

        jet = vaultClient.getJet();
        solend = vaultClient.getSolend();
        port = vaultClient.getPort();

        console.log("Jet");
        console.log(
            "value: ",
            (await vaultClient.getVaultJetLpTokenAccountValue()).getAmount()
        );
        console.log((await jet.getApy()).toNumber());
        console.log((await jet.getBorrowedAmount()).getAmount());
        console.log((await jet.getDepositedAmount()).getAmount());

        console.log("Solend");
        console.log(
            "value: ",
            (await vaultClient.getVaultSolendLpTokenAccountValue()).getAmount()
        );
        console.log((await solend.getApy()).toNumber());
        console.log((await solend.getBorrowedAmount()).getAmount());
        console.log((await solend.getDepositedAmount()).getAmount());

        console.log("Port");
        console.log(
            "value: ",
            (await vaultClient.getVaultPortLpTokenAccountValue()).getAmount()
        );
        console.log((await port.getApy()).toNumber());
        console.log((await port.getBorrowedAmount()).getAmount());
        console.log((await port.getDepositedAmount()).getAmount());
    });

    it("deposits", async () => {
        const startUserValue = await vaultClient.getUserValue(wallet.publicKey);
        console.log("start value: ", startUserValue.getAmount());

        //const userReserveTokenAccount = wallet.publicKey;
        const userReserveTokenAccount =
            await vaultClient.getUserReserveTokenAccount(wallet.publicKey);
        try {
            const sigs = await vaultClient.deposit(
                wallet,
                depositAmount.lamports.toNumber(),
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
        console.log("end value: ", endUserValue.getAmount());

        assert.isAtMost(
            Math.abs(
                endUserValue.sub(startUserValue).sub(depositAmount).getAmount()
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

    it("withdraws", async () => {
        const startUserValue = await vaultClient.getUserValue(wallet.publicKey);
        console.log("start value: ", startUserValue.getAmount());

        const exchangeRate = await vaultClient.getLpExchangeRate();
        const withdrawAmount = depositAmount.lamports
            .div(exchangeRate.toBig())
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
        console.log("end value: ", endUserValue.getAmount());

        assert.isAtMost(
            Math.abs(
                startUserValue.sub(endUserValue).sub(depositAmount).getAmount()
            ),
            1000
        );
    });
});
