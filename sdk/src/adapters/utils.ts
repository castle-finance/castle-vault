import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { Token as SplToken, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { Token } from "../utils";

export async function getToken(
    connection: Connection,
    mintAddress: PublicKey
): Promise<Token> {
    const splToken = new SplToken(
        connection,
        mintAddress,
        TOKEN_PROGRAM_ID,
        Keypair.generate() // dummy signer since we aren't making any txs
    );
    return new Token(mintAddress, await splToken.getMintInfo());
}
