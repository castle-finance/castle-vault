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
    // const cluster: Cluster = "mainnet-beta";
    const cluster: Cluster = "devnet";
    const connection = new Connection(
        // "https://solana-api.syndica.io/access-token/PBhwkfVgRLe1MEpLI5VbMDcfzXThjLKDHroc31shR5e7qrPqQi9TAUoV6aD3t0pg/rpc"
        "https://psytrbhymqlkfrhudd.dev.genesysgo.net:8899/"
    );
    const wallet = Wallet.local();
    const owner = wallet.payer;
    const provider = new Provider(connection, wallet, {
        commitment: "confirmed",
    });

    // const reserveMint = NATIVE_MINT;
    const reserveMint = new PublicKey(
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
    );

    const vaultClient = await VaultClient.initialize(
        provider,
        wallet,
        DeploymentEnvs.mainnet,
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

    // When the reserve token is not available on a lending pool, the load instruction will fail.
    // This is how we detect if we should disable (i.e. not initialize) a lending pool.
    try {
        const solend = await SolendReserveAsset.load(
            provider,
            cluster,
            reserveMint
        );
        await vaultClient.initializeSolend(provider, wallet, solend, owner);
        console.log("Succesfully initialized Solend");
    } catch (error) {
        console.log("Failed to initialize Solend: ", error);
    }

    try {
        const port = await PortReserveAsset.load(
            provider,
            cluster,
            reserveMint
        );
        await vaultClient.initializePort(provider, wallet, port, owner);
        console.log("Succesfully initialized Port");
    } catch (error) {
        console.log("Failed to initialize Port: ", error);
    }

    try {
        const jet = await JetReserveAsset.load(provider, cluster, reserveMint);
        await vaultClient.initializeJet(provider, wallet, jet, owner);
        console.log("Succesfully initialized Jet");
    } catch (error) {
        console.log("Failed to initialize Jet: ", error);
    }
};

main();
