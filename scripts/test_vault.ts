import { AnchorProvider, Wallet, WalletAdaptor } from "@project-serum/anchor";
import { Connection, PublicKey, Transaction, Keypair } from "@solana/web3.js";
import {
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    Token as SplToken,
} from "@solana/spl-token";
import * as anchor from "@project-serum/anchor";

import { VaultClient } from "../sdk";
import { LedgerWallet } from "./utils/ledger";

import { DeploymentEnvs } from "@castlefinance/vault-core";

const CONNECTION_DEVNET = new Connection("https://api.devnet.solana.com");
const CONNECTION_MAINNET = new Connection(
    "https://solana-api.syndica.io/access-token/PBhwkfVgRLe1MEpLI5VbMDcfzXThjLKDHroc31shR5e7qrPqQi9TAUoV6aD3t0pg/rpc"
);

const VAULT_ID_DEVNET = new PublicKey(
    "FmaTu3heJTGsCFUsBondGRHNPx7bG5brYht8XBmposFC"
);
const VAULT_ID_MAINNET = new PublicKey(
    "3tBqjyYtf9Utb1NNsx4o7AV1qtzHoxsMXgkmat3rZ3y6"
);

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
    let connection = CONNECTION_DEVNET;
    let vaultId = VAULT_ID_DEVNET;

    let args = process.argv.slice(2);
    if (args.includes("--mainnet")) {
        env = DeploymentEnvs.mainnet;
        connection = CONNECTION_MAINNET;
        vaultId = VAULT_ID_MAINNET;
        args.splice(args.indexOf("--mainnet"), 1);
    }

    // TODO replace with vault owner
    const wallet = Wallet.local();

    let owner: Keypair | WalletAdaptor = wallet.payer;
    if (args.includes("--ledger")) {
        const ledgerWallet = new LedgerWallet(0);
        await ledgerWallet.connect();
        owner = ledgerWallet as WalletAdaptor;
        args.splice(args.indexOf("--ledger"), 1);
    }

    const provider = new AnchorProvider(connection, wallet, {
        commitment: "confirmed",
    });

    let vaultClient = await VaultClient.load(provider, vaultId, env);

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
            solend: 4000,
            port: 6000,
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
