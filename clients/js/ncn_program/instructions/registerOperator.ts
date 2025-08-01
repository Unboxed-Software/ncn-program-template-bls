/**
 * This code was AUTOGENERATED using the kinobi library.
 * Please DO NOT EDIT THIS FILE, instead use visitors
 * to add features, then rerun kinobi to update it.
 *
 * @see https://github.com/kinobi-so/kinobi
 */

import {
  combineCodec,
  fixDecoderSize,
  fixEncoderSize,
  getBytesDecoder,
  getBytesEncoder,
  getStructDecoder,
  getStructEncoder,
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
  type ReadonlyAccount,
  type ReadonlySignerAccount,
  type ReadonlyUint8Array,
  type TransactionSigner,
  type WritableAccount,
} from '@solana/web3.js';
import { NCN_PROGRAM_PROGRAM_ADDRESS } from '../programs';
import { getAccountMetaFactory, type ResolvedAccount } from '../shared';

export const REGISTER_OPERATOR_DISCRIMINATOR = 5;

export function getRegisterOperatorDiscriminatorBytes() {
  return getU8Encoder().encode(REGISTER_OPERATOR_DISCRIMINATOR);
}

export type RegisterOperatorInstruction<
  TProgram extends string = typeof NCN_PROGRAM_PROGRAM_ADDRESS,
  TAccountConfig extends string | IAccountMeta<string> = string,
  TAccountOperatorRegistry extends string | IAccountMeta<string> = string,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountOperator extends string | IAccountMeta<string> = string,
  TAccountOperatorAdmin extends string | IAccountMeta<string> = string,
  TAccountNcnOperatorState extends string | IAccountMeta<string> = string,
  TAccountRestakingConfig extends string | IAccountMeta<string> = string,
  TRemainingAccounts extends readonly IAccountMeta<string>[] = [],
> = IInstruction<TProgram> &
  IInstructionWithData<Uint8Array> &
  IInstructionWithAccounts<
    [
      TAccountConfig extends string
        ? ReadonlyAccount<TAccountConfig>
        : TAccountConfig,
      TAccountOperatorRegistry extends string
        ? WritableAccount<TAccountOperatorRegistry>
        : TAccountOperatorRegistry,
      TAccountNcn extends string ? ReadonlyAccount<TAccountNcn> : TAccountNcn,
      TAccountOperator extends string
        ? ReadonlyAccount<TAccountOperator>
        : TAccountOperator,
      TAccountOperatorAdmin extends string
        ? ReadonlySignerAccount<TAccountOperatorAdmin> &
            IAccountSignerMeta<TAccountOperatorAdmin>
        : TAccountOperatorAdmin,
      TAccountNcnOperatorState extends string
        ? ReadonlyAccount<TAccountNcnOperatorState>
        : TAccountNcnOperatorState,
      TAccountRestakingConfig extends string
        ? ReadonlyAccount<TAccountRestakingConfig>
        : TAccountRestakingConfig,
      ...TRemainingAccounts,
    ]
  >;

export type RegisterOperatorInstructionData = {
  discriminator: number;
  g1Pubkey: ReadonlyUint8Array;
  g2Pubkey: ReadonlyUint8Array;
  signature: ReadonlyUint8Array;
};

export type RegisterOperatorInstructionDataArgs = {
  g1Pubkey: ReadonlyUint8Array;
  g2Pubkey: ReadonlyUint8Array;
  signature: ReadonlyUint8Array;
};

export function getRegisterOperatorInstructionDataEncoder(): Encoder<RegisterOperatorInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([
      ['discriminator', getU8Encoder()],
      ['g1Pubkey', fixEncoderSize(getBytesEncoder(), 32)],
      ['g2Pubkey', fixEncoderSize(getBytesEncoder(), 64)],
      ['signature', fixEncoderSize(getBytesEncoder(), 64)],
    ]),
    (value) => ({ ...value, discriminator: REGISTER_OPERATOR_DISCRIMINATOR })
  );
}

export function getRegisterOperatorInstructionDataDecoder(): Decoder<RegisterOperatorInstructionData> {
  return getStructDecoder([
    ['discriminator', getU8Decoder()],
    ['g1Pubkey', fixDecoderSize(getBytesDecoder(), 32)],
    ['g2Pubkey', fixDecoderSize(getBytesDecoder(), 64)],
    ['signature', fixDecoderSize(getBytesDecoder(), 64)],
  ]);
}

export function getRegisterOperatorInstructionDataCodec(): Codec<
  RegisterOperatorInstructionDataArgs,
  RegisterOperatorInstructionData
> {
  return combineCodec(
    getRegisterOperatorInstructionDataEncoder(),
    getRegisterOperatorInstructionDataDecoder()
  );
}

export type RegisterOperatorInput<
  TAccountConfig extends string = string,
  TAccountOperatorRegistry extends string = string,
  TAccountNcn extends string = string,
  TAccountOperator extends string = string,
  TAccountOperatorAdmin extends string = string,
  TAccountNcnOperatorState extends string = string,
  TAccountRestakingConfig extends string = string,
> = {
  config: Address<TAccountConfig>;
  operatorRegistry: Address<TAccountOperatorRegistry>;
  ncn: Address<TAccountNcn>;
  operator: Address<TAccountOperator>;
  operatorAdmin: TransactionSigner<TAccountOperatorAdmin>;
  ncnOperatorState: Address<TAccountNcnOperatorState>;
  restakingConfig: Address<TAccountRestakingConfig>;
  g1Pubkey: RegisterOperatorInstructionDataArgs['g1Pubkey'];
  g2Pubkey: RegisterOperatorInstructionDataArgs['g2Pubkey'];
  signature: RegisterOperatorInstructionDataArgs['signature'];
};

export function getRegisterOperatorInstruction<
  TAccountConfig extends string,
  TAccountOperatorRegistry extends string,
  TAccountNcn extends string,
  TAccountOperator extends string,
  TAccountOperatorAdmin extends string,
  TAccountNcnOperatorState extends string,
  TAccountRestakingConfig extends string,
  TProgramAddress extends Address = typeof NCN_PROGRAM_PROGRAM_ADDRESS,
>(
  input: RegisterOperatorInput<
    TAccountConfig,
    TAccountOperatorRegistry,
    TAccountNcn,
    TAccountOperator,
    TAccountOperatorAdmin,
    TAccountNcnOperatorState,
    TAccountRestakingConfig
  >,
  config?: { programAddress?: TProgramAddress }
): RegisterOperatorInstruction<
  TProgramAddress,
  TAccountConfig,
  TAccountOperatorRegistry,
  TAccountNcn,
  TAccountOperator,
  TAccountOperatorAdmin,
  TAccountNcnOperatorState,
  TAccountRestakingConfig
> {
  // Program address.
  const programAddress = config?.programAddress ?? NCN_PROGRAM_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    config: { value: input.config ?? null, isWritable: false },
    operatorRegistry: {
      value: input.operatorRegistry ?? null,
      isWritable: true,
    },
    ncn: { value: input.ncn ?? null, isWritable: false },
    operator: { value: input.operator ?? null, isWritable: false },
    operatorAdmin: { value: input.operatorAdmin ?? null, isWritable: false },
    ncnOperatorState: {
      value: input.ncnOperatorState ?? null,
      isWritable: false,
    },
    restakingConfig: {
      value: input.restakingConfig ?? null,
      isWritable: false,
    },
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
      getAccountMeta(accounts.operatorRegistry),
      getAccountMeta(accounts.ncn),
      getAccountMeta(accounts.operator),
      getAccountMeta(accounts.operatorAdmin),
      getAccountMeta(accounts.ncnOperatorState),
      getAccountMeta(accounts.restakingConfig),
    ],
    programAddress,
    data: getRegisterOperatorInstructionDataEncoder().encode(
      args as RegisterOperatorInstructionDataArgs
    ),
  } as RegisterOperatorInstruction<
    TProgramAddress,
    TAccountConfig,
    TAccountOperatorRegistry,
    TAccountNcn,
    TAccountOperator,
    TAccountOperatorAdmin,
    TAccountNcnOperatorState,
    TAccountRestakingConfig
  >;

  return instruction;
}

export type ParsedRegisterOperatorInstruction<
  TProgram extends string = typeof NCN_PROGRAM_PROGRAM_ADDRESS,
  TAccountMetas extends readonly IAccountMeta[] = readonly IAccountMeta[],
> = {
  programAddress: Address<TProgram>;
  accounts: {
    config: TAccountMetas[0];
    operatorRegistry: TAccountMetas[1];
    ncn: TAccountMetas[2];
    operator: TAccountMetas[3];
    operatorAdmin: TAccountMetas[4];
    ncnOperatorState: TAccountMetas[5];
    restakingConfig: TAccountMetas[6];
  };
  data: RegisterOperatorInstructionData;
};

export function parseRegisterOperatorInstruction<
  TProgram extends string,
  TAccountMetas extends readonly IAccountMeta[],
>(
  instruction: IInstruction<TProgram> &
    IInstructionWithAccounts<TAccountMetas> &
    IInstructionWithData<Uint8Array>
): ParsedRegisterOperatorInstruction<TProgram, TAccountMetas> {
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
      config: getNextAccount(),
      operatorRegistry: getNextAccount(),
      ncn: getNextAccount(),
      operator: getNextAccount(),
      operatorAdmin: getNextAccount(),
      ncnOperatorState: getNextAccount(),
      restakingConfig: getNextAccount(),
    },
    data: getRegisterOperatorInstructionDataDecoder().decode(instruction.data),
  };
}
