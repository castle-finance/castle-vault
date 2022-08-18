import { Cluster, Connection, Keypair, PublicKey } from "@solana/web3.js";
import { WalletAdaptor, Wallet, AnchorProvider } from "@project-serum/anchor";
import { NATIVE_MINT } from "@solana/spl-token";
import { LedgerWallet } from "../sdk/lib";

import {
    DeploymentEnvs,
    RebalanceModes,
    StrategyTypes,
} from "@castlefinance/vault-core";

import { PortReserveAsset, SolendReserveAsset, VaultClient } from "../sdk/src";

const CONNECTION_DEVNET = new Connection("https://devnet.genesysgo.net/");
const CONNECTION_MAINNET = new Connection(
    "https://solana-api.syndica.io/access-token/PBhwkfVgRLe1MEpLI5VbMDcfzXThjLKDHroc31shR5e7qrPqQi9TAUoV6aD3t0pg/rpc"
);

const main = async () => {
    let env = DeploymentEnvs.devnetStaging;
    let connection = CONNECTION_DEVNET;
    let cluster: Cluster = "devnet";

    let args = process.argv.slice(2);
    if (args.includes("--mainnet")) {
        env = DeploymentEnvs.mainnet;
        connection = CONNECTION_MAINNET;
        cluster = "mainnet-beta";
    }

    const wallet = Wallet.local();
    const provider = new AnchorProvider(connection, wallet, {
        commitment: "finalized",
    });

    let owner: Keypair | WalletAdaptor = wallet.payer;
    if (args.includes("--ledger")) {
        const ledgerWallet = new LedgerWallet(0);
        await ledgerWallet.connect();
        owner = ledgerWallet as WalletAdaptor;
    }

    console.log("Vault Owner:", owner.publicKey.toString());

    const reserveMint = NATIVE_MINT;
    // const reserveMint = new PublicKey(
    //     "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
    // );

    console.log("Vault Reserve Token:", reserveMint.toString());

    let vaultClient = await VaultClient.initialize(
        provider,
        wallet,
        env,
        reserveMint,
        owner.publicKey,
        new PublicKey("jvUsXAgE2Gg92BbEBDAu7h5p8SEZpVjFqURJkzSsLNk"),
        {
            allocationCapPct: 60,
            rebalanceMode: { [RebalanceModes.calculator]: {} },
            strategyType: { [StrategyTypes.maxYield]: {} },
        }
    );
    const vaultId = vaultClient.vaultId;
    console.log("Vault ID: ", vaultId.toString());

    // This step creates the PDA that holds DEX account date.
    // The DEX are used to sell the liquidity mining rewards.
    await vaultClient.initializeDexStates(wallet, owner);

    // When the reserve token is not available on a lending pool, the load instruction will fail.
    // This is how we detect if we should disable (i.e. not initialize) a lending pool.
    try {
        const solend = await SolendReserveAsset.load(
            provider,
            cluster,
            reserveMint
        );
        await vaultClient.initializeSolend(wallet, solend, owner);
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
        await vaultClient.initializePort(wallet, port, owner);
        await vaultClient.initializePortAdditionalState(wallet, owner);
        await vaultClient.initializePortRewardAccounts(wallet, owner);
        await vaultClient.initializeOrcaLegacy(wallet, owner, env);
        await vaultClient.initializeOrcaLegacyMarket(wallet, owner);
        console.log("Succesfully initialized Port");
    } catch (error) {
        console.log("Failed to initialize Port: ", error);
    }
};

main();
