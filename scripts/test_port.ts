import { Provider, Wallet } from "@project-serum/anchor";
import * as pyth from "@pythnetwork/client"
import {
    PublicKey,
    Connection,
} from "@solana/web3.js";

import {
    DEFAULT_PORT_LENDING_MARKET,
    Environment,
    MintId,
    Port,
    TokenAccount,
} from "@castlefinance/port-sdk";

// TODO make this a CLI
const main = async () => {
    const connection = new Connection(
        "https://solana-api.syndica.io/access-token/PBhwkfVgRLe1MEpLI5VbMDcfzXThjLKDHroc31shR5e7qrPqQi9TAUoV6aD3t0pg/rpc"
    );

    const reserveMint = new PublicKey(
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" // USDC
        // "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" // USDT
        // "So11111111111111111111111111111111111111112" // mSOL
    );

    const env = Environment.forMainNet();
    const market = DEFAULT_PORT_LENDING_MARKET;
    const client = new Port(connection, env, market);
    const reserveContext = await client.getReserveContext();
    const reserve = reserveContext.getByAssetMintId(MintId.of(reserveMint));
    const stakingPools = await client.getStakingPoolContext();
    const stakingPoolId = await reserve.getStakingPoolId();
    const targetStakingPool = stakingPools.getStakingPool(stakingPoolId);
    const rewardMintRaw = await connection.getAccountInfo(
        targetStakingPool.getRewardTokenPool()
    );
    const rewardTokenMint = TokenAccount.fromRaw({
        pubkey: targetStakingPool.getRewardTokenPool(),
        account: rewardMintRaw,
    });
    const [stakingProgamAuthority] = await PublicKey.findProgramAddress(
        [targetStakingPool.getId().toBuffer()],
        env.getStakingProgramPk()
    );

    const subRewardPool = targetStakingPool.getSubRewardTokenPool();
    let subrewardMint = undefined;
    if (subRewardPool != undefined) {
        const subrewardMintRaw = await connection.getAccountInfo(
            targetStakingPool.getSubRewardTokenPool()
        );
        subrewardMint = TokenAccount.fromRaw({
            pubkey: targetStakingPool.getSubRewardTokenPool(),
            account: subrewardMintRaw,
        }).getMintId();
    }

    console.log("stakingPool", targetStakingPool.getId().toString());
    console.log(
        "rewardPool",
        targetStakingPool.getRewardTokenPool().toString()
    );
    console.log("rewardTokenMint", rewardTokenMint.getMintId().toString());

    if (subRewardPool != undefined) {
        console.log("subRewardPool", subRewardPool.toString());
        console.log("subRewardTokenMint", subrewardMint.toString());
    }

    console.log("rate_per_slot: ", targetStakingPool.getRatePerSlot().getRaw().toNumber());
    console.log("pool size: ", targetStakingPool.getPoolSize().getRaw().toNumber());

    console.log("pyth key:", pyth.getPythProgramKeyForCluster("mainnet-beta"));
    

    let con = new pyth.PythHttpClient(connection, pyth.getPythProgramKeyForCluster("mainnet-beta") );
    const data = await con.getData();
    const price = data.productPrice.get('Crypto.PORT/USD')!;

    console.log(price);
};

main();
