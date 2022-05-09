# Castle Vault

This repository contains the open-source code for the Castle Vault program and Typescript SDK. More information can be found on [our website here](https://www.castle.finance).

## Documentation

General Castle docs can be found [here](https://docs.castle.finance/).

SDK docs can be found [here](https://github.com/castle-finance/castle-vault/sdk/README.md)

## Getting Help

Join [our Discord channel](https://discord.castle.finance) and post a message in #developers

## Setup, Build, and Test

First, install dependencies:

```
$ yarn install
```

And install Anchor by following the [instructions here](https://project-serum.github.io/anchor/getting-started/installation.html).

Build the program:

```
$ anchor build
```

Finally, run the tests:

```
$ cargo test
$ anchor test
```

## Security

Our security policy can be found [here](https://docs.castle.finance/security-policy)

## Version semantics

eg. v1.2.3 → (1: major, 2: minor, 3: patch)

Major version bump indicates a backwards-incompatible change in the program API

Minor version bump indicates a backwards-incompatible change in the SDK API

Patch versions are bumped for any backwards-compatible change
