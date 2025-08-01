/**
 * This code was AUTOGENERATED using the kinobi library.
 * Please DO NOT EDIT THIS FILE, instead use visitors
 * to add features, then rerun kinobi to update it.
 *
 * @see https://github.com/kinobi-so/kinobi
 */

import {
  combineCodec,
  getOptionDecoder,
  getOptionEncoder,
  getStructDecoder,
  getStructEncoder,
  getU128Decoder,
  getU128Encoder,
  getU64Decoder,
  getU64Encoder,
  getU8Decoder,
  getU8Encoder,
  transformEncoder,
  type Address,
  type Codec,
  type Decoder,
  type Encoder,
  type IAccountMeta,
  type IAccountSignerMeta,
  type IInstruction,
  type IInstructionWithAccounts,
  type IInstructionWithData,
  type Option,
  type OptionOrNullable,
  type ReadonlyAccount,
  type ReadonlySignerAccount,
  type TransactionSigner,
  type WritableAccount,
} from '@solana/web3.js';
import { NCN_PROGRAM_PROGRAM_ADDRESS } from '../programs';
import { getAccountMetaFactory, type ResolvedAccount } from '../shared';

export const ADMIN_SET_PARAMETERS_DISCRIMINATOR = 18;

export function getAdminSetParametersDiscriminatorBytes() {
  return getU8Encoder().encode(ADMIN_SET_PARAMETERS_DISCRIMINATOR);
}

export type AdminSetParametersInstruction<
  TProgram extends string = typeof NCN_PROGRAM_PROGRAM_ADDRESS,
  TAccountConfig extends string | IAccountMeta<string> = string,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountNcnAdmin extends string | IAccountMeta<string> = string,
  TRemainingAccounts extends readonly IAccountMeta<string>[] = [],
> = IInstruction<TProgram> &
  IInstructionWithData<Uint8Array> &
  IInstructionWithAccounts<
    [
      TAccountConfig extends string
        ? WritableAccount<TAccountConfig>
        : TAccountConfig,
      TAccountNcn extends string ? ReadonlyAccount<TAccountNcn> : TAccountNcn,
      TAccountNcnAdmin extends string
        ? ReadonlySignerAccount<TAccountNcnAdmin> &
            IAccountSignerMeta<TAccountNcnAdmin>
        : TAccountNcnAdmin,
      ...TRemainingAccounts,
    ]
  >;

export type AdminSetParametersInstructionData = {
  discriminator: number;
  startingValidEpoch: Option<bigint>;
  epochsBeforeStall: Option<bigint>;
  epochsAfterConsensusBeforeClose: Option<bigint>;
  validSlotsAfterConsensus: Option<bigint>;
  minimumStakeWeight: Option<bigint>;
};

export type AdminSetParametersInstructionDataArgs = {
  startingValidEpoch: OptionOrNullable<number | bigint>;
  epochsBeforeStall: OptionOrNullable<number | bigint>;
  epochsAfterConsensusBeforeClose: OptionOrNullable<number | bigint>;
  validSlotsAfterConsensus: OptionOrNullable<number | bigint>;
  minimumStakeWeight: OptionOrNullable<number | bigint>;
};

export function getAdminSetParametersInstructionDataEncoder(): Encoder<AdminSetParametersInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([
      ['discriminator', getU8Encoder()],
      ['startingValidEpoch', getOptionEncoder(getU64Encoder())],
      ['epochsBeforeStall', getOptionEncoder(getU64Encoder())],
      ['epochsAfterConsensusBeforeClose', getOptionEncoder(getU64Encoder())],
      ['validSlotsAfterConsensus', getOptionEncoder(getU64Encoder())],
      ['minimumStakeWeight', getOptionEncoder(getU128Encoder())],
    ]),
    (value) => ({ ...value, discriminator: ADMIN_SET_PARAMETERS_DISCRIMINATOR })
  );
}

export function getAdminSetParametersInstructionDataDecoder(): Decoder<AdminSetParametersInstructionData> {
  return getStructDecoder([
    ['discriminator', getU8Decoder()],
    ['startingValidEpoch', getOptionDecoder(getU64Decoder())],
    ['epochsBeforeStall', getOptionDecoder(getU64Decoder())],
    ['epochsAfterConsensusBeforeClose', getOptionDecoder(getU64Decoder())],
    ['validSlotsAfterConsensus', getOptionDecoder(getU64Decoder())],
    ['minimumStakeWeight', getOptionDecoder(getU128Decoder())],
  ]);
}

export function getAdminSetParametersInstructionDataCodec(): Codec<
  AdminSetParametersInstructionDataArgs,
  AdminSetParametersInstructionData
> {
  return combineCodec(
    getAdminSetParametersInstructionDataEncoder(),
    getAdminSetParametersInstructionDataDecoder()
  );
}

export type AdminSetParametersInput<
  TAccountConfig extends string = string,
  TAccountNcn extends string = string,
  TAccountNcnAdmin extends string = string,
> = {
  config: Address<TAccountConfig>;
  ncn: Address<TAccountNcn>;
  ncnAdmin: TransactionSigner<TAccountNcnAdmin>;
  startingValidEpoch: AdminSetParametersInstructionDataArgs['startingValidEpoch'];
  epochsBeforeStall: AdminSetParametersInstructionDataArgs['epochsBeforeStall'];
  epochsAfterConsensusBeforeClose: AdminSetParametersInstructionDataArgs['epochsAfterConsensusBeforeClose'];
  validSlotsAfterConsensus: AdminSetParametersInstructionDataArgs['validSlotsAfterConsensus'];
  minimumStakeWeight: AdminSetParametersInstructionDataArgs['minimumStakeWeight'];
};

export function getAdminSetParametersInstruction<
  TAccountConfig extends string,
  TAccountNcn extends string,
  TAccountNcnAdmin extends string,
  TProgramAddress extends Address = typeof NCN_PROGRAM_PROGRAM_ADDRESS,
>(
  input: AdminSetParametersInput<TAccountConfig, TAccountNcn, TAccountNcnAdmin>,
  config?: { programAddress?: TProgramAddress }
): AdminSetParametersInstruction<
  TProgramAddress,
  TAccountConfig,
  TAccountNcn,
  TAccountNcnAdmin
> {
  // Program address.
  const programAddress = config?.programAddress ?? NCN_PROGRAM_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    config: { value: input.config ?? null, isWritable: true },
    ncn: { value: input.ncn ?? null, isWritable: false },
    ncnAdmin: { value: input.ncnAdmin ?? null, isWritable: false },
  };
  const accounts = originalAccounts as Record<
    keyof typeof originalAccounts,
    ResolvedAccount
  >;

  // Original args.
  const args = { ...input };

  const getAccountMeta = getAccountMetaFactory(programAddress, 'programId');
  const instruction = {
    accounts: [
      getAccountMeta(accounts.config),
      getAccountMeta(accounts.ncn),
      getAccountMeta(accounts.ncnAdmin),
    ],
    programAddress,
    data: getAdminSetParametersInstructionDataEncoder().encode(
      args as AdminSetParametersInstructionDataArgs
    ),
  } as AdminSetParametersInstruction<
    TProgramAddress,
    TAccountConfig,
    TAccountNcn,
    TAccountNcnAdmin
  >;

  return instruction;
}

export type ParsedAdminSetParametersInstruction<
  TProgram extends string = typeof NCN_PROGRAM_PROGRAM_ADDRESS,
  TAccountMetas extends readonly IAccountMeta[] = readonly IAccountMeta[],
> = {
  programAddress: Address<TProgram>;
  accounts: {
    config: TAccountMetas[0];
    ncn: TAccountMetas[1];
    ncnAdmin: TAccountMetas[2];
  };
  data: AdminSetParametersInstructionData;
};

export function parseAdminSetParametersInstruction<
  TProgram extends string,
  TAccountMetas extends readonly IAccountMeta[],
>(
  instruction: IInstruction<TProgram> &
    IInstructionWithAccounts<TAccountMetas> &
    IInstructionWithData<Uint8Array>
): ParsedAdminSetParametersInstruction<TProgram, TAccountMetas> {
  if (instruction.accounts.length < 3) {
    // TODO: Coded error.
    throw new Error('Not enough accounts');
  }
  let accountIndex = 0;
  const getNextAccount = () => {
    const accountMeta = instruction.accounts![accountIndex]!;
    accountIndex += 1;
    return accountMeta;
  };
  return {
    programAddress: instruction.programAddress,
    accounts: {
      config: getNextAccount(),
      ncn: getNextAccount(),
      ncnAdmin: getNextAccount(),
    },
    data: getAdminSetParametersInstructionDataDecoder().decode(
      instruction.data
    ),
  };
}
