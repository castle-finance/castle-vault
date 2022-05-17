import Big, { BigSource } from "big.js";
import { AccountInfo, MintInfo } from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";

export class Token {
    constructor(public mint: PublicKey, public mintInfo: MintInfo) {}
}

export class TokenAmount {
    constructor(
        public lamports: Big,
        public decimals: number,
        public mint?: PublicKey
    ) {}

    public static fromTokenAccountInfo(
        tokenAccountInfo: AccountInfo,
        decimals: number
    ): TokenAmount {
        return new TokenAmount(
            Big(tokenAccountInfo.amount.toString()),
            decimals,
            tokenAccountInfo.mint
        );
    }

    public static fromToken(token: Token, lamports: Big): TokenAmount {
        return new TokenAmount(lamports, token.mintInfo.decimals, token.mint);
    }

    public static zero(decimals: number, mint?: PublicKey): TokenAmount {
        return new TokenAmount(Big(0), decimals, mint);
    }

    public getAmount(): number {
        return this.lamports.div(Big(10).pow(this.decimals)).toNumber();
    }

    private static op_verify(a: TokenAmount, b: TokenAmount) {
        if (b.decimals != a.decimals) {
            throw Error(`Decimals do not match: ${b.decimals} / ${a.decimals}`);
        }

        if (b.mint && a.mint && !b.mint.equals(a.mint)) {
            throw Error(`Mints do not match: ${b.mint} / ${a.mint}`);
        }
    }

    public sub(a: TokenAmount): TokenAmount {
        TokenAmount.op_verify(this, a);
        return new TokenAmount(
            this.lamports.sub(a.lamports),
            this.decimals,
            this.mint
        );
    }

    public add(a: TokenAmount): TokenAmount {
        TokenAmount.op_verify(this, a);
        return new TokenAmount(
            this.lamports.add(a.lamports),
            this.decimals,
            this.mint
        );
    }
}

export class Rate {
    constructor(protected value: Big) {}

    public static zero(): Rate {
        return new Rate(Big(0));
    }

    public static fromPercent(percent_value: BigSource): Rate {
        return new Rate(Big(percent_value).div(100));
    }

    public static fromBps(bps_value: BigSource): Rate {
        return new Rate(Big(bps_value).div(10000));
    }

    public toBig(): Big {
        return this.value;
    }

    public toNumber(): number {
        return this.value.toNumber();
    }

    public asPercent(): Big {
        return this.value.mul(100);
    }

    public asBps(): Big {
        return this.value.mul(10000);
    }

    public mul(m: Rate | Big): Rate {
        const result =
            m instanceof Rate ? this.value.mul(m.toBig()) : this.value.mul(m);
        return new Rate(result);
    }

    public add(a: Rate | Big): Rate {
        const result =
            a instanceof Rate ? this.value.add(a.toBig()) : this.value.add(a);
        return new Rate(result);
    }

    public div(d: Rate | Big): Rate {
        const result =
            d instanceof Rate ? this.value.div(d.toBig()) : this.value.div(d);
        return new Rate(result);
    }
}

export class ExchangeRate extends Rate {
    constructor(value: Big, public baseToken: Token, public quoteToken: Token) {
        super(value);
    }

    public convertToQuote(baseAmount: TokenAmount): TokenAmount {
        if (baseAmount.mint && !baseAmount.mint.equals(this.baseToken.mint)) {
            throw Error(
                `Mints do not match: ${this.baseToken.mint} / ${baseAmount.mint}`
            );
        }

        return TokenAmount.fromToken(
            this.quoteToken,
            baseAmount.lamports.div(this.value)
        );
    }

    public convertToBase(quoteAmount: TokenAmount): TokenAmount {
        if (
            quoteAmount.mint &&
            !quoteAmount.mint.equals(this.quoteToken.mint)
        ) {
            throw Error(
                `Mints do not match: ${this.quoteToken.mint} / ${quoteAmount.mint}`
            );
        }

        return TokenAmount.fromToken(
            this.baseToken,
            quoteAmount.lamports.mul(this.value)
        );
    }
}
