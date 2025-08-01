/**
 * This code was AUTOGENERATED using the kinobi library.
 * Please DO NOT EDIT THIS FILE, instead use visitors
 * to add features, then rerun kinobi to update it.
 *
 * @see https://github.com/kinobi-so/kinobi
 */

import {
  combineCodec,
  getStructDecoder,
  getStructEncoder,
  getU64Decoder,
  getU64Encoder,
  type Codec,
  type Decoder,
  type Encoder,
} from '@solana/web3.js';
import { getFeeDecoder, getFeeEncoder, type Fee, type FeeArgs } from '.';

export type Fees = {
  activationEpoch: bigint;
  protocolFeeBps: Fee;
  ncnFeeBps: Fee;
};

export type FeesArgs = {
  activationEpoch: number | bigint;
  protocolFeeBps: FeeArgs;
  ncnFeeBps: FeeArgs;
};

export function getFeesEncoder(): Encoder<FeesArgs> {
  return getStructEncoder([
    ['activationEpoch', getU64Encoder()],
    ['protocolFeeBps', getFeeEncoder()],
    ['ncnFeeBps', getFeeEncoder()],
  ]);
}

export function getFeesDecoder(): Decoder<Fees> {
  return getStructDecoder([
    ['activationEpoch', getU64Decoder()],
    ['protocolFeeBps', getFeeDecoder()],
    ['ncnFeeBps', getFeeDecoder()],
  ]);
}

export function getFeesCodec(): Codec<FeesArgs, Fees> {
  return combineCodec(getFeesEncoder(), getFeesDecoder());
}
