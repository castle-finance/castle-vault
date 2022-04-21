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
} from "../sdk/src";

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
        //new PublicKey("Cast1eoVj8hwfKKRPji4cqX7WFgcnYz3um7TTgnaJKFn"),
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
        { calculator: {} },
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
};

main();
