import { Cluster, clusterApiUrl, Connection, PublicKey } from "@solana/web3.js";
import { Program, Provider, Wallet } from "@project-serum/anchor";
import { NATIVE_MINT } from "@solana/spl-token";

import {
  CastleLendingAggregator,
  JetReserveAsset,
  PortReserveAsset,
  //PROGRAM_ID,
  SolendReserveAsset,
  VaultClient,
} from "vault-sdk";

const PROGRAM_ID = new PublicKey("E5xEvrNhrknmgGbRv8iDDqHsgqG1xeMEdfPjL8i4YEVo");

const main = async () => {
  const cluster: Cluster = "devnet";
  const connection = new Connection(clusterApiUrl(cluster));
  const wallet = Wallet.local();
  const provider = new Provider(connection, wallet, { commitment: "confirmed" });
  const program = (await Program.at(
    PROGRAM_ID,
    provider
  )) as Program<CastleLendingAggregator>;

  const reserveMint = NATIVE_MINT;

  const solend = await SolendReserveAsset.load(provider, cluster, reserveMint);
  const port = await PortReserveAsset.load(provider, cluster, reserveMint);
  const jet = await JetReserveAsset.load(provider, cluster, reserveMint);

  const vaultClient = await VaultClient.initialize(
    program,
    wallet,
    reserveMint,
    solend,
    port,
    jet,
    { maxYield: {} }
  );
  console.log(vaultClient.vaultId);

  //const vaultId = new PublicKey("81krfC8ptDbjwY5bkur1SqHYKLPxGQLYBQEUv5zhojUW");
  //const vaultClient = await VaultClient.load(provider, cluster, reserveMint, vaultId);
  console.log(vaultClient.vaultState);
};

main();
