import { assert } from "chai";
import {
  Cluster,
  Connection,
  clusterApiUrl,
  LAMPORTS_PER_SOL,
  PublicKey,
} from "@solana/web3.js";
import { NATIVE_MINT } from "@solana/spl-token";
import { Wallet, Provider } from "@project-serum/anchor";

import { VaultClient } from "../src";

describe("VaultClient", () => {
  const cluster: Cluster = "devnet";
  const connection = new Connection(clusterApiUrl(cluster));
  const wallet = Wallet.local();
  const provider = new Provider(connection, wallet, { commitment: "confirmed" });

  const depositAmount = LAMPORTS_PER_SOL * 0.1;

  let vaultClient: VaultClient;

  before(async () => {
    //const sig = await connection.requestAirdrop(wallet.publicKey, LAMPORTS_PER_SOL);
    //await connection.confirmTransaction(sig, "confirmed");
  });

  it("loads devnet sol vault", async () => {
    const vaultId = new PublicKey("81krfC8ptDbjwY5bkur1SqHYKLPxGQLYBQEUv5zhojUW");
    vaultClient = await VaultClient.load(provider, cluster, NATIVE_MINT, vaultId);
    assert.isNotNull(vaultClient);

    console.log("Total value: ", await vaultClient.getTotalValue());
    console.log("Vault APY: ", await vaultClient.getApy());
    console.log("Jet APY: ", await vaultClient.jet.getApy());
    console.log("Port APY: ", await vaultClient.port.getApy());
    console.log("Solend APY: ", await vaultClient.solend.getApy());
  });

  it("deposits", async () => {
    const startUserValue = await vaultClient.getUserValue(wallet.publicKey);

    const sigs = await vaultClient.deposit(wallet, depositAmount, wallet.publicKey);
    await connection.confirmTransaction(sigs[sigs.length - 1], "finalized");
    const endUserValue = await vaultClient.getUserValue(wallet.publicKey);

    assert.equal(endUserValue - startUserValue, depositAmount);
  });

  it("withdraws", async () => {
    const startUserValue = await vaultClient.getUserValue(wallet.publicKey);

    const userLpTokenAccount = await vaultClient.getUserLpTokenAccount(
      wallet.publicKey
    );

    const sigs = await vaultClient.withdraw(wallet, depositAmount, userLpTokenAccount);
    await connection.confirmTransaction(sigs[sigs.length - 1], "finalized");

    const endUserValue = await vaultClient.getUserValue(wallet.publicKey);
    assert.equal(startUserValue - endUserValue, depositAmount);
  });

  it("rebalances", async () => {
    const sigs = await vaultClient.rebalance();
    const result = await connection.confirmTransaction(
      sigs[sigs.length - 1],
      "finalized"
    );
    assert.isNull(result.value.err);
  });
});
