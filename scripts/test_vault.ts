import { AnchorProvider, Wallet } from "@project-serum/anchor";
import { Connection, PublicKey, Transaction, Keypair } from "@solana/web3.js";
import {
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    Token as SplToken,
} from "@solana/spl-token";
import * as anchor from "@project-serum/anchor";

import { VaultClient } from "../sdk";
import { DeploymentEnvs } from "@castlefinance/vault-core";

async function getSplTokenAccountBalance(
    program: any,
    mint: PublicKey,
    account: PublicKey
): Promise<number> {
    const tokenMint = new SplToken(
        program.provider.connection,
        mint,
        TOKEN_PROGRAM_ID,
        Keypair.generate()
    );
    return (await tokenMint.getAccountInfo(account)).amount.toNumber();
}

const main = async () => {
    let env: any = DeploymentEnvs.devnetStaging;
    // let env: any = DeploymentEnvs.mainnet;

    let connection: Connection;
    let vaultId: PublicKey;
    if (env == DeploymentEnvs.devnetStaging) {
        connection = new Connection("https://api.devnet.solana.com");
        vaultId = new PublicKey("EfZQifTFaXsuZr1zykh876UeD1ay5AFu61hEmEcNJPaL");
        // vaultId = new PublicKey("7MXreZLSP1Xm9EiLvEf2gZKsQqeuyUHuL54vVSyvFfZi");
    } else if (env == DeploymentEnvs.mainnet) {
        connection = new Connection(
            "https://solana-api.syndica.io/access-token/lBo6ki5ZTs0yyhuG44oFo4Hq49BQdO6udrd2ZSrTCt4M8u2ipRNNS5WDply9zgaF/rpc"
        );
        vaultId = new PublicKey("3tBqjyYtf9Utb1NNsx4o7AV1qtzHoxsMXgkmat3rZ3y6");
    } else {
        return;
    }

    // TODO replace with vault owner
    const wallet = Wallet.local();
    const owner = wallet.payer;

    const provider = new AnchorProvider(connection, wallet, {
        commitment: "confirmed",
    });

    let vaultClient = await VaultClient.load(provider, vaultId, env);

    let args = process.argv.slice(2);

    if (args[0] == "show") {
        console.log(
            "vault total value:",
            vaultClient.getVaultState().value.value.toNumber()
        );
        console.log("allocations:");
        console.log(
            "    Reserve:",
            (
                await vaultClient.getVaultReserveTokenAccountValue()
            ).lamports.toNumber()
        );
        console.log(
            "  Solend:",
            (
                await vaultClient.getVaultSolendLpTokenAccountValue()
            ).lamports.toNumber()
        );
        console.log(
            "    Port:",
            (
                await vaultClient.getVaultPortLpTokenAccountValue()
            ).lamports.toNumber()
        );

        const claimedRewardAmount = await getSplTokenAccountBalance(
            vaultClient.program,
            vaultClient.getPort().accounts.stakingRewardTokenMint,
            vaultClient.getPort().accounts.vaultPortRewardToken
        );
        console.log("Port claimed & unsold reward:", claimedRewardAmount);

        const referralAccountInfo =
            await vaultClient.getReferralFeeReceiverAccountInfo();
        const feeAccountInfo = await vaultClient.getFeeReceiverAccountInfo();
        const actualReferralFees = referralAccountInfo.amount.toNumber();
        const actualMgmtFees = feeAccountInfo.amount.toNumber();
        console.log("Collected fees:");
        console.log("    mgmt: ", actualMgmtFees);
        console.log("    referal: ", actualReferralFees);

        const vaultConfig = vaultClient.getVaultConfig();
        console.log("Config: ", vaultConfig);
    } else if (args[0] == "deposit") {
        let value = parseFloat(args[1]);
        console.log("deposit: ", value);
        let reserveTokenMint = vaultClient.getReserveTokenMint();
        let userReserveTokenAccount = await SplToken.getAssociatedTokenAddress(
            ASSOCIATED_TOKEN_PROGRAM_ID,
            TOKEN_PROGRAM_ID,
            reserveTokenMint,
            owner.publicKey,
            true
        );
        let sig = await vaultClient.deposit(
            wallet,
            value,
            userReserveTokenAccount
        );
        console.log("sig: ", sig);
    } else if (args[0] == "withdraw") {
        let value = parseFloat(args[1]);
        console.log("withdraw: ", value);
        let sig = await vaultClient.withdraw(wallet, value);
        console.log("sig: ", sig);
    } else if (args[0] == "rebalance") {
        let sig = await vaultClient.rebalance({
            solend: 6000,
            port: 4000,
        });
        console.log("sig: ", sig);
    } else if (args[0] == "refresh") {
        let tx = new Transaction();
        (await vaultClient.getRefreshIxs()).forEach((element) => {
            tx.add(element);
        });
        let sig = await provider.sendAndConfirm(tx);
        console.log("sig: ", sig);
    } else if (args[0] == "reconcile") {
        let pool = args[1];
        let amount = new anchor.BN(parseFloat(args[2]));
        let tx = new Transaction();
        (await vaultClient.getRefreshIxs()).forEach((element) => {
            tx.add(element);
        });
        if (pool == "solend") {
            tx.add(
                await vaultClient
                    .getSolend()
                    .getReconcileIx(
                        vaultClient.program,
                        vaultId,
                        vaultClient.getVaultState(),
                        amount
                    )
            );
        } else if (pool == "port") {
            tx.add(
                await vaultClient
                    .getPort()
                    .getReconcileIx(
                        vaultClient.program,
                        vaultId,
                        vaultClient.getVaultState(),
                        amount
                    )
            );
        } else {
        }
        await vaultClient.program.provider.send(tx);
    } else if (args[0] == "yield_sources_on") {
        console.log("All yield sources enabled");
        await vaultClient.updateYieldSourceFlags(owner, 0b11);
    } else if (args[0] == "yield_sources_off") {
        console.log("All yield sources disabled");
        await vaultClient.updateYieldSourceFlags(owner, 0b0);
    } else if (args[0] == "halt_flags_on") {
        console.log("Vault halted");
        await vaultClient.updateHaltFlags(owner, 0b111);
    } else if (args[0] == "halt_flags_off") {
        console.log("Vault enabled");
        await vaultClient.updateHaltFlags(owner, 0b0);
    } else if (args[0] == "claim_port_reward") {
        console.log("Claim port reward");
        await vaultClient.claimPortReward();
    } else if (args[0] == "update_fees") {
        let feeRateBps = parseFloat(args[1]);

        const oldConfig = vaultClient.getVaultConfig();
        const newConfig = {
            ...oldConfig,
            feeCarryBps: feeRateBps,
            feeMgmtBps: feeRateBps,
            referralFeePct: 0,
        };
        const txSig = await vaultClient.updateConfig(owner, newConfig);
    }
};

main();
