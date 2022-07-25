import { Cluster, Connection, PublicKey } from "@solana/web3.js";
import { Program, Provider, Wallet } from "@project-serum/anchor";
import { NATIVE_MINT } from "@solana/spl-token";

import {
    DeploymentEnvs,
    RebalanceModes,
    StrategyTypes,
} from "@castlefinance/vault-core";

import {
    JetReserveAsset,
    PortReserveAsset,
    SolendReserveAsset,
    VaultClient,
} from "../sdk/src";

const main = async () => {
    const cluster: Cluster = "devnet";
    const connection = new Connection("https://api.devnet.solana.com");
    const wallet = Wallet.local();
    const owner = wallet.payer;
    const provider = new Provider(connection, wallet, {
        commitment: "confirmed",
    });

    const reserveMint = new PublicKey(
        "G6YKv19AeGZ6pUYUwY9D7n4Ry9ESNFa376YqwEkUkhbi"
    );

    const vaultClient = await VaultClient.initialize(
        provider,
        wallet,
        DeploymentEnvs.devnetStaging,
        reserveMint,
        owner.publicKey,
        new PublicKey("jvUsXAgE2Gg92BbEBDAu7h5p8SEZpVjFqURJkzSsLNk"),
        {
            allocationCapPct: 60,
            rebalanceMode: { [RebalanceModes.calculator]: {} },
            strategyType: { [StrategyTypes.maxYield]: {} },
        }
    );
    console.log("Vauld ID: ", vaultClient.vaultId.toString());

    try {
        const port = await PortReserveAsset.load(
            provider,
            cluster,
            reserveMint
        );
        await vaultClient.initializePort(wallet, port, owner);
        await vaultClient.initializePortAdditionalState(wallet, owner);
        await vaultClient.initializePortRewardAccounts(
            wallet,
            owner,
            provider,
            DeploymentEnvs.devnetStaging
        );
        console.log("Succesfully initialized Port");
    } catch (error) {
        console.log("Failed to initialize Port: ", error);
    }
};

main();
