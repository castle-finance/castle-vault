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
    //const cluster: Cluster = "mainnet-beta";
    const cluster: Cluster = "devnet";
    const connection = new Connection(
        //"https://solana-api.syndica.io/access-token/PBhwkfVgRLe1MEpLI5VbMDcfzXThjLKDHroc31shR5e7qrPqQi9TAUoV6aD3t0pg/rpc"
        "https://psytrbhymqlkfrhudd.dev.genesysgo.net:8899/"
    );
    const wallet = Wallet.local();
    const provider = new Provider(connection, wallet, {
        commitment: "confirmed",
    });

    const reserveMint = NATIVE_MINT;
    //const reserveMint = new PublicKey(
    //    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
    //);

    const solend = await SolendReserveAsset.load(
        provider,
        cluster,
        reserveMint
    );
    const port = await PortReserveAsset.load(provider, cluster, reserveMint);
    const jet = await JetReserveAsset.load(provider, cluster, reserveMint);

    const vaultClient = await VaultClient.initialize(
        provider,
        wallet,
        DeploymentEnvs.devnetStaging,
        reserveMint,
        solend,
        port,
        jet,
        wallet.publicKey,
        new PublicKey("jvUsXAgE2Gg92BbEBDAu7h5p8SEZpVjFqURJkzSsLNk"),
        {
            allocationCapPct: 60,
            rebalanceMode: { [RebalanceModes.proofChecker]: {} },
            strategyType: { [StrategyTypes.maxYield]: {} },
        }
    );
    console.log(vaultClient.vaultId.toString());
};

main();
