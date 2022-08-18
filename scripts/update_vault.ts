import { AnchorProvider, Wallet } from "@project-serum/anchor";
import { Connection, PublicKey, Transaction } from "@solana/web3.js";

import { VaultClient } from "../sdk";

import { DeploymentEnvs } from "@castlefinance/vault-core";

const main = async () => {
    let env: any = DeploymentEnvs.devnetStaging;

    let connection: Connection;
    let vaultId: PublicKey;
    if (env == DeploymentEnvs.devnetStaging) {
        connection = new Connection("https://api.devnet.solana.com");

        vaultId = new PublicKey("FmaTu3heJTGsCFUsBondGRHNPx7bG5brYht8XBmposFC");
    } else if (env == DeploymentEnvs.mainnet) {
        connection = new Connection(
            "https://solana-api.syndica.io/access-token/lBo6ki5ZTs0yyhuG44oFo4Hq49BQdO6udrd2ZSrTCt4M8u2ipRNNS5WDply9zgaF/rpc"
        );

        // Mainnet vault
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

    console.log("All vault actions suspended");
    await vaultClient.updateHaltFlags(owner, 0b111);

    console.log("All yield sources enabled");
    await vaultClient.updateYieldSourceFlags(owner, 0b11);

    // Initialize accounts for Port staking feature
    console.log("initializePortAdditionalState");
    await vaultClient.initializePortAdditionalState(wallet, owner);

    // We must completely reload the vault for the new yield source flags to take effect
    vaultClient = await VaultClient.load(provider, vaultId, env);

    console.log("initializePortRewardAccounts");
    await vaultClient.initializePortRewardAccounts(wallet, owner);

    // Update new state variables
    const tx = new Transaction().add(
        await vaultClient.program.methods
            .syncLpTokenSupply()
            .accounts({
                vault: vaultId,
                lpTokenMint: vaultClient.getVaultState().lpTokenMint,
                owner: owner.publicKey,
            })
            .instruction()
    );
    await vaultClient.program.provider.sendAll([{ tx: tx, signers: [owner] }]);

    // Initialize Orca DEX for selling staking rewards
    // Not testable on devnet? Because Orca does not have the right market on devnet
    if (env == DeploymentEnvs.mainnet) {
        // Initialize Orca DEX
        await vaultClient.initializeOrcaLegacy(wallet, owner, env);
        await vaultClient.initializeOrcaLegacyMarket(wallet, owner);
    }

    // Re-enable vault actions
    await vaultClient.updateHaltFlags(owner, 0);
};

main();
