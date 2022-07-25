import { Provider, Wallet } from "@project-serum/anchor";
import {
    Cluster,
    Keypair,
    PublicKey,
    SystemProgram,
    Transaction,
    TransactionInstruction,
    Connection,
    SYSVAR_CLOCK_PUBKEY,
    SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import {
    NATIVE_MINT,
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    Token as SplToken,
} from "@solana/spl-token";
import { ENV } from "@solana/spl-token-registry";
import * as anchor from "@project-serum/anchor";

import { DeploymentEnvs } from "@castlefinance/vault-core";
import {
    VaultClient,
    VaultFlags,
    YieldSourceFlags,
    PortReserveAsset,
} from "../sdk/src";

// TODO make this a CLI
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
        // "So11111111111111111111111111111111111111112"
    );

    let vaultId = new PublicKey(
        "2Hwkb1L5Gw5yKAEpCkuXDm5avXGCbHw1B7S5TNj8Wd2y"
    );

    let vaultClient = await VaultClient.load(
        provider,
        vaultId,
        DeploymentEnvs.devnetStaging
    );
    let state = vaultClient.getVaultState();

    let userReserveTokenAccount = await SplToken.getAssociatedTokenAddress(
        ASSOCIATED_TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        state.reserveTokenMint,
        wallet.payer.publicKey,
        true
    );

    // let sig = await vaultClient.deposit(
    //     wallet,
    //     100000 * 2,
    //     userReserveTokenAccount
    // );

     let sig = await vaultClient.withdraw(
        wallet,
        199000 
    );

    // let sig = await vaultClient.MyReconcile("port", "deposit", 100000);
    // let sig = await vaultClient.MyReconcile("port", "withdraw", 9000);
    // let sig = await vaultClient.claimPortReward();
    console.log("sig: ", sig);
};

main();
