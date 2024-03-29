import Transport from "@ledgerhq/hw-transport";
import TransportNodeHid from "@ledgerhq/hw-transport-node-hid";
import { PublicKey, Transaction } from "@solana/web3.js";
import * as utils from "./utils";

export class LedgerWallet {
    public publicKey: PublicKey | null;
    private transport: Transport | null;
    private derivationPath: Buffer;

    constructor(account?: number, change?: number) {
        this.derivationPath = utils.getDerivationPath(account, change);
    }

    async connect(): Promise<void> {
        try {
            this.transport = await TransportNodeHid.create();
            this.publicKey = await utils.getPublicKey(
                this.transport,
                this.derivationPath
            );
            this.transport.on("disconnect", this.disconnectCallback);

            console.log(
                "Loaded wallet from ledger: ",
                this.publicKey.toString()
            );
        } catch (error) {
            console.log("Failed to connect to ledger: ", error);
            throw error;
        }
    }

    async disconnect(): Promise<void> {
        if (this.transport != null) {
            await this.transport.close();
        }
    }

    async signTransaction(transaction: Transaction): Promise<Transaction> {
        try {
            if (this.transport == null || this.publicKey == null) {
                throw new Error("Not connected to ledger.");
            }

            const signature = await utils.signTransaction(
                this.transport,
                transaction,
                this.derivationPath
            );
            transaction.addSignature(this.publicKey, signature);
            return transaction;
        } catch (error: any) {
            console.log(
                "Failed to sign transaction using ledger wallet: ",
                error
            );
            throw error;
        }
    }

    async signAllTransactions(txs: Transaction[]): Promise<Transaction[]> {
        return await Promise.all(
            txs.map(async (tx): Promise<Transaction> => {
                return await this.signTransaction(tx);
            })
        );
    }

    private disconnectCallback = () => {
        if (this.transport != null) {
            this.transport.off("disconnect", this.disconnectCallback);
            this.transport = null;
            this.publicKey = null;
            console.log("Ledger wallet disconnected.");
        }
    };
}
