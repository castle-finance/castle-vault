import { AnchorProvider, Wallet, WalletAdaptor } from "@castlefinance/anchor";
import { Connection, PublicKey, Transaction, Keypair } from "@solana/web3.js";

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

    const wallet = Wallet.local();

    let owner: Keypair | WalletAdaptor = wallet.payer;
    if (args.includes("--ledger")) {
        const ledgerWallet = new LedgerWallet(0);
        await ledgerWallet.connect();
        owner = ledgerWallet as WalletAdaptor;
        args.splice(args.indexOf("--ledger"), 1);
    }

    const provider = new AnchorProvider(connection, wallet, {
        commitment: "finalized",
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
