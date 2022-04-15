import { Cluster, Connection, PublicKey } from "@solana/web3.js";
import { Program, Provider, Wallet } from "@project-serum/anchor";
import { NATIVE_MINT } from "@solana/spl-token";

import {
    CastleLendingAggregator,
    JetReserveAsset,
    PortReserveAsset,
    PROGRAM_ID,
    SolendReserveAsset,
    VaultClient,
} from "vault-sdk";

const main = async () => {
    const cluster: Cluster = "devnet";
    const connection = new Connection(
        "https://psytrbhymqlkfrhudd.dev.genesysgo.net:8899/"
    );
    const wallet = Wallet.local();
    const provider = new Provider(connection, wallet, {
        commitment: "confirmed",
    });
    const program = (await Program.at(
        PROGRAM_ID,
        provider
    )) as Program<CastleLendingAggregator>;

    const reserveMint = NATIVE_MINT;

    const solend = await SolendReserveAsset.load(
        provider,
        cluster,
        reserveMint
    );
    const port = await PortReserveAsset.load(provider, cluster, reserveMint);
    const jet = await JetReserveAsset.load(provider, cluster, reserveMint);

    const vaultClient = await VaultClient.initialize(
        program,
        wallet,
        reserveMint,
        solend,
        port,
        jet,
        { maxYield: {} },
        wallet.publicKey,
        {
            feeCarryBps: 0,
            feeMgmtBps: 0,
            referralFeePct: 0,
            referralFeeOwner: new PublicKey(
                "jvUsXAgE2Gg92BbEBDAu7h5p8SEZpVjFqURJkzSsLNk"
            ),
        }
    );
    console.log(vaultClient.vaultId);
    console.log(vaultClient.vaultState);
};

main();
