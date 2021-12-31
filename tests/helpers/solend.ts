import * as solend from "@dbricks/dbricks-solend";
import * as anchor from "@project-serum/anchor";
import { AccountLayout, MintLayout, Token, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram, SYSVAR_CLOCK_PUBKEY } from "@solana/web3.js";


export class Solend {
    provider: anchor.Provider;
    lendingProgram: PublicKey;

    constructor(provider: anchor.Provider, programId: PublicKey) {
        this.provider = provider;
        this.lendingProgram = programId;
    }

    async initLendingMarket(owner: PublicKey, payer: Keypair, pythProgramId: PublicKey, switchboardProgramId: PublicKey) {
        const lendingMarketAccount = anchor.web3.Keypair.generate();
        const balanceNeeded = await this.provider.connection.getMinimumBalanceForRentExemption(solend.LENDING_MARKET_SIZE);

        const initTx = new anchor.web3.Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: lendingMarketAccount.publicKey,
                lamports: balanceNeeded,
                space: solend.LENDING_MARKET_SIZE,
                programId: this.lendingProgram,
            })
        ).add(
            solend.initLendingMarketInstruction(
                owner,
                quoteCurrency("USD"),
                lendingMarketAccount.publicKey,
                pythProgramId,
                switchboardProgramId,
                this.lendingProgram
            )
        );
        await this.provider.send(initTx, [payer, lendingMarketAccount]);
        return lendingMarketAccount;
    }

    async addReserve(
        owner: Keypair, 
        payer: Keypair, 
        reserveTokenMint: Token, 
        reserve: Keypair, 
        collateralMint: Keypair,
        liquiditySupply: Keypair,
        pythProduct: PublicKey,
        pythPrice: PublicKey,
        switchboardFeed: PublicKey,
        lendingMarket: PublicKey,
        lendingMarketAuthority: PublicKey,
    ) {
        const ownerReserveTokenAccount = await reserveTokenMint.createAccount(owner.publicKey);
        const liquidityAmount = 10;
        await reserveTokenMint.mintTo(ownerReserveTokenAccount, owner, [], liquidityAmount);

        const collateralSupply = anchor.web3.Keypair.generate();
        const liquidityFeeReceiver = anchor.web3.Keypair.generate();
        const userCollateral = anchor.web3.Keypair.generate();
        const userTransferAuthority = anchor.web3.Keypair.generate();

        const reserveBalance = await this.provider.connection.getMinimumBalanceForRentExemption(solend.RESERVE_SIZE);
        const mintBalance = await this.provider.connection.getMinimumBalanceForRentExemption(MintLayout.span);
        const accountBalance = await this.provider.connection.getMinimumBalanceForRentExemption(AccountLayout.span);

        const tx1 = new anchor.web3.Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: reserve.publicKey,
                lamports: reserveBalance,
                space: solend.RESERVE_SIZE,
                programId: this.lendingProgram,
            })
        ).add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: collateralMint.publicKey,
                lamports: mintBalance,
                space: MintLayout.span,
                programId: TOKEN_PROGRAM_ID,
            })
        ).add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: collateralSupply.publicKey,
                lamports: accountBalance,
                space: AccountLayout.span,
                programId: TOKEN_PROGRAM_ID,
            })
        ).add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: userCollateral.publicKey,
                lamports: accountBalance,
                space: AccountLayout.span,
                programId: TOKEN_PROGRAM_ID,
            })
        );
        const tx2 = new anchor.web3.Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: liquiditySupply.publicKey,
                lamports: accountBalance,
                space: AccountLayout.span,
                programId: TOKEN_PROGRAM_ID,
            })
        ).add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: liquidityFeeReceiver.publicKey,
                lamports: accountBalance,
                space: AccountLayout.span,
                programId: TOKEN_PROGRAM_ID,
            })
        );

        const reserveConfig = {
            optimalUtilizationRate: 80,
            loanToValueRatio: 50,
            liquidationBonus: 5,
            liquidationThreshold: 55,
            minBorrowRate: 0,
            optimalBorrowRate: 4,
            maxBorrowRate: 30,
            fees: {
                /// 0.00001% (Aave borrow fee)
                borrowFeeWad: 100_000_000_000n,
                /// 0.3% (Aave flash loan fee)
                flashLoanFeeWad: 3_000_000_000_000_000n,
                hostFeePercentage: 20,
            },
            depositLimit: 100_000_000n,
            borrowLimit: 100_000_000n,
            feeReceiver: liquidityFeeReceiver.publicKey,
        };

        const tx3 = new anchor.web3.Transaction().add(
            Token.createApproveInstruction(
                TOKEN_PROGRAM_ID,
                ownerReserveTokenAccount,
                userTransferAuthority.publicKey,
                owner.publicKey,
                [],
                liquidityAmount,
            )
        ).add(
            solend.initReserveInstruction(
                liquidityAmount,
                reserveConfig,
                ownerReserveTokenAccount,
                userCollateral.publicKey,
                reserve.publicKey,
                reserveTokenMint.publicKey,
                liquiditySupply.publicKey,
                liquidityFeeReceiver.publicKey,
                pythProduct,
                pythPrice,
                collateralMint.publicKey,
                collateralSupply.publicKey,
                lendingMarket,
                lendingMarketAuthority,
                owner.publicKey,
                userTransferAuthority.publicKey,
                switchboardFeed,
                this.lendingProgram,
            )
        );
        await this.provider.sendAll([
            {tx: tx1, signers: [payer, reserve, collateralMint, collateralSupply, userCollateral]},
            {tx: tx2, signers: [payer, liquiditySupply, liquidityFeeReceiver]},
            {tx: tx3, signers: [owner, userTransferAuthority]},
        ]);
    }
}

const quoteCurrency = (s: string) => {
    const buf = Buffer.alloc(32);
    const strBuf = Buffer.from(s);
    strBuf.copy(buf, 0);
    return buf;
};