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
* `crank-snapshot-unupdated` — 
* `admin-create-config` — Admin
* `admin-register-st-mint` — 
* `admin-set-tie-breaker` — 
* `admin-set-parameters` — 
* `admin-set-new-admin` — 
* `admin-fund-account-payer` — 
* `create-vault-registry` — Instructions
* `create-vote-counter` — 
* `register-vault` — 
* `register-operator` — 
* `create-snapshot` — 
* `snapshot-vault-operator-delegation` — 
* `cast-vote` — Cast a vote using BLS multi-signature aggregation
* `generate-vote-signature` — Generate BLS signature for vote aggregation
* `aggregate-signatures` — Aggregate multiple BLS signatures for voting
* `get-ncn` — Getters
* `get-ncn-operator-state` — 
* `get-vault-ncn-ticket` — 
* `get-ncn-vault-ticket` — 
* `get-vault-operator-delegation` — 
* `get-all-tickets` — 
* `get-all-operators-in-ncn` — 
* `get-all-vaults-in-ncn` — 
* `get-all-ncn-operator-accounts` — 
* `get-ncn-program-config` — 
* `get-vault-registry` — 
* `get-vote-counter` — 
* `get-snapshot` — 
* `get-operator-snapshot` — 
* `get-account-payer` — 
* `get-total-epoch-rent-cost` — 
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



## `ncn-program-bls-cli crank-snapshot-unupdated`

**Usage:** `ncn-program-bls-cli crank-snapshot-unupdated [OPTIONS]`

###### **Options:**

* `--verbose` — Show detailed progress information



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
* `--minimum-stake <MINIMUM_STAKE>` — Minimum stake required for operators (in lamports)



## `ncn-program-bls-cli admin-register-st-mint`

**Usage:** `ncn-program-bls-cli admin-register-st-mint`



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



## `ncn-program-bls-cli create-vote-counter`

**Usage:** `ncn-program-bls-cli create-vote-counter`



## `ncn-program-bls-cli register-vault`

**Usage:** `ncn-program-bls-cli register-vault`



## `ncn-program-bls-cli register-operator`

**Usage:** `ncn-program-bls-cli register-operator [OPTIONS] --operator <OPERATOR>`

###### **Options:**

* `--operator <OPERATOR>` — Operator address
* `--g1-pubkey <G1_PUBKEY>` — G1 public key (32 bytes as hex string) - auto-generated if not provided
* `--g2-pubkey <G2_PUBKEY>` — G2 public key (64 bytes as hex string) - auto-generated if not provided
* `--signature <SIGNATURE>` — BLS signature (64 bytes as hex string) - auto-generated if not provided (deprecated, will be auto-generated)
* `--keys-file <KEYS_FILE>` — Path to save/load BLS keys JSON file

  Default value: `bls-keys.json`



## `ncn-program-bls-cli create-snapshot`

**Usage:** `ncn-program-bls-cli create-snapshot`



## `ncn-program-bls-cli snapshot-vault-operator-delegation`

**Usage:** `ncn-program-bls-cli snapshot-vault-operator-delegation --operator <OPERATOR>`

###### **Options:**

* `--operator <OPERATOR>` — Operator address



## `ncn-program-bls-cli cast-vote`

Cast a vote using BLS multi-signature aggregation

**Usage:** `ncn-program-bls-cli cast-vote [OPTIONS] --aggregated-signature <AGGREGATED_SIGNATURE> --aggregated-g2 <AGGREGATED_G2> --signers-bitmap <SIGNERS_BITMAP>`

###### **Options:**

* `--aggregated-signature <AGGREGATED_SIGNATURE>` — Aggregated G1 signature (64 bytes hex)
* `--aggregated-g2 <AGGREGATED_G2>` — Aggregated G2 public key (128 bytes hex)
* `--signers-bitmap <SIGNERS_BITMAP>` — Bitmap indicating which operators signed (hex string)
* `--message <MESSAGE>` — Message to sign (32 bytes hex, defaults to current vote counter)



## `ncn-program-bls-cli generate-vote-signature`

Generate BLS signature for vote aggregation

**Usage:** `ncn-program-bls-cli generate-vote-signature [OPTIONS] --private-key <PRIVATE_KEY>`

###### **Options:**

* `--private-key <PRIVATE_KEY>` — Operator private key (32 bytes hex)
* `--message <MESSAGE>` — Message to sign (32 bytes hex, defaults to current vote counter)



## `ncn-program-bls-cli aggregate-signatures`

Aggregate multiple BLS signatures for voting

**Usage:** `ncn-program-bls-cli aggregate-signatures --signatures <SIGNATURES> --g1-public-keys <G1_PUBLIC_KEYS> --g2-public-keys <G2_PUBLIC_KEYS> --signers-bitmap <SIGNERS_BITMAP>`

###### **Options:**

* `--signatures <SIGNATURES>` — Comma-separated list of signatures (64 bytes hex each)
* `--g1-public-keys <G1_PUBLIC_KEYS>` — Comma-separated list of G1 public keys (32 bytes hex each)
* `--g2-public-keys <G2_PUBLIC_KEYS>` — Comma-separated list of G2 public keys (64 bytes hex each)
* `--signers-bitmap <SIGNERS_BITMAP>` — Bitmap indicating which operators signed (hex string)



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



## `ncn-program-bls-cli get-all-ncn-operator-accounts`

**Usage:** `ncn-program-bls-cli get-all-ncn-operator-accounts`



## `ncn-program-bls-cli get-ncn-program-config`

**Usage:** `ncn-program-bls-cli get-ncn-program-config`



## `ncn-program-bls-cli get-vault-registry`

**Usage:** `ncn-program-bls-cli get-vault-registry`



## `ncn-program-bls-cli get-vote-counter`

**Usage:** `ncn-program-bls-cli get-vote-counter`



## `ncn-program-bls-cli get-snapshot`

**Usage:** `ncn-program-bls-cli get-snapshot`



## `ncn-program-bls-cli get-operator-snapshot`

**Usage:** `ncn-program-bls-cli get-operator-snapshot --operator <OPERATOR>`

###### **Options:**

* `--operator <OPERATOR>` — Operator Account Address



## `ncn-program-bls-cli get-account-payer`

**Usage:** `ncn-program-bls-cli get-account-payer`



## `ncn-program-bls-cli get-total-epoch-rent-cost`

**Usage:** `ncn-program-bls-cli get-total-epoch-rent-cost`



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

