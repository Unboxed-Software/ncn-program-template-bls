/**
 * This code was AUTOGENERATED using the kinobi library.
 * Please DO NOT EDIT THIS FILE, instead use visitors
 * to add features, then rerun kinobi to update it.
 *
 * @see https://github.com/kinobi-so/kinobi
 */

import {
  assertAccountExists,
  assertAccountsExist,
  combineCodec,
  decodeAccount,
  fetchEncodedAccount,
  fetchEncodedAccounts,
  getAddressDecoder,
  getAddressEncoder,
  getArrayDecoder,
  getArrayEncoder,
  getStructDecoder,
  getStructEncoder,
  getU64Decoder,
  getU64Encoder,
  getU8Decoder,
  getU8Encoder,
  type Account,
  type Address,
  type Codec,
  type Decoder,
  type EncodedAccount,
  type Encoder,
  type FetchAccountConfig,
  type FetchAccountsConfig,
  type MaybeAccount,
  type MaybeEncodedAccount,
} from '@solana/web3.js';
import {
  getOperatorEntryDecoder,
  getOperatorEntryEncoder,
  type OperatorEntry,
  type OperatorEntryArgs,
} from '../types';

export type OperatorRegistry = {
  discriminator: bigint;
  ncn: Address;
  bump: number;
  operatorList: Array<OperatorEntry>;
};

export type OperatorRegistryArgs = {
  discriminator: number | bigint;
  ncn: Address;
  bump: number;
  operatorList: Array<OperatorEntryArgs>;
};

export function getOperatorRegistryEncoder(): Encoder<OperatorRegistryArgs> {
  return getStructEncoder([
    ['discriminator', getU64Encoder()],
    ['ncn', getAddressEncoder()],
    ['bump', getU8Encoder()],
    ['operatorList', getArrayEncoder(getOperatorEntryEncoder(), { size: 256 })],
  ]);
}

export function getOperatorRegistryDecoder(): Decoder<OperatorRegistry> {
  return getStructDecoder([
    ['discriminator', getU64Decoder()],
    ['ncn', getAddressDecoder()],
    ['bump', getU8Decoder()],
    ['operatorList', getArrayDecoder(getOperatorEntryDecoder(), { size: 256 })],
  ]);
}

export function getOperatorRegistryCodec(): Codec<
  OperatorRegistryArgs,
  OperatorRegistry
> {
  return combineCodec(
    getOperatorRegistryEncoder(),
    getOperatorRegistryDecoder()
  );
}

export function decodeOperatorRegistry<TAddress extends string = string>(
  encodedAccount: EncodedAccount<TAddress>
): Account<OperatorRegistry, TAddress>;
export function decodeOperatorRegistry<TAddress extends string = string>(
  encodedAccount: MaybeEncodedAccount<TAddress>
): MaybeAccount<OperatorRegistry, TAddress>;
export function decodeOperatorRegistry<TAddress extends string = string>(
  encodedAccount: EncodedAccount<TAddress> | MaybeEncodedAccount<TAddress>
):
  | Account<OperatorRegistry, TAddress>
  | MaybeAccount<OperatorRegistry, TAddress> {
  return decodeAccount(
    encodedAccount as MaybeEncodedAccount<TAddress>,
    getOperatorRegistryDecoder()
  );
}

export async function fetchOperatorRegistry<TAddress extends string = string>(
  rpc: Parameters<typeof fetchEncodedAccount>[0],
  address: Address<TAddress>,
  config?: FetchAccountConfig
): Promise<Account<OperatorRegistry, TAddress>> {
  const maybeAccount = await fetchMaybeOperatorRegistry(rpc, address, config);
  assertAccountExists(maybeAccount);
  return maybeAccount;
}

export async function fetchMaybeOperatorRegistry<
  TAddress extends string = string,
>(
  rpc: Parameters<typeof fetchEncodedAccount>[0],
  address: Address<TAddress>,
  config?: FetchAccountConfig
): Promise<MaybeAccount<OperatorRegistry, TAddress>> {
  const maybeAccount = await fetchEncodedAccount(rpc, address, config);
  return decodeOperatorRegistry(maybeAccount);
}

export async function fetchAllOperatorRegistry(
  rpc: Parameters<typeof fetchEncodedAccounts>[0],
  addresses: Array<Address>,
  config?: FetchAccountsConfig
): Promise<Account<OperatorRegistry>[]> {
  const maybeAccounts = await fetchAllMaybeOperatorRegistry(
    rpc,
    addresses,
    config
  );
  assertAccountsExist(maybeAccounts);
  return maybeAccounts;
}

export async function fetchAllMaybeOperatorRegistry(
  rpc: Parameters<typeof fetchEncodedAccounts>[0],
  addresses: Array<Address>,
  config?: FetchAccountsConfig
): Promise<MaybeAccount<OperatorRegistry>[]> {
  const maybeAccounts = await fetchEncodedAccounts(rpc, addresses, config);
  return maybeAccounts.map((maybeAccount) =>
    decodeOperatorRegistry(maybeAccount)
  );
}
