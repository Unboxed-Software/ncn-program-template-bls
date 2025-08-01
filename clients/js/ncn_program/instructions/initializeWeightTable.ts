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
  getU8Decoder,
  getU8Encoder,
  transformEncoder,
  type Address,
  type Codec,
  type Decoder,
  type Encoder,
  type IAccountMeta,
  type IInstruction,
  type IInstructionWithAccounts,
  type IInstructionWithData,
  type ReadonlyAccount,
  type WritableAccount,
} from '@solana/web3.js';
import { NCN_PROGRAM_PROGRAM_ADDRESS } from '../programs';
import { getAccountMetaFactory, type ResolvedAccount } from '../shared';

export const INITIALIZE_WEIGHT_TABLE_DISCRIMINATOR = 9;

export function getInitializeWeightTableDiscriminatorBytes() {
  return getU8Encoder().encode(INITIALIZE_WEIGHT_TABLE_DISCRIMINATOR);
}

export type InitializeWeightTableInstruction<
  TProgram extends string = typeof NCN_PROGRAM_PROGRAM_ADDRESS,
  TAccountEpochMarker extends string | IAccountMeta<string> = string,
  TAccountEpochState extends string | IAccountMeta<string> = string,
  TAccountVaultRegistry extends string | IAccountMeta<string> = string,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountWeightTable extends string | IAccountMeta<string> = string,
  TAccountAccountPayer extends string | IAccountMeta<string> = string,
  TAccountSystemProgram extends
    | string
    | IAccountMeta<string> = '11111111111111111111111111111111',
  TRemainingAccounts extends readonly IAccountMeta<string>[] = [],
> = IInstruction<TProgram> &
  IInstructionWithData<Uint8Array> &
  IInstructionWithAccounts<
    [
      TAccountEpochMarker extends string
        ? ReadonlyAccount<TAccountEpochMarker>
        : TAccountEpochMarker,
      TAccountEpochState extends string
        ? WritableAccount<TAccountEpochState>
        : TAccountEpochState,
      TAccountVaultRegistry extends string
        ? ReadonlyAccount<TAccountVaultRegistry>
        : TAccountVaultRegistry,
      TAccountNcn extends string ? ReadonlyAccount<TAccountNcn> : TAccountNcn,
      TAccountWeightTable extends string
        ? WritableAccount<TAccountWeightTable>
        : TAccountWeightTable,
      TAccountAccountPayer extends string
        ? WritableAccount<TAccountAccountPayer>
        : TAccountAccountPayer,
      TAccountSystemProgram extends string
        ? ReadonlyAccount<TAccountSystemProgram>
        : TAccountSystemProgram,
      ...TRemainingAccounts,
    ]
  >;

export type InitializeWeightTableInstructionData = {
  discriminator: number;
  epoch: bigint;
};

export type InitializeWeightTableInstructionDataArgs = {
  epoch: number | bigint;
};

export function getInitializeWeightTableInstructionDataEncoder(): Encoder<InitializeWeightTableInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([
      ['discriminator', getU8Encoder()],
      ['epoch', getU64Encoder()],
    ]),
    (value) => ({
      ...value,
      discriminator: INITIALIZE_WEIGHT_TABLE_DISCRIMINATOR,
    })
  );
}

export function getInitializeWeightTableInstructionDataDecoder(): Decoder<InitializeWeightTableInstructionData> {
  return getStructDecoder([
    ['discriminator', getU8Decoder()],
    ['epoch', getU64Decoder()],
  ]);
}

export function getInitializeWeightTableInstructionDataCodec(): Codec<
  InitializeWeightTableInstructionDataArgs,
  InitializeWeightTableInstructionData
> {
  return combineCodec(
    getInitializeWeightTableInstructionDataEncoder(),
    getInitializeWeightTableInstructionDataDecoder()
  );
}

export type InitializeWeightTableInput<
  TAccountEpochMarker extends string = string,
  TAccountEpochState extends string = string,
  TAccountVaultRegistry extends string = string,
  TAccountNcn extends string = string,
  TAccountWeightTable extends string = string,
  TAccountAccountPayer extends string = string,
  TAccountSystemProgram extends string = string,
> = {
  epochMarker: Address<TAccountEpochMarker>;
  epochState: Address<TAccountEpochState>;
  vaultRegistry: Address<TAccountVaultRegistry>;
  ncn: Address<TAccountNcn>;
  weightTable: Address<TAccountWeightTable>;
  accountPayer: Address<TAccountAccountPayer>;
  systemProgram?: Address<TAccountSystemProgram>;
  epoch: InitializeWeightTableInstructionDataArgs['epoch'];
};

export function getInitializeWeightTableInstruction<
  TAccountEpochMarker extends string,
  TAccountEpochState extends string,
  TAccountVaultRegistry extends string,
  TAccountNcn extends string,
  TAccountWeightTable extends string,
  TAccountAccountPayer extends string,
  TAccountSystemProgram extends string,
  TProgramAddress extends Address = typeof NCN_PROGRAM_PROGRAM_ADDRESS,
>(
  input: InitializeWeightTableInput<
    TAccountEpochMarker,
    TAccountEpochState,
    TAccountVaultRegistry,
    TAccountNcn,
    TAccountWeightTable,
    TAccountAccountPayer,
    TAccountSystemProgram
  >,
  config?: { programAddress?: TProgramAddress }
): InitializeWeightTableInstruction<
  TProgramAddress,
  TAccountEpochMarker,
  TAccountEpochState,
  TAccountVaultRegistry,
  TAccountNcn,
  TAccountWeightTable,
  TAccountAccountPayer,
  TAccountSystemProgram
> {
  // Program address.
  const programAddress = config?.programAddress ?? NCN_PROGRAM_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    epochMarker: { value: input.epochMarker ?? null, isWritable: false },
    epochState: { value: input.epochState ?? null, isWritable: true },
    vaultRegistry: { value: input.vaultRegistry ?? null, isWritable: false },
    ncn: { value: input.ncn ?? null, isWritable: false },
    weightTable: { value: input.weightTable ?? null, isWritable: true },
    accountPayer: { value: input.accountPayer ?? null, isWritable: true },
    systemProgram: { value: input.systemProgram ?? null, isWritable: false },
  };
  const accounts = originalAccounts as Record<
    keyof typeof originalAccounts,
    ResolvedAccount
  >;

  // Original args.
  const args = { ...input };

  // Resolve default values.
  if (!accounts.systemProgram.value) {
    accounts.systemProgram.value =
      '11111111111111111111111111111111' as Address<'11111111111111111111111111111111'>;
  }

  const getAccountMeta = getAccountMetaFactory(programAddress, 'programId');
  const instruction = {
    accounts: [
      getAccountMeta(accounts.epochMarker),
      getAccountMeta(accounts.epochState),
      getAccountMeta(accounts.vaultRegistry),
      getAccountMeta(accounts.ncn),
      getAccountMeta(accounts.weightTable),
      getAccountMeta(accounts.accountPayer),
      getAccountMeta(accounts.systemProgram),
    ],
    programAddress,
    data: getInitializeWeightTableInstructionDataEncoder().encode(
      args as InitializeWeightTableInstructionDataArgs
    ),
  } as InitializeWeightTableInstruction<
    TProgramAddress,
    TAccountEpochMarker,
    TAccountEpochState,
    TAccountVaultRegistry,
    TAccountNcn,
    TAccountWeightTable,
    TAccountAccountPayer,
    TAccountSystemProgram
  >;

  return instruction;
}

export type ParsedInitializeWeightTableInstruction<
  TProgram extends string = typeof NCN_PROGRAM_PROGRAM_ADDRESS,
  TAccountMetas extends readonly IAccountMeta[] = readonly IAccountMeta[],
> = {
  programAddress: Address<TProgram>;
  accounts: {
    epochMarker: TAccountMetas[0];
    epochState: TAccountMetas[1];
    vaultRegistry: TAccountMetas[2];
    ncn: TAccountMetas[3];
    weightTable: TAccountMetas[4];
    accountPayer: TAccountMetas[5];
    systemProgram: TAccountMetas[6];
  };
  data: InitializeWeightTableInstructionData;
};

export function parseInitializeWeightTableInstruction<
  TProgram extends string,
  TAccountMetas extends readonly IAccountMeta[],
>(
  instruction: IInstruction<TProgram> &
    IInstructionWithAccounts<TAccountMetas> &
    IInstructionWithData<Uint8Array>
): ParsedInitializeWeightTableInstruction<TProgram, TAccountMetas> {
  if (instruction.accounts.length < 7) {
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
      epochMarker: getNextAccount(),
      epochState: getNextAccount(),
      vaultRegistry: getNextAccount(),
      ncn: getNextAccount(),
      weightTable: getNextAccount(),
      accountPayer: getNextAccount(),
      systemProgram: getNextAccount(),
    },
    data: getInitializeWeightTableInstructionDataDecoder().decode(
      instruction.data
    ),
  };
}
