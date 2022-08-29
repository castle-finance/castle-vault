import { Provider, Wallet } from "@project-serum/anchor";
import { Connection, PublicKey, Transaction } from "@solana/web3.js";
import {
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    Token as SplToken,
} from "@solana/spl-token";
import * as anchor from "@project-serum/anchor";

import { VaultClient } from "../sdk/lib";
import { DeploymentEnvs } from "@castlefinance/vault-core";

const main = async () => {
    let env: any = DeploymentEnvs.devnetStaging;
    // let env: any = DeploymentEnvs.mainnet;

    let connection: Connection;
    let vaultId: PublicKey;
    if (env == DeploymentEnvs.devnetStaging) {
        connection = new Connection("https://api.devnet.solana.com");
        vaultId = new PublicKey("FmaTu3heJTGsCFUsBondGRHNPx7bG5brYht8XBmposFC");
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

    const provider = new Provider(connection, wallet, {
        commitment: "confirmed",
    });

    let vaultClient = await VaultClient.load(provider, vaultId, env);

    let args = process.argv.slice(2);

    if (args[0] == "show") {
        console.log("allocations:");
        console.log(
            "    reserve:",
            (
                await vaultClient.getVaultReserveTokenAccountValue()
            ).lamports.toNumber()
        );
        console.log(
            "  solend:",
            (
                await vaultClient.getVaultSolendLpTokenAccountValue()
            ).lamports.toNumber()
        );
        console.log(
            "    port:",
            (
                await vaultClient.getVaultPortLpTokenAccountValue()
            ).lamports.toNumber()
        );
        console.log(
            "    jet:",
            (
                await vaultClient.getVaultJetLpTokenAccountValue()
            ).lamports.toNumber()
        );
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
    } else if (args[0] == "rebalance") {
        let sig = await vaultClient.rebalance({
            solend: 0,
            port: 4000,
            jet: 6000,
        });
        console.log("sig: ", sig);
    } else if (args[0] == "refresh") {
        let tx = new Transaction().add(vaultClient.getRefreshIx());
        await vaultClient.program.provider.send(tx);
    } else if (args[0] == "reconcile") {
        let pool = args[1];
        let amount = new anchor.BN(parseFloat(args[2]));
        let tx = new Transaction().add(
            vaultClient.getRefreshIx()
        );

        if(pool == "solend") {
            tx.add(vaultClient.getReconcileSolendIx(amount));
        } else if(pool == "port") {
            tx.add(vaultClient.getReconcilePortIx(amount));
        } else if(pool == "jet") {
            tx.add(vaultClient.getReconcileJetIx(amount));
        } else {}

        await vaultClient.program.provider.send(tx);
    }else if (args[0] == "update_flags") {
        await vaultClient.updateFlags(owner, 0);
    }
};

main();
