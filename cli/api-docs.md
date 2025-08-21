---
title: CLI
category: Jekyll
layout: post
weight: 1
---

# Command-Line Help for `ncn-program-bls-cli`

This document contains the help content for the `ncn-program-bls-cli` command-line program.

## `ncn-program-bls-cli`

A CLI for creating and managing the ncn program

**Usage:** `ncn-program-bls-cli [OPTIONS] --vault <VAULT> <COMMAND>`

###### **Subcommands:**

* `run-keeper` — NCN Keeper
* `crank-register-vaults` — Crank Functions
* `crank-snapshot` — 
* `crank-close-epoch-accounts` — 
* `set-epoch-weights` — 
* `admin-create-config` — Admin
* `admin-register-st-mint` — 
* `admin-set-weight` — 
* `admin-set-tie-breaker` — 
* `admin-set-parameters` — 
* `admin-set-new-admin` — 
* `admin-fund-account-payer` — 
* `create-vault-registry` — Instructions
* `create-operator-registry` — 
* `register-vault` — 
* `register-operator` — 
* `create-epoch-state` — 
* `create-weight-table` — 
* `create-epoch-snapshot` — 
* `create-operator-snapshot` — 
* `snapshot-vault-operator-delegation` — 
* `create-ballot-box` — 
* `operator-cast-vote` — 
* `get-ncn` — Getters
* `get-ncn-operator-state` — 
* `get-vault-ncn-ticket` — 
* `get-ncn-vault-ticket` — 
* `get-vault-operator-delegation` — 
* `get-all-tickets` — 
* `get-all-operators-in-ncn` — 
* `get-all-vaults-in-ncn` — 
* `get-ncn-program-config` — 
* `get-vault-registry` — 
* `get-weight-table` — 
* `get-epoch-state` — 
* `get-epoch-snapshot` — 
* `get-operator-snapshot` — 
* `get-ballot-box` — 
* `get-account-payer` — 
* `get-total-epoch-rent-cost` — 
* `get-consensus-result` — 
* `get-operator-stakes` — 
* `get-vault-stakes` — 
* `get-vault-operator-stakes` — 
* `full-update-vault` — 

###### **Options:**

* `--rpc-url <RPC_URL>` — RPC URL to use

  Default value: `https://api.mainnet-beta.solana.com`
* `--commitment <COMMITMENT>` — Commitment level

  Default value: `confirmed`
* `--priority-fee-micro-lamports <PRIORITY_FEE_MICRO_LAMPORTS>` — Priority fee in micro lamports

  Default value: `1`
* `--transaction-retries <TRANSACTION_RETRIES>` — Amount of times to retry a transaction

  Default value: `0`
* `--ncn-program-id <NCN_PROGRAM_ID>` — NCN program ID

  Default value: `3fKQSi6VzzDUJSmeksS8qK6RB3Gs3UoZWtsQD3xagy45`
* `--restaking-program-id <RESTAKING_PROGRAM_ID>` — Restaking program ID

  Default value: `RestkWeAVL8fRGgzhfeoqFhsqKRchg6aa1XrcH96z4Q`
* `--vault-program-id <VAULT_PROGRAM_ID>` — Vault program ID

  Default value: `Vau1t6sLNxnzB7ZDsef8TLbPLfyZMYXH8WTNqUdm9g8`
* `--token-program-id <TOKEN_PROGRAM_ID>` — Token Program ID

  Default value: `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA`
* `--ncn <NCN>` — NCN Account Address
* `--vault <VAULT>` — Vault Account Address
* `--epoch <EPOCH>` — Epoch - defaults to current epoch
* `--keypair-path <KEYPAIR_PATH>` — keypair path
* `--verbose` — Verbose mode
* `--open-weather-api-key <OPEN_WEATHER_API_KEY>` — Open weather api key



## `ncn-program-bls-cli run-keeper`

NCN Keeper

**Usage:** `ncn-program-bls-cli run-keeper [OPTIONS]`

###### **Options:**

* `--loop-timeout-ms <LOOP_TIMEOUT_MS>` — Maximum time in milliseconds between keeper loop iterations

  Default value: `600000`
* `--error-timeout-ms <ERROR_TIMEOUT_MS>` — Timeout in milliseconds when an error occurs before retrying

  Default value: `10000`



## `ncn-program-bls-cli crank-register-vaults`

Crank Functions

**Usage:** `ncn-program-bls-cli crank-register-vaults`



## `ncn-program-bls-cli crank-snapshot`

**Usage:** `ncn-program-bls-cli crank-snapshot`



## `ncn-program-bls-cli crank-close-epoch-accounts`

**Usage:** `ncn-program-bls-cli crank-close-epoch-accounts`



## `ncn-program-bls-cli set-epoch-weights`

**Usage:** `ncn-program-bls-cli set-epoch-weights`



## `ncn-program-bls-cli admin-create-config`

Admin

**Usage:** `ncn-program-bls-cli admin-create-config [OPTIONS] --ncn-fee-wallet <NCN_FEE_WALLET> --ncn-fee-bps <NCN_FEE_BPS> --minimum-stake <MINIMUM_STAKE>`

###### **Options:**

* `--ncn-fee-wallet <NCN_FEE_WALLET>` — Ncn Fee Wallet Address
* `--ncn-fee-bps <NCN_FEE_BPS>` — Ncn Fee bps
* `--epochs-before-stall <EPOCHS_BEFORE_STALL>` — Epochs before tie breaker can set consensus

  Default value: `10`
* `--valid-slots-after-consensus <VALID_SLOTS_AFTER_CONSENSUS>` — Valid slots after consensus

  Default value: `43200`
* `--epochs-after-consensus-before-close <EPOCHS_AFTER_CONSENSUS_BEFORE_CLOSE>` — Epochs after consensus before accounts can be closed

  Default value: `10`
* `--tie-breaker-admin <TIE_BREAKER_ADMIN>` — Tie breaker admin address
* `--minimum-stake <MINIMUM_STAKE>` — Minimum stake weight required for operators (in lamports)



## `ncn-program-bls-cli admin-register-st-mint`

**Usage:** `ncn-program-bls-cli admin-register-st-mint [OPTIONS]`

###### **Options:**

* `--weight <WEIGHT>` — Weight



## `ncn-program-bls-cli admin-set-weight`

**Usage:** `ncn-program-bls-cli admin-set-weight --weight <WEIGHT>`

###### **Options:**

* `--weight <WEIGHT>` — Weight value



## `ncn-program-bls-cli admin-set-tie-breaker`

**Usage:** `ncn-program-bls-cli admin-set-tie-breaker --weather-status <WEATHER_STATUS>`

###### **Options:**

* `--weather-status <WEATHER_STATUS>` — tie breaker for voting



## `ncn-program-bls-cli admin-set-parameters`

**Usage:** `ncn-program-bls-cli admin-set-parameters [OPTIONS]`

###### **Options:**

* `--epochs-before-stall <EPOCHS_BEFORE_STALL>` — Epochs before tie breaker can set consensus
* `--epochs-after-consensus-before-close <EPOCHS_AFTER_CONSENSUS_BEFORE_CLOSE>` — Epochs after consensus before accounts can be closed
* `--valid-slots-after-consensus <VALID_SLOTS_AFTER_CONSENSUS>` — Slots to which voting is allowed after consensus
* `--starting-valid-epoch <STARTING_VALID_EPOCH>` — Starting valid epoch



## `ncn-program-bls-cli admin-set-new-admin`

**Usage:** `ncn-program-bls-cli admin-set-new-admin [OPTIONS] --new-admin <NEW_ADMIN>`

###### **Options:**

* `--new-admin <NEW_ADMIN>` — New admin address
* `--set-tie-breaker-admin` — Set tie breaker admin



## `ncn-program-bls-cli admin-fund-account-payer`

**Usage:** `ncn-program-bls-cli admin-fund-account-payer --amount-in-sol <AMOUNT_IN_SOL>`

###### **Options:**

* `--amount-in-sol <AMOUNT_IN_SOL>` — Amount of SOL to fund



## `ncn-program-bls-cli create-vault-registry`

Instructions

**Usage:** `ncn-program-bls-cli create-vault-registry`



## `ncn-program-bls-cli create-operator-registry`

**Usage:** `ncn-program-bls-cli create-operator-registry`



## `ncn-program-bls-cli register-vault`

**Usage:** `ncn-program-bls-cli register-vault`



## `ncn-program-bls-cli register-operator`

**Usage:** `ncn-program-bls-cli register-operator [OPTIONS] --operator <OPERATOR>`

###### **Options:**

* `--operator <OPERATOR>` — Operator address
* `--g1-pubkey <G1_PUBKEY>` — G1 public key (32 bytes as hex string) - auto-generated if not provided
* `--g2-pubkey <G2_PUBKEY>` — G2 public key (64 bytes as hex string) - auto-generated if not provided
* `--signature <SIGNATURE>` — BLS signature (64 bytes as hex string) - auto-generated if not provided
* `--keys-file <KEYS_FILE>` — Path to save/load BLS keys JSON file

  Default value: `bls-keys.json`



## `ncn-program-bls-cli create-epoch-state`

**Usage:** `ncn-program-bls-cli create-epoch-state`



## `ncn-program-bls-cli create-weight-table`

**Usage:** `ncn-program-bls-cli create-weight-table`



## `ncn-program-bls-cli create-epoch-snapshot`

**Usage:** `ncn-program-bls-cli create-epoch-snapshot`



## `ncn-program-bls-cli create-operator-snapshot`

**Usage:** `ncn-program-bls-cli create-operator-snapshot --operator <OPERATOR>`

###### **Options:**

* `--operator <OPERATOR>` — Operator address



## `ncn-program-bls-cli snapshot-vault-operator-delegation`

**Usage:** `ncn-program-bls-cli snapshot-vault-operator-delegation --operator <OPERATOR>`

###### **Options:**

* `--operator <OPERATOR>` — Operator address



## `ncn-program-bls-cli create-ballot-box`

**Usage:** `ncn-program-bls-cli create-ballot-box`



## `ncn-program-bls-cli operator-cast-vote`

**Usage:** `ncn-program-bls-cli operator-cast-vote --operator <OPERATOR> --weather-status <WEATHER_STATUS>`

###### **Options:**

* `--operator <OPERATOR>` — Operator address
* `--weather-status <WEATHER_STATUS>` — weather status at solana beach



## `ncn-program-bls-cli get-ncn`

Getters

**Usage:** `ncn-program-bls-cli get-ncn`



## `ncn-program-bls-cli get-ncn-operator-state`

**Usage:** `ncn-program-bls-cli get-ncn-operator-state --operator <OPERATOR>`

###### **Options:**

* `--operator <OPERATOR>` — Operator Account Address



## `ncn-program-bls-cli get-vault-ncn-ticket`

**Usage:** `ncn-program-bls-cli get-vault-ncn-ticket`



## `ncn-program-bls-cli get-ncn-vault-ticket`

**Usage:** `ncn-program-bls-cli get-ncn-vault-ticket`



## `ncn-program-bls-cli get-vault-operator-delegation`

**Usage:** `ncn-program-bls-cli get-vault-operator-delegation --operator <OPERATOR>`

###### **Options:**

* `--operator <OPERATOR>` — Operator Account Address



## `ncn-program-bls-cli get-all-tickets`

**Usage:** `ncn-program-bls-cli get-all-tickets`



## `ncn-program-bls-cli get-all-operators-in-ncn`

**Usage:** `ncn-program-bls-cli get-all-operators-in-ncn`



## `ncn-program-bls-cli get-all-vaults-in-ncn`

**Usage:** `ncn-program-bls-cli get-all-vaults-in-ncn`



## `ncn-program-bls-cli get-ncn-program-config`

**Usage:** `ncn-program-bls-cli get-ncn-program-config`



## `ncn-program-bls-cli get-vault-registry`

**Usage:** `ncn-program-bls-cli get-vault-registry`



## `ncn-program-bls-cli get-weight-table`

**Usage:** `ncn-program-bls-cli get-weight-table`



## `ncn-program-bls-cli get-epoch-state`

**Usage:** `ncn-program-bls-cli get-epoch-state`



## `ncn-program-bls-cli get-epoch-snapshot`

**Usage:** `ncn-program-bls-cli get-epoch-snapshot`



## `ncn-program-bls-cli get-operator-snapshot`

**Usage:** `ncn-program-bls-cli get-operator-snapshot --operator <OPERATOR>`

###### **Options:**

* `--operator <OPERATOR>` — Operator Account Address



## `ncn-program-bls-cli get-ballot-box`

**Usage:** `ncn-program-bls-cli get-ballot-box`



## `ncn-program-bls-cli get-account-payer`

**Usage:** `ncn-program-bls-cli get-account-payer`



## `ncn-program-bls-cli get-total-epoch-rent-cost`

**Usage:** `ncn-program-bls-cli get-total-epoch-rent-cost`



## `ncn-program-bls-cli get-consensus-result`

**Usage:** `ncn-program-bls-cli get-consensus-result`



## `ncn-program-bls-cli get-operator-stakes`

**Usage:** `ncn-program-bls-cli get-operator-stakes`



## `ncn-program-bls-cli get-vault-stakes`

**Usage:** `ncn-program-bls-cli get-vault-stakes`



## `ncn-program-bls-cli get-vault-operator-stakes`

**Usage:** `ncn-program-bls-cli get-vault-operator-stakes`



## `ncn-program-bls-cli full-update-vault`

**Usage:** `ncn-program-bls-cli full-update-vault`



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>

