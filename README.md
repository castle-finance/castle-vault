# Castle Finance Lending Aggregator

## Build, Deploy and Test

First, install dependencies:

```
$ npm install
```

Make sure you have your local solana validator running if you want to deploy the program locally:

```
$ solana-test-validator
```

> If you are on Apple Sillicon M1 chip, you will have to build Solana from the source. See [this document](https://docs.solana.com/cli/install-solana-cli-tools#build-from-source) for more details
> Next, we will build and deploy the program via Anchor.

Build the program:

```
$ anchor build
```

Deploy the program:

```
$ anchor deploy
```

Finally, run the test:

TODO add instructions on downloading and building integration test dependencies

```
$ anchor test
```
