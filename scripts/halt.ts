import { Connection, PublicKey } from "@solana/web3.js";
import { AnchorProvider, Wallet } from "@project-serum/anchor";

import { DeploymentEnvs } from "@castlefinance/vault-core";

import { VaultClient, VaultFlags } from "../sdk/src";

// TODO make this a CLI
const main = async () => {
    const vaultId = new PublicKey(
        "Bv4d2wWb7myxpjWudHnEMjdJstxjkiWqX61xLhPBrBx" //devnet-staging
        //"3tBqjyYtf9Utb1NNsx4o7AV1qtzHoxsMXgkmat3rZ3y6" //mainnet
    );
    const connection = new Connection(
        // "https://solana-api.syndica.io/access-token/PBhwkfVgRLe1MEpLI5VbMDcfzXThjLKDHroc31shR5e7qrPqQi9TAUoV6aD3t0pg/rpc"
        "https://psytrbhymqlkfrhudd.dev.genesysgo.net:8899/"
    );
    const wallet = Wallet.local();
    // TODO figure out how to use ledger as owner
    const owner = wallet.payer;
    const provider = new AnchorProvider(connection, wallet, {
        commitment: "confirmed",
    });

    const vaultClient = await VaultClient.load(
        provider,
        vaultId,
        DeploymentEnvs.devnetStaging
    );
    console.log("Vault client loaded");

    const txSig = await vaultClient.updateHaltFlags(owner, VaultFlags.HaltAll);
    console.log("Tx sent: ", txSig);
    await connection.confirmTransaction(txSig, "finalized");
    console.log("Tx confirmed: ", txSig);

    await vaultClient.reload();

    console.log("Vault halt flags: ", vaultClient.getHaltFlags());
};

main();
