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

import {
    StakeAccount,
    refreshObligationInstruction,
} from "@castlefinance/port-sdk";

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
    let port = vaultClient.getPort();

    let userReserveTokenAccount = await SplToken.getAssociatedTokenAddress(
        ASSOCIATED_TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        state.reserveTokenMint,
        wallet.payer.publicKey,
        true
    );

    // const raw = await connection.getAccountInfo(
    //     new PublicKey("7QjdPYTdfGiw3PVb5M4raPt2ca1Keodp7kDbCbvshZPk")
    // );

    const raw = await connection.getAccountInfo(
        new PublicKey(port.accounts.vaultPortStakeAccount)
    );
    const stakeAccountData = StakeAccount.fromRaw({
        pubkey: port.accounts.vaultPortStakeAccount,
        account: raw,
    });

    console.log("");
    console.log(
        "userReserveTokenAccount: ",
        userReserveTokenAccount.toString()
    );

    console.log("");
    console.log(
        "port obligation account: ",
        port.accounts.vaultPortObligation.toString()
    );
    console.log(
        "    port reward account: ",
        port.accounts.vaultPortRewardToken.toString()
    );
    console.log(
        "    port subreward account: ",
        port.accounts.vaultPortSubRewardToken.toString()
    );
    console.log(
        "     port stake account: ",
        port.accounts.vaultPortStakeAccount.toString()
    );
    console.log(
        "stake account deposit: ",
        stakeAccountData.getDepositAmount().toU64().toNumber()
    );
    console.log(
        "unclaimed reward: ",
        stakeAccountData.getUnclaimedReward().toU64().toNumber()
    );

    console.log("");
    console.log("liquiditySupply:", port.accounts.liquiditySupply.toString());
    console.log("lpTokenAccount:", port.accounts.lpTokenAccount.toString());
    console.log(
        "rewardTokenMint:",
        port.accounts.stakingRewardTokenMint.toString()
    );
    console.log("stakingPoolId: ", port.accounts.stakingPool.toString());
    console.log(
        "stakingAuthorith: ",
        port.accounts.stakingProgamAuthority.toString()
    );
    console.log(
        "rewardTokenPool: ",
        port.accounts.stakingRewardPool.toString()
    );
    console.log(
        "subRewardTokenPool: ",
        port.accounts.stakingSubRewardPool.toString()
    );
    console.log(
        "subRewardTokenPoolMint: ",
        port.accounts.stakingSubRewardTokenMint.toString()
    );

    console.log("");
    console.log("vault value: ", (await vaultClient.getTotalValue()).lamports.toNumber());
};

main();
