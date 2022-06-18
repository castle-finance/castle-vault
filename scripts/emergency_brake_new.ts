import Big from "big.js";

import { Connection, PublicKey, Transaction } from "@solana/web3.js";
import { BN, Provider, Wallet } from "@project-serum/anchor";

import { DeploymentEnvs } from "@castlefinance/vault-core";

import { VaultClient, VaultFlags } from "../sdk/src";

function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

// TODO make this a CLI
const main = async () => {
    const vaultId = new PublicKey(
        //"Bv4d2wWb7myxpjWudHnEMjdJstxjkiWqX61xLhPBrBx" //devnet-staging
        "3tBqjyYtf9Utb1NNsx4o7AV1qtzHoxsMXgkmat3rZ3y6" //mainnet
    );
    const connection = new Connection(
        "https://solana-api.syndica.io/access-token/PBhwkfVgRLe1MEpLI5VbMDcfzXThjLKDHroc31shR5e7qrPqQi9TAUoV6aD3t0pg/rpc"
        //"https://psytrbhymqlkfrhudd.dev.genesysgo.net:8899/"
    );
    const wallet = Wallet.local();
    // TODO figure out how to use ledger as owner
    const owner = wallet.payer;
    const provider = new Provider(connection, wallet, {});

    const vaultClient = await VaultClient.load(
        provider,
        vaultId,
        DeploymentEnvs.mainnet
    );
    console.log("Vault client loaded");

    while (true) {
        console.log("\nChecking...");
        const solend = vaultClient.getSolend();
        const borrowed = (await solend.getBorrowedAmount()).lamports;
        const deposited = (await solend.getDepositedAmount()).lamports;
        console.log("Borrowed:", borrowed.toString());
        console.log("Deposited:", deposited.toString());

        const withdrawable = deposited.sub(borrowed);
        console.log("Withdrawable:", withdrawable.toString());

        // $100
        if (withdrawable.gt(100000000)) {
            const exchangeRate = new Big(
                solend.reserve.stats.cTokenExchangeRate
            );

            const amount = withdrawable.div(exchangeRate).round().sub(1);
            const brakeTx = new Transaction();
            brakeTx.add(vaultClient.getRefreshIx());
            brakeTx.add(
                vaultClient.getReconcileSolendIx(new BN(amount.toString()))
            );

            const sig = await provider.send(brakeTx, [], {
                skipPreflight: true,
            });
            console.log(sig);
            try {
                await connection.confirmTransaction(sig);
            } catch (e) {
                console.error(e);
            }
            console.log("confirmed");
        }

        await sleep(10000);
    }

    // Emergency brake
    //const brakeSigs = await vaultClient.emergencyBrake();
    const brakeTx = new Transaction();
    brakeTx.add(vaultClient.getRefreshIx());
    //581913022438
    brakeTx.add(vaultClient.getReconcileSolendIx(new BN(300000000000)));

    const sig = await provider.send(brakeTx, [], { skipPreflight: true });
    await connection.confirmTransaction(sig);

    //await Promise.all(
    //    brakeSigs.map((sig) => connection.confirmTransaction(sig, "finalized"))
    //);
    console.log("Brake txs finalized");

    // Halt reconciles
    //const haltSig = await vaultClient.updateHaltFlags(
    //    owner,
    //    VaultFlags.HaltReconciles
    //);
    //console.log("Halt tx sent: ", haltSig);
    //await connection.confirmTransaction(haltSig, "finalized");
    //console.log("Halt tx finalized");

    //await vaultClient.reload();

    //console.log("Vault halt flags: ", vaultClient.getHaltFlags());
};

main();
