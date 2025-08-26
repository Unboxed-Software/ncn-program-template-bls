mod admin_initialize_config;
mod admin_register_st_mint;
mod admin_set_new_admin;
mod admin_set_parameters;
mod cast_vote;

mod initialize_snapshot;

mod initialize_vault_registry;
mod initialize_vote_counter;
mod realloc_snapshot;

mod register_operator;
mod register_vault;
mod snapshot_vault_operator_delegation;
mod update_operator_bn128_keys;

use admin_set_new_admin::process_admin_set_new_admin;
use borsh::BorshDeserialize;

use ncn_program_core::instruction::NCNProgramInstruction;
use solana_program::{
    account_info::AccountInfo, declare_id, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey,
};
#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

use crate::{
    admin_initialize_config::process_admin_initialize_config,
    admin_register_st_mint::process_admin_register_st_mint,
    admin_set_parameters::process_admin_set_parameters, cast_vote::process_cast_vote,
    initialize_snapshot::process_initialize_snapshot,
    initialize_vault_registry::process_initialize_vault_registry,
    initialize_vote_counter::process_initialize_vote_counter,
    realloc_snapshot::process_realloc_snapshot, register_operator::process_register_operator,
    register_vault::process_register_vault,
    snapshot_vault_operator_delegation::process_snapshot_vault_operator_delegation,
    update_operator_bn128_keys::process_update_operator_bn128_keys,
};

declare_id!("3fKQSi6VzzDUJSmeksS8qK6RB3Gs3UoZWtsQD3xagy45");

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    // Required fields
    name: "NCN Program Template",
    project_url: "https://jito.network/",
    contacts: "email:team@jito.network",
    policy: "https://github.com/jito-foundation/ncn-program",
    // Optional Fields
    preferred_languages: "en",
    source_code: "https://github.com/jito-foundation/ncn-program"
}

#[cfg(not(feature = "no-entrypoint"))]
solana_program::entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if *program_id != id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    let instruction = NCNProgramInstruction::try_from_slice(instruction_data)?;

    match instruction {
        // ---------------------------------------------------- //
        //                         GLOBAL                       //
        // ---------------------------------------------------- //
        NCNProgramInstruction::InitializeConfig {
            epochs_before_stall,
            epochs_after_consensus_before_close,
            valid_slots_after_consensus,
            minimum_stake,
            ncn_fee_bps,
        } => {
            msg!("Instruction: InitializeConfig");
            process_admin_initialize_config(
                program_id,
                accounts,
                epochs_before_stall,
                epochs_after_consensus_before_close,
                valid_slots_after_consensus,
                minimum_stake,
                ncn_fee_bps,
            )
        }
        NCNProgramInstruction::InitializeVaultRegistry => {
            msg!("Instruction: InitializeVaultRegistry");
            process_initialize_vault_registry(program_id, accounts)
        }
        NCNProgramInstruction::RegisterVault => {
            msg!("Instruction: RegisterVault");
            process_register_vault(program_id, accounts)
        }

        NCNProgramInstruction::RegisterOperator {
            g1_pubkey,
            g2_pubkey,
            signature,
        } => {
            msg!("Instruction: RegisterOperator");
            process_register_operator(program_id, accounts, g1_pubkey, g2_pubkey, signature)
        }
        NCNProgramInstruction::UpdateOperatorBN128Keys {
            g1_pubkey,
            g2_pubkey,
            signature,
        } => {
            msg!("Instruction: UpdateOperatorBN128Keys");
            process_update_operator_bn128_keys(
                program_id, accounts, g1_pubkey, g2_pubkey, signature,
            )
        }

        NCNProgramInstruction::InitializeVoteCounter => {
            msg!("Instruction: InitializeVoteCounter");
            process_initialize_vote_counter(program_id, accounts)
        }

        // ---------------------------------------------------- //
        //                       SNAPSHOT                       //
        // ---------------------------------------------------- //
        NCNProgramInstruction::InitializeSnapshot {} => {
            msg!("Instruction: InitializeSnapshot");
            process_initialize_snapshot(program_id, accounts)
        }
        NCNProgramInstruction::ReallocSnapshot {} => {
            msg!("Instruction: ReallocSnapshot");
            process_realloc_snapshot(program_id, accounts)
        }
        NCNProgramInstruction::SnapshotVaultOperatorDelegation {} => {
            msg!("Instruction: SnapshotVaultOperatorDelegation");
            process_snapshot_vault_operator_delegation(program_id, accounts)
        }

        // ---------------------------------------------------- //
        //                         VOTE                         //
        // ---------------------------------------------------- //
        NCNProgramInstruction::CastVote {
            aggregated_g2,
            aggregated_signature,
            operators_signature_bitmap,
        } => {
            msg!("Instruction: CastVote");
            process_cast_vote(
                program_id,
                accounts,
                aggregated_g2,
                aggregated_signature,
                operators_signature_bitmap,
            )
        }

        // ---------------------------------------------------- //
        //                        ADMIN                         //
        // ---------------------------------------------------- //
        NCNProgramInstruction::AdminSetParameters {
            starting_valid_epoch,
            epochs_before_stall,
            epochs_after_consensus_before_close,
            valid_slots_after_consensus,
            minimum_stake,
        } => {
            msg!("Instruction: AdminSetParameters");
            process_admin_set_parameters(
                program_id,
                accounts,
                starting_valid_epoch,
                epochs_before_stall,
                epochs_after_consensus_before_close,
                minimum_stake,
                valid_slots_after_consensus,
            )
        }
        NCNProgramInstruction::AdminSetNewAdmin { role } => {
            msg!("Instruction: AdminSetNewAdmin");
            process_admin_set_new_admin(program_id, accounts, role)
        }
        NCNProgramInstruction::AdminRegisterStMint {} => {
            msg!("Instruction: AdminRegisterStMint");
            process_admin_register_st_mint(program_id, accounts)
        }
    }
}
