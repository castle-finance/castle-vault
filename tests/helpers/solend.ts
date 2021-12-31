import { Provider } from "@project-serum/anchor";

const PROGRAM_ID = "BwTGCAdzPncEFqP5JBAeCLRWKE8MDVvbGDVMD7XX2fvu";

export class Solend {
    provider: Provider;

    constructor(provider: Provider) {
        this.provider = provider;
    }
}