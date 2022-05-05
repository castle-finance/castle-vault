# Castle Vault SDK

Full technical documentation can be found [here](https://castle-finance.github.io/castle-vault/sdk/).

## Installation

`yarn add @castlefinance/vault-sdk`

## Getting Help

Join [our Discord channel](https://discord.castle.finance) and post a message in #developers

## Examples

### Create the client

```
import { VaultClient, VaultConfig } from '@castlefinance/vault-sdk'

// Pull down the appropriate vault from the API.
const configResponse = await fetch('https://api.castle.finance/configs')
const vaults = (await response.json()) as VaultConfig[]
const vault = vaults.find(
  (v) => v.deploymentEnv == 'mainnet' && v.token_label == 'USDC'
)


// Create the vault client
const vaultClient = await VaultClient.load(
  new anchor.Provider(...),
  vault.vault_id,
  vault.deploymentEnv
)
```

### Deposit

```
// Get the users reserve token ATA
const userReserveToken = await splToken.Token.getAssociatedTokenAddress(
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  vaultClient.getVaultState().reserveTokenMint,
  reserveTokenOwner, // e.g. wallet.pubkey or DAO's account
  true
);

// Deposit into the vault
const sig = await vaultClient.deposit(wallet, amount, userReserveToken)
```

### Withdraw

`const sig = await vaultClient.withdraw(wallet, amount)`
