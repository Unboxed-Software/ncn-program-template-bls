use std::time::Duration;

use crate::{
    getters::{
        get_account, get_all_operators_in_ncn, get_all_sorted_operators_for_vault, get_all_vaults,
        get_all_vaults_in_ncn, get_ballot_box, get_consensus_result, get_current_slot,
        get_epoch_snapshot, get_ncn_program_config, get_ncn_reward_receiver_rewards,
        get_ncn_reward_router, get_operator, get_operator_snapshot,
        get_operator_vault_reward_receiver_rewards, get_operator_vault_reward_router,
        get_or_create_vault_registry, get_vault, get_vault_config, get_vault_registry,
        get_vault_update_state_tracker, get_weight_table,
    },
    handler::CliHandler,
    log::boring_progress_bar,
};
use anyhow::{anyhow, Ok, Result};
use jito_restaking_core::{
    config::Config as RestakingConfig, ncn_operator_state::NcnOperatorState,
    ncn_vault_ticket::NcnVaultTicket,
};
use jito_vault_client::{
    instructions::{
        CloseVaultUpdateStateTrackerBuilder, CrankVaultUpdateStateTrackerBuilder,
        InitializeVaultUpdateStateTrackerBuilder,
    },
    types::WithdrawalAllocationMethod,
};
use jito_vault_core::{
    config::Config as VaultConfig, vault_ncn_ticket::VaultNcnTicket,
    vault_operator_delegation::VaultOperatorDelegation,
    vault_update_state_tracker::VaultUpdateStateTracker,
};
use log::info;
use ncn_program_client::{
    instructions::{
        AdminRegisterStMintBuilder, AdminSetNewAdminBuilder, AdminSetParametersBuilder,
        AdminSetTieBreakerBuilder, AdminSetWeightBuilder, CastVoteBuilder,
        CloseEpochAccountBuilder, DistributeNCNRewardsBuilder, DistributeOperatorRewardsBuilder,
        DistributeOperatorVaultRewardRouteBuilder, DistributeProtocolRewardsBuilder,
        DistributeVaultRewardsBuilder, InitializeBallotBoxBuilder,
        InitializeConfigBuilder as InitializeNCNProgramConfigBuilder,
        InitializeEpochSnapshotBuilder, InitializeEpochStateBuilder,
        InitializeNCNRewardRouterBuilder, InitializeOperatorSnapshotBuilder,
        InitializeOperatorVaultRewardRouterBuilder, InitializeVaultRegistryBuilder,
        InitializeWeightTableBuilder, ReallocBallotBoxBuilder, ReallocNCNRewardRouterBuilder,
        ReallocVaultRegistryBuilder, ReallocWeightTableBuilder, RegisterVaultBuilder,
        RouteNCNRewardsBuilder, RouteOperatorVaultRewardsBuilder, SetEpochWeightsBuilder,
        SnapshotVaultOperatorDelegationBuilder,
    },
    types::ConfigAdminRole,
};
use ncn_program_core::{
    account_payer::AccountPayer,
    ballot_box::{BallotBox, WeatherStatus},
    config::Config as NCNProgramConfig,
    consensus_result::ConsensusResult,
    constants::MAX_REALLOC_BYTES,
    epoch_marker::EpochMarker,
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    epoch_state::EpochState,
    ncn_reward_router::{NCNRewardReceiver, NCNRewardRouter},
    operator_vault_reward_router::{OperatorVaultRewardReceiver, OperatorVaultRewardRouter},
    vault_registry::VaultRegistry,
    weight_table::WeightTable,
};
use solana_client::rpc_config::RpcSendTransactionConfig;

use serde::Deserialize;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    instruction::Instruction,
    native_token::sol_to_lamports,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    system_instruction::transfer,
    system_program,
    transaction::Transaction,
};
use tokio::time::sleep;

// --------------------- ADMIN ------------------------------
#[allow(clippy::too_many_arguments)]
pub async fn admin_create_config(
    handler: &CliHandler,
    ncn_fee_wallet: Pubkey,
    ncn_fee_bps: u16,
    tie_breaker_admin: Option<Pubkey>,
    epochs_before_stall: u64,
    valid_slots_after_consensus: u64,
    epochs_after_consensus_before_close: u64,
) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;

    let (config, _, _) = NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn);

    let (account_payer, _, _) = AccountPayer::find_program_address(&handler.ncn_program_id, &ncn);
    println!("Account Payer: {}", account_payer.to_string());

    let tie_breaker_admin = tie_breaker_admin.unwrap_or_else(|| keypair.pubkey());

    let initialize_config_ix = InitializeNCNProgramConfigBuilder::new()
        .config(config)
        .ncn_admin(keypair.pubkey())
        .ncn(ncn)
        .account_payer(account_payer)
        .ncn_fee_wallet(ncn_fee_wallet)
        .ncn_fee_bps(ncn_fee_bps)
        .epochs_before_stall(epochs_before_stall)
        .valid_slots_after_consensus(valid_slots_after_consensus)
        .epochs_after_consensus_before_close(epochs_after_consensus_before_close)
        .tie_breaker_admin(tie_breaker_admin)
        .ncn_admin(keypair.pubkey())
        .instruction();

    let program = client.get_account(&handler.ncn_program_id).await?;

    info!(
        "\n\n----------------------\nProgram: {:?}\n\nProgram Account:\n{:?}\n\nIX:\n{:?}\n----------------------\n",
        &handler.ncn_program_id, program, &initialize_config_ix
    );

    send_and_log_transaction(
        handler,
        &[initialize_config_ix],
        &[],
        "Created NCN Program Config",
        &[
            format!("NCN: {:?}", ncn),
            format!("Ncn Admin: {:?}", keypair.pubkey()),
            format!("Tie Breaker Admin: {:?}", tie_breaker_admin),
            format!(
                "Valid Slots After Consensus: {:?}",
                valid_slots_after_consensus
            ),
        ],
    )
    .await?;

    Ok(())
}

pub async fn admin_register_st_mint(
    handler: &CliHandler,
    vault: &Pubkey,
    weight: Option<u128>,
) -> Result<()> {
    let keypair = handler.keypair()?;

    let ncn = *handler.ncn()?;

    let (config, _, _) = NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn);

    let (vault_registry, _, _) = VaultRegistry::find_program_address(&handler.ncn_program_id, &ncn);

    let vault_account = get_vault(handler, vault).await?;

    let mut register_st_mint_builder = AdminRegisterStMintBuilder::new();

    register_st_mint_builder
        .config(config)
        .admin(keypair.pubkey())
        .vault_registry(vault_registry)
        .ncn(ncn)
        .st_mint(vault_account.supported_mint);

    if let Some(weight) = weight {
        register_st_mint_builder.weight(weight);
    }

    let register_st_mint_ix = register_st_mint_builder.instruction();

    send_and_log_transaction(
        handler,
        &[register_st_mint_ix],
        &[],
        "Registered ST Mint",
        &[
            format!("NCN: {:?}", ncn),
            format!("ST Mint: {:?}", vault_account.supported_mint),
            format!("Weight: {:?}", weight.unwrap_or_default()),
        ],
    )
    .await?;

    Ok(())
}

pub async fn admin_set_weight(
    handler: &CliHandler,
    vault: &Pubkey,
    epoch: u64,
    weight: u128,
) -> Result<()> {
    let vault_account = get_vault(handler, vault).await?;

    admin_set_weight_with_st_mint(handler, &vault_account.supported_mint, epoch, weight).await
}

pub async fn admin_set_weight_with_st_mint(
    handler: &CliHandler,
    st_mint: &Pubkey,
    epoch: u64,
    weight: u128,
) -> Result<()> {
    let keypair = handler.keypair()?;

    let ncn = *handler.ncn()?;

    let (weight_table, _, _) =
        WeightTable::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let admin_set_weight_ix = AdminSetWeightBuilder::new()
        .ncn(ncn)
        .weight_table(weight_table)
        .epoch_state(epoch_state)
        .weight_table_admin(keypair.pubkey())
        .st_mint(*st_mint)
        .weight(weight)
        .epoch(epoch)
        .instruction();

    send_and_log_transaction(
        handler,
        &[admin_set_weight_ix],
        &[],
        "Set Weight",
        &[
            format!("NCN: {:?}", ncn),
            format!("Epoch: {:?}", epoch),
            format!("ST Mint: {:?}", st_mint),
            format!("Weight: {:?}", weight),
        ],
    )
    .await?;

    Ok(())
}

pub async fn admin_set_tie_breaker(
    handler: &CliHandler,
    epoch: u64,
    weather_status: u8,
) -> Result<()> {
    let keypair = handler.keypair()?;

    let ncn = *handler.ncn()?;

    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (ncn_config, _, _) = NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn);

    let (ballot_box, _, _) = BallotBox::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let set_tie_breaker_ix = AdminSetTieBreakerBuilder::new()
        .epoch_state(epoch_state)
        .config(ncn_config)
        .ballot_box(ballot_box)
        .ncn(ncn)
        .tie_breaker_admin(keypair.pubkey())
        .weather_status(weather_status)
        .epoch(epoch)
        .instruction();

    send_and_log_transaction(
        handler,
        &[set_tie_breaker_ix],
        &[],
        "Set Tie Breaker",
        &[
            format!("NCN: {:?}", ncn),
            format!("weather_status: {:?}", weather_status),
            format!("Epoch: {:?}", epoch),
        ],
    )
    .await?;

    Ok(())
}

pub async fn admin_set_new_admin(
    handler: &CliHandler,
    new_admin: &Pubkey,
    set_tie_breaker_admin: bool,
) -> Result<()> {
    let keypair = handler.keypair()?;
    let ncn = *handler.ncn()?;

    let config_pda = NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn).0;

    let roles = [(set_tie_breaker_admin, ConfigAdminRole::TieBreakerAdmin)];

    for (should_set, role) in roles.iter() {
        if !should_set {
            continue;
        }

        let mut ix = AdminSetNewAdminBuilder::new();
        ix.config(config_pda)
            .ncn(ncn)
            .ncn_admin(keypair.pubkey())
            .new_admin(*new_admin)
            .role(*role);

        send_and_log_transaction(
            handler,
            &[ix.instruction()],
            &[],
            "Admin Set New Admin",
            &[
                format!("NCN: {:?}", ncn),
                format!("New Admin: {:?}", new_admin),
                format!("Role: {:?}", role),
            ],
        )
        .await?;
    }

    Ok(())
}

pub async fn admin_set_parameters(
    handler: &CliHandler,
    epochs_before_stall: Option<u64>,
    epochs_after_consensus_before_close: Option<u64>,
    valid_slots_after_consensus: Option<u64>,
    starting_valid_epoch: Option<u64>,
) -> Result<()> {
    let keypair = handler.keypair()?;
    let ncn = *handler.ncn()?;

    let config_pda = NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn).0;

    let mut ix = AdminSetParametersBuilder::new();
    ix.config(config_pda).ncn(ncn).ncn_admin(keypair.pubkey());

    if let Some(epochs) = epochs_before_stall {
        ix.epochs_before_stall(epochs);
    }

    if let Some(epochs) = epochs_after_consensus_before_close {
        ix.epochs_after_consensus_before_close(epochs);
    }

    if let Some(slots) = valid_slots_after_consensus {
        ix.valid_slots_after_consensus(slots);
    }

    if let Some(epoch) = starting_valid_epoch {
        ix.starting_valid_epoch(epoch);
    }

    send_and_log_transaction(
        handler,
        &[ix.instruction()],
        &[],
        "Set Parameters",
        &[
            format!("NCN: {:?}", ncn),
            format!("Epochs Before Stall: {:?}", epochs_before_stall),
            format!(
                "Epochs After Consensus Before Close: {:?}",
                epochs_after_consensus_before_close
            ),
            format!(
                "Valid Slots After Consensus: {:?}",
                valid_slots_after_consensus
            ),
        ],
    )
    .await?;

    Ok(())
}

pub async fn admin_fund_account_payer(handler: &CliHandler, amount: f64) -> Result<()> {
    let keypair = handler.keypair()?;
    let ncn = *handler.ncn()?;

    let (account_payer, _, _) = AccountPayer::find_program_address(&handler.ncn_program_id, &ncn);

    let transfer_ix = transfer(&keypair.pubkey(), &account_payer, sol_to_lamports(amount));

    send_and_log_transaction(
        handler,
        &[transfer_ix],
        &[],
        "Fund Account Payer",
        &[
            format!("NCN: {:?}", ncn),
            format!("Amount: {:?} SOL", amount),
        ],
    )
    .await?;

    Ok(())
}

// --------------------- NCN Program ------------------------------

// ----------------------- Keeper ---------------------------------

pub async fn create_vault_registry(handler: &CliHandler) -> Result<()> {
    let ncn = *handler.ncn()?;

    let (config, _, _) = NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn);

    let (vault_registry, _, _) = VaultRegistry::find_program_address(&handler.ncn_program_id, &ncn);

    let (account_payer, _, _) = AccountPayer::find_program_address(&handler.ncn_program_id, &ncn);

    let vault_registry_account = get_account(handler, &vault_registry).await?;

    // Skip if vault registry already exists
    if vault_registry_account.is_none() {
        let initialize_vault_registry_ix = InitializeVaultRegistryBuilder::new()
            .config(config)
            .account_payer(account_payer)
            .ncn(ncn)
            .vault_registry(vault_registry)
            .instruction();

        send_and_log_transaction(
            handler,
            &[initialize_vault_registry_ix],
            &[],
            "Created Vault Registry",
            &[format!("NCN: {:?}", ncn)],
        )
        .await?;
    }

    // Number of reallocations needed based on VaultRegistry::SIZE
    let num_reallocs =
        ((VaultRegistry::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1).max(1);

    let realloc_vault_registry_ix = ReallocVaultRegistryBuilder::new()
        .config(config)
        .vault_registry(vault_registry)
        .ncn(ncn)
        .account_payer(account_payer)
        .system_program(system_program::id())
        .instruction();

    let mut realloc_ixs = Vec::with_capacity(num_reallocs as usize);
    realloc_ixs.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000));
    for _ in 0..num_reallocs {
        realloc_ixs.push(realloc_vault_registry_ix.clone());
    }

    send_and_log_transaction(
        handler,
        &realloc_ixs,
        &[],
        "Reallocated Vault Registry",
        &[
            format!("NCN: {:?}", ncn),
            format!("Number of reallocations: {:?}", num_reallocs),
        ],
    )
    .await?;

    Ok(())
}

pub async fn register_vault(handler: &CliHandler, vault: &Pubkey) -> Result<()> {
    let ncn = *handler.ncn()?;
    let vault = *vault;

    let (ncn_program_config, _, _) =
        NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn);

    let (vault_registry, _, _) = VaultRegistry::find_program_address(&handler.ncn_program_id, &ncn);

    let (ncn_vault_ticket, _, _) =
        NcnVaultTicket::find_program_address(&handler.restaking_program_id, &ncn, &vault);

    let register_vault_ix = RegisterVaultBuilder::new()
        .config(ncn_program_config)
        .vault_registry(vault_registry)
        .vault(vault)
        .ncn(ncn)
        .ncn_vault_ticket(ncn_vault_ticket)
        .vault_registry(vault_registry)
        .instruction();

    send_and_log_transaction(
        handler,
        &[register_vault_ix],
        &[],
        "Registered Vault",
        &[format!("NCN: {:?}", ncn), format!("Vault: {:?}", vault)],
    )
    .await?;

    Ok(())
}

pub async fn create_epoch_state(handler: &CliHandler, epoch: u64) -> Result<()> {
    let ncn = *handler.ncn()?;

    let (config, _, _) = NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn);

    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (account_payer, _, _) = AccountPayer::find_program_address(&handler.ncn_program_id, &ncn);
    let (epoch_marker, _, _) = EpochMarker::find_program_address(&ncn_program::id(), &ncn, epoch);

    let epoch_state_account = get_account(handler, &epoch_state).await?;

    // Skip if ballot box already exists
    if epoch_state_account.is_none() {
        // Initialize ballot box
        let initialize_epoch_state_ix = InitializeEpochStateBuilder::new()
            .epoch_marker(epoch_marker)
            .config(config)
            .epoch_state(epoch_state)
            .ncn(ncn)
            .epoch(epoch)
            .account_payer(account_payer)
            .system_program(system_program::id())
            .instruction();

        send_and_log_transaction(
            handler,
            &[initialize_epoch_state_ix],
            &[],
            "Initialized Epoch State",
            &[format!("NCN: {:?}", ncn), format!("Epoch: {:?}", epoch)],
        )
        .await?;
    }

    Ok(())
}

pub async fn create_weight_table(handler: &CliHandler, epoch: u64) -> Result<()> {
    let ncn = *handler.ncn()?;

    let (config, _, _) = NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn);

    let (vault_registry, _, _) = VaultRegistry::find_program_address(&handler.ncn_program_id, &ncn);

    let (weight_table, _, _) =
        WeightTable::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (account_payer, _, _) = AccountPayer::find_program_address(&handler.ncn_program_id, &ncn);
    let (epoch_marker, _, _) = EpochMarker::find_program_address(&ncn_program::id(), &ncn, epoch);

    let weight_table_account = get_account(handler, &weight_table).await?;

    // Skip if weight table already exists
    if weight_table_account.is_none() {
        // Initialize weight table
        let initialize_weight_table_ix = InitializeWeightTableBuilder::new()
            .epoch_marker(epoch_marker)
            .vault_registry(vault_registry)
            .ncn(ncn)
            .epoch_state(epoch_state)
            .weight_table(weight_table)
            .account_payer(account_payer)
            .system_program(system_program::id())
            .epoch(epoch)
            .instruction();

        send_and_log_transaction(
            handler,
            &[initialize_weight_table_ix],
            &[],
            "Initialized Weight Table",
            &[format!("NCN: {:?}", ncn), format!("Epoch: {:?}", epoch)],
        )
        .await?;
    }

    // Number of reallocations needed based on WeightTable::SIZE
    let num_reallocs = (WeightTable::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1;

    // Realloc weight table
    let realloc_weight_table_ix = ReallocWeightTableBuilder::new()
        .config(config)
        .weight_table(weight_table)
        .ncn(ncn)
        .epoch_state(epoch_state)
        .vault_registry(vault_registry)
        .epoch(epoch)
        .account_payer(account_payer)
        .system_program(system_program::id())
        .instruction();

    let mut realloc_ixs = Vec::with_capacity(num_reallocs as usize);
    realloc_ixs.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000));
    for _ in 0..num_reallocs {
        realloc_ixs.push(realloc_weight_table_ix.clone());
    }

    send_and_log_transaction(
        handler,
        &realloc_ixs,
        &[],
        "Reallocated Weight Table",
        &[
            format!("NCN: {:?}", ncn),
            format!("Epoch: {:?}", epoch),
            format!("Number of reallocations: {:?}", num_reallocs),
        ],
    )
    .await?;

    Ok(())
}

pub async fn set_epoch_weights(handler: &CliHandler, epoch: u64) -> Result<()> {
    let ncn = *handler.ncn()?;

    let (weight_table, _, _) =
        WeightTable::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (vault_registry, _, _) = VaultRegistry::find_program_address(&handler.ncn_program_id, &ncn);

    let set_epoch_weights_ix = SetEpochWeightsBuilder::new()
        .ncn(ncn)
        .weight_table(weight_table)
        .epoch_state(epoch_state)
        .vault_registry(vault_registry)
        .epoch(epoch)
        .instruction();

    send_and_log_transaction(
        handler,
        &[set_epoch_weights_ix],
        &[],
        "Set Epoch Weights",
        &[
            format!("NCN: {:?}", ncn),
            format!("Epoch: {:?}", epoch),
            format!("Weight Table: {:?}", weight_table),
            format!("Epoch State: {:?}", epoch_state),
            format!("Vault Registry: {:?}", vault_registry),
        ],
    )
    .await?;

    Ok(())
}

pub async fn create_epoch_snapshot(handler: &CliHandler, epoch: u64) -> Result<()> {
    let ncn = *handler.ncn()?;

    let (config, _, _) = NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn);

    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (weight_table, _, _) =
        WeightTable::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (epoch_snapshot, _, _) =
        EpochSnapshot::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (account_payer, _, _) = AccountPayer::find_program_address(&handler.ncn_program_id, &ncn);
    let (epoch_marker, _, _) = EpochMarker::find_program_address(&ncn_program::id(), &ncn, epoch);

    let initialize_epoch_snapshot_ix = InitializeEpochSnapshotBuilder::new()
        .epoch_marker(epoch_marker)
        .config(config)
        .ncn(ncn)
        .epoch_state(epoch_state)
        .weight_table(weight_table)
        .epoch_snapshot(epoch_snapshot)
        .account_payer(account_payer)
        .system_program(system_program::id())
        .epoch(epoch)
        .instruction();

    send_and_log_transaction(
        handler,
        &[initialize_epoch_snapshot_ix],
        &[],
        "Initialized Epoch Snapshot",
        &[format!("NCN: {:?}", ncn), format!("Epoch: {:?}", epoch)],
    )
    .await?;

    Ok(())
}

pub async fn create_operator_snapshot(
    handler: &CliHandler,
    operator: &Pubkey,
    epoch: u64,
) -> Result<()> {
    let ncn = *handler.ncn()?;

    let operator = *operator;

    let (config, _, _) = NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn);

    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (ncn_operator_state, _, _) =
        NcnOperatorState::find_program_address(&handler.restaking_program_id, &ncn, &operator);

    let (epoch_snapshot, _, _) =
        EpochSnapshot::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (operator_snapshot, _, _) =
        OperatorSnapshot::find_program_address(&handler.ncn_program_id, &operator, &ncn, epoch);

    let (account_payer, _, _) = AccountPayer::find_program_address(&handler.ncn_program_id, &ncn);
    let (epoch_marker, _, _) = EpochMarker::find_program_address(&ncn_program::id(), &ncn, epoch);

    let operator_snapshot_account = get_account(handler, &operator_snapshot).await?;

    // Skip if operator snapshot already exists
    if operator_snapshot_account.is_none() {
        // Initialize operator snapshot
        let initialize_operator_snapshot_ix = InitializeOperatorSnapshotBuilder::new()
            .epoch_marker(epoch_marker)
            .config(config)
            .restaking_config(
                RestakingConfig::find_program_address(&handler.restaking_program_id).0,
            )
            .ncn(ncn)
            .operator(operator)
            .epoch_state(epoch_state)
            .ncn_operator_state(ncn_operator_state)
            .epoch_snapshot(epoch_snapshot)
            .operator_snapshot(operator_snapshot)
            .account_payer(account_payer)
            .system_program(system_program::id())
            .epoch(epoch)
            .instruction();

        send_and_log_transaction(
            handler,
            &[initialize_operator_snapshot_ix],
            &[],
            "Initialized Operator Snapshot",
            &[
                format!("NCN: {:?}", ncn),
                format!("Operator: {:?}", operator),
                format!("Epoch: {:?}", epoch),
            ],
        )
        .await?;
    }

    Ok(())
}

pub async fn snapshot_vault_operator_delegation(
    handler: &CliHandler,
    vault: &Pubkey,
    operator: &Pubkey,
    epoch: u64,
) -> Result<()> {
    let ncn = *handler.ncn()?;

    let vault = *vault;
    let operator = *operator;

    let (config, _, _) = NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn);

    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (restaking_config, _, _) =
        RestakingConfig::find_program_address(&handler.restaking_program_id);

    let (vault_ncn_ticket, _, _) =
        VaultNcnTicket::find_program_address(&handler.vault_program_id, &vault, &ncn);

    let (ncn_vault_ticket, _, _) =
        NcnVaultTicket::find_program_address(&handler.restaking_program_id, &ncn, &vault);

    let (vault_operator_delegation, _, _) =
        VaultOperatorDelegation::find_program_address(&handler.vault_program_id, &vault, &operator);

    let (weight_table, _, _) =
        WeightTable::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (epoch_snapshot, _, _) =
        EpochSnapshot::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (operator_snapshot, _, _) =
        OperatorSnapshot::find_program_address(&handler.ncn_program_id, &operator, &ncn, epoch);

    let snapshot_vault_operator_delegation_ix = SnapshotVaultOperatorDelegationBuilder::new()
        .config(config)
        .epoch_state(epoch_state)
        .restaking_config(restaking_config)
        .ncn(ncn)
        .operator(operator)
        .vault(vault)
        .vault_ncn_ticket(vault_ncn_ticket)
        .ncn_vault_ticket(ncn_vault_ticket)
        .vault_operator_delegation(vault_operator_delegation)
        .weight_table(weight_table)
        .epoch_snapshot(epoch_snapshot)
        .operator_snapshot(operator_snapshot)
        .epoch(epoch)
        .instruction();

    send_and_log_transaction(
        handler,
        &[snapshot_vault_operator_delegation_ix],
        &[],
        "Snapshotted Vault Operator Delegation",
        &[
            format!("NCN: {:?}", ncn),
            format!("Vault: {:?}", vault),
            format!("Operator: {:?}", operator),
            format!("Epoch: {:?}", epoch),
        ],
    )
    .await?;

    Ok(())
}

pub async fn create_ballot_box(handler: &CliHandler, epoch: u64) -> Result<()> {
    let ncn = *handler.ncn()?;

    let (config, _, _) = NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn);

    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (ballot_box, _, _) = BallotBox::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (account_payer, _, _) = AccountPayer::find_program_address(&handler.ncn_program_id, &ncn);
    let (epoch_marker, _, _) = EpochMarker::find_program_address(&ncn_program::id(), &ncn, epoch);
    let (consensus_result, _, _) =
        ConsensusResult::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let ballot_box_account = get_account(handler, &ballot_box).await?;

    // Skip if ballot box already exists
    if ballot_box_account.is_none() {
        // Initialize ballot box
        let initialize_ballot_box_ix = InitializeBallotBoxBuilder::new()
            .epoch_marker(epoch_marker)
            .config(config)
            .epoch_state(epoch_state)
            .ballot_box(ballot_box)
            .ncn(ncn)
            .epoch(epoch)
            .account_payer(account_payer)
            .consensus_result(consensus_result)
            .system_program(system_program::id())
            .instruction();

        send_and_log_transaction(
            handler,
            &[initialize_ballot_box_ix],
            &[],
            "Initialized Ballot Box",
            &[format!("NCN: {:?}", ncn), format!("Epoch: {:?}", epoch)],
        )
        .await?;
    }

    // Number of reallocations needed based on BallotBox::SIZE
    let num_reallocs = (BallotBox::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1;

    // Realloc ballot box
    let realloc_ballot_box_ix = ReallocBallotBoxBuilder::new()
        .config(config)
        .epoch_state(epoch_state)
        .ballot_box(ballot_box)
        .ncn(ncn)
        .epoch(epoch)
        .account_payer(account_payer)
        .system_program(system_program::id())
        .instruction();

    let mut realloc_ixs = Vec::with_capacity(num_reallocs as usize);
    realloc_ixs.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000));
    for _ in 0..num_reallocs {
        realloc_ixs.push(realloc_ballot_box_ix.clone());
    }

    send_and_log_transaction(
        handler,
        &realloc_ixs,
        &[],
        "Reallocated Ballot Box",
        &[
            format!("NCN: {:?}", ncn),
            format!("Epoch: {:?}", epoch),
            format!("Number of reallocations: {:?}", num_reallocs),
        ],
    )
    .await?;

    Ok(())
}

pub async fn close_epoch_account(
    handler: &CliHandler,
    ncn: Pubkey,
    epoch: u64,
    account_to_close: Pubkey,
) -> Result<()> {
    let (epoch_marker, _, _) =
        EpochMarker::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (account_payer, _, _) = AccountPayer::find_program_address(&handler.ncn_program_id, &ncn);

    let (config, _, _) = NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn);

    let account_already_closed = get_account(handler, &account_to_close)
        .await?
        .map_or(true, |account| {
            account.data.is_empty() || account.lamports == 0
        });
    if account_already_closed {
        info!("Account already closed: {:?}", account_to_close);
        return Ok(());
    }

    let mut ix = CloseEpochAccountBuilder::new();

    ix.account_payer(account_payer)
        .epoch_marker(epoch_marker)
        .config(config)
        .account_to_close(account_to_close)
        .epoch_state(epoch_state)
        .ncn(ncn)
        .system_program(system_program::id())
        .epoch(epoch);

    send_and_log_transaction(
        handler,
        &[ix.instruction()],
        &[],
        "Close Epoch Account",
        &[
            format!("NCN: {:?}", ncn),
            format!("Account to Close: {:?}", account_to_close),
            format!("Epoch: {:?}", epoch),
        ],
    )
    .await?;

    Ok(())
}

// --------------------- operator ------------------------------

pub async fn operator_cast_vote(
    handler: &CliHandler,
    operator: &Pubkey,
    epoch: u64,
    weather_status: u8,
) -> Result<()> {
    let keypair = handler.keypair()?;

    let ncn = *handler.ncn()?;

    let operator = *operator;

    let (config, _, _) = NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn);

    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (ballot_box, _, _) = BallotBox::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (epoch_snapshot, _, _) =
        EpochSnapshot::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (operator_snapshot, _, _) =
        OperatorSnapshot::find_program_address(&handler.ncn_program_id, &operator, &ncn, epoch);
    let (consensus_result, _, _) =
        ConsensusResult::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let cast_vote_ix = CastVoteBuilder::new()
        .config(config)
        .epoch_state(epoch_state)
        .ballot_box(ballot_box)
        .ncn(ncn)
        .epoch_snapshot(epoch_snapshot)
        .operator_snapshot(operator_snapshot)
        .operator(operator)
        .operator_voter(keypair.pubkey())
        .consensus_result(consensus_result)
        .weather_status(weather_status)
        .epoch(epoch)
        .instruction();

    send_and_log_transaction(
        handler,
        &[cast_vote_ix],
        &[],
        "Cast Vote",
        &[
            format!("NCN: {:?}", ncn),
            format!("Operator: {:?}", operator),
            format!(
                "Weather Status: {:?}",
                WeatherStatus::from_u8(weather_status)
            ),
            format!("Epoch: {:?}", epoch),
        ],
    )
    .await?;

    Ok(())
}

// --------------------- MIDDLEWARE ------------------------------

pub async fn get_consensus_result_instruction(handler: &CliHandler, epoch: u64) -> Result<()> {
    let consensus_result = get_consensus_result(handler, epoch).await?;

    info!(
        "Consensus Result for epoch {}: {:?}",
        epoch, consensus_result
    );

    Ok(())
}

pub const CREATE_TIMEOUT_MS: u64 = 2000;
pub const CREATE_GET_RETRIES: u64 = 3;
pub async fn check_created(handler: &CliHandler, address: &Pubkey) -> Result<()> {
    let mut retries = 0;
    let mut account = get_account(handler, address).await?;
    while account.is_none() && retries < CREATE_GET_RETRIES {
        sleep(Duration::from_millis(CREATE_TIMEOUT_MS * (retries + 1))).await;
        account = get_account(handler, address).await?;
        retries += 1;
    }

    if account.is_none() {
        return Err(anyhow!(
            "Failed to get account after creation {:?}",
            address
        ));
    }

    Ok(())
}

pub async fn update_all_vaults_in_network(handler: &CliHandler) -> Result<()> {
    let vaults = get_all_vaults(handler).await?;
    for vault in vaults {
        full_vault_update(handler, &vault).await?;
    }

    Ok(())
}

pub async fn full_vault_update(handler: &CliHandler, vault: &Pubkey) -> Result<()> {
    let payer = handler.keypair()?;

    // Get Epoch Info
    let current_slot = get_current_slot(handler).await?;
    let (ncn_epoch, epoch_length) = {
        let vault_config = get_vault_config(handler).await?;
        let ncn_epoch = vault_config.get_epoch_from_slot(current_slot)?;
        let epoch_length = vault_config.epoch_length();
        (ncn_epoch, epoch_length)
    };

    // Check Vault
    let vault_account = get_vault(handler, vault).await?;
    let current_slot = get_current_slot(handler).await?;

    if !vault_account.is_update_needed(current_slot, epoch_length)? {
        return Ok(());
    }

    // Initialize Vault Update State Tracker
    let (vault_config, _, _) = VaultConfig::find_program_address(&handler.vault_program_id);

    let (vault_update_state_tracker, _, _) =
        VaultUpdateStateTracker::find_program_address(&handler.vault_program_id, vault, ncn_epoch);

    let vault_update_state_tracker_account =
        get_account(handler, &vault_update_state_tracker).await?;

    if vault_update_state_tracker_account.is_none() {
        let initialize_vault_update_state_tracker_ix =
            InitializeVaultUpdateStateTrackerBuilder::new()
                .vault(*vault)
                .vault_update_state_tracker(vault_update_state_tracker)
                .system_program(system_program::id())
                .withdrawal_allocation_method(WithdrawalAllocationMethod::Greedy)
                .payer(payer.pubkey())
                .config(vault_config)
                .instruction();

        let result = send_and_log_transaction(
            handler,
            &[initialize_vault_update_state_tracker_ix],
            &[payer],
            "Initialize Vault Update State Tracker",
            &[
                format!("VAULT: {:?}", vault),
                format!("Vault Epoch: {:?}", ncn_epoch),
            ],
        )
        .await;

        if result.is_err() {
            log::error!(
                "Failed to initialize Vault Update State Tracker for Vault: {:?} at NCN Epoch: {:?} with error: {:?}",
                vault,
                ncn_epoch,
                result.err().unwrap()
            );
        }
    }

    // Crank Vault Update State Tracker
    let all_operators = get_all_sorted_operators_for_vault(handler, vault).await?;

    if !all_operators.is_empty() {
        let starting_index = {
            let vault_update_state_tracker_account =
                get_vault_update_state_tracker(handler, vault, ncn_epoch).await?;
            let last_updated_index = vault_update_state_tracker_account.last_updated_index();

            if last_updated_index == u64::MAX {
                ncn_epoch % all_operators.len() as u64
            } else {
                (last_updated_index + 1) % all_operators.len() as u64
            }
        };

        for index in 0..all_operators.len() {
            let current_index = (starting_index as usize + index) % all_operators.len();
            let operator = all_operators.get(current_index).unwrap();

            let (vault_operator_delegation, _, _) = VaultOperatorDelegation::find_program_address(
                &handler.vault_program_id,
                vault,
                operator,
            );

            let crank_vault_update_state_tracker_ix = CrankVaultUpdateStateTrackerBuilder::new()
                .vault(*vault)
                .operator(*operator)
                .config(vault_config)
                .vault_operator_delegation(vault_operator_delegation)
                .vault_update_state_tracker(vault_update_state_tracker)
                .instruction();

            let result = send_and_log_transaction(
                handler,
                &[crank_vault_update_state_tracker_ix],
                &[payer],
                "Crank Vault Update State Tracker",
                &[
                    format!("VAULT: {:?}", vault),
                    format!("Operator: {:?}", operator),
                    format!("Vault Epoch: {:?}", ncn_epoch),
                ],
            )
            .await;

            if result.is_err() {
                log::error!(
                "Failed to crank Vault Update State Tracker for Vault: {:?} and Operator: {:?} at NCN Epoch: {:?} with error: {:?}",
                vault,
                operator,
                ncn_epoch,
                result.err().unwrap()
            );
            }
        }
    }

    // Close Update State Tracker
    let vault_update_state_tracker_account =
        get_account(handler, &vault_update_state_tracker).await?;

    if vault_update_state_tracker_account.is_some() {
        let close_vault_update_state_tracker_ix = CloseVaultUpdateStateTrackerBuilder::new()
            .vault(*vault)
            .vault_update_state_tracker(vault_update_state_tracker)
            .payer(payer.pubkey())
            .config(vault_config)
            .ncn_epoch(ncn_epoch)
            .instruction();

        let result = send_and_log_transaction(
            handler,
            &[close_vault_update_state_tracker_ix],
            &[payer],
            "Close Vault Update State Tracker",
            &[
                format!("VAULT: {:?}", vault),
                format!("Vault Epoch: {:?}", ncn_epoch),
            ],
        )
        .await;

        if result.is_err() {
            log::error!(
                "Failed to close Vault Update State Tracker for Vault: {:?} at NCN Epoch: {:?} with error: {:?}",
                vault,
                ncn_epoch,
                result.err().unwrap()
            );
        }
    }

    Ok(())
}

pub async fn get_or_create_weight_table(handler: &CliHandler, epoch: u64) -> Result<WeightTable> {
    let ncn = *handler.ncn()?;

    let (weight_table, _, _) =
        WeightTable::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    if get_account(handler, &weight_table)
        .await?
        .map_or(true, |table| table.data.len() < WeightTable::SIZE)
    {
        create_weight_table(handler, epoch).await?;
        check_created(handler, &weight_table).await?;
    }
    get_weight_table(handler, epoch).await
}

pub async fn get_or_create_epoch_snapshot(
    handler: &CliHandler,
    epoch: u64,
) -> Result<EpochSnapshot> {
    let ncn = *handler.ncn()?;
    let (epoch_snapshot, _, _) =
        EpochSnapshot::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    if get_account(handler, &epoch_snapshot)
        .await?
        .map_or(true, |snapshot| snapshot.data.len() < EpochSnapshot::SIZE)
    {
        create_epoch_snapshot(handler, epoch).await?;
        check_created(handler, &epoch_snapshot).await?;
    }

    get_epoch_snapshot(handler, epoch).await
}

pub async fn get_or_create_ncn_reward_router(
    handler: &CliHandler,
    epoch: u64,
) -> Result<NCNRewardRouter> {
    let ncn = *handler.ncn()?;
    let (ncn_reward_router, _, _) =
        NCNRewardRouter::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    if get_account(handler, &ncn_reward_router)
        .await?
        .map_or(true, |router| router.data.len() < NCNRewardRouter::SIZE)
    {
        create_ncn_reward_router(handler, epoch).await?;
        check_created(handler, &ncn_reward_router).await?;
    }
    get_ncn_reward_router(handler, epoch).await
}

pub async fn get_or_create_operator_vault_reward_router(
    handler: &CliHandler,
    operator: &Pubkey,
    epoch: u64,
) -> Result<OperatorVaultRewardRouter> {
    let ncn = *handler.ncn()?;
    let (operator_snapshot, _, _) =
        OperatorSnapshot::find_program_address(&handler.ncn_program_id, operator, &ncn, epoch);

    let (operator_vault_reward_router, _, _) = OperatorVaultRewardRouter::find_program_address(
        &handler.ncn_program_id,
        operator,
        &ncn,
        epoch,
    );

    // If operator snapshot does not exist, we cannot create the ncn reward router
    if get_account(handler, &operator_snapshot).await?.is_none() {
        return Err(anyhow!("Invalid Route"));
    }

    if get_account(handler, &operator_vault_reward_router)
        .await?
        .map_or(true, |router| {
            router.data.len() < OperatorVaultRewardRouter::SIZE
        })
    {
        create_operator_vault_reward_router(handler, operator, epoch).await?;
        check_created(handler, &operator_vault_reward_router).await?;
    }
    get_operator_vault_reward_router(handler, operator, epoch).await
}

pub async fn get_or_create_operator_snapshot(
    handler: &CliHandler,
    operator: &Pubkey,
    epoch: u64,
) -> Result<OperatorSnapshot> {
    let ncn = *handler.ncn()?;
    let (operator_snapshot, _, _) =
        OperatorSnapshot::find_program_address(&handler.ncn_program_id, operator, &ncn, epoch);

    if get_account(handler, &operator_snapshot)
        .await?
        .map_or(true, |snapshot| {
            snapshot.data.len() < OperatorSnapshot::SIZE
        })
    {
        create_operator_snapshot(handler, operator, epoch).await?;
        check_created(handler, &operator_snapshot).await?;
    }
    get_operator_snapshot(handler, operator, epoch).await
}

#[allow(clippy::large_stack_frames)]
pub async fn get_or_create_ballot_box(handler: &CliHandler, epoch: u64) -> Result<BallotBox> {
    let ncn = *handler.ncn()?;
    let (ballot_box, _, _) = BallotBox::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    if get_account(handler, &ballot_box)
        .await?
        .map_or(true, |ballot_box| ballot_box.data.len() < BallotBox::SIZE)
    {
        create_ballot_box(handler, epoch).await?;
        check_created(handler, &ballot_box).await?;
    }
    get_ballot_box(handler, epoch).await
}

// --------------------- CRANKERS ------------------------------

pub async fn crank_register_vaults(handler: &CliHandler) -> Result<()> {
    let all_ncn_vaults = get_all_vaults_in_ncn(handler).await?;
    let vault_registry = get_or_create_vault_registry(handler).await?;
    let all_registered_vaults: Vec<Pubkey> = vault_registry
        .get_valid_vault_entries()
        .iter()
        .map(|entry| *entry.vault())
        .collect();

    let vaults_to_register: Vec<Pubkey> = all_ncn_vaults
        .iter()
        .filter(|vault| !all_registered_vaults.contains(vault))
        .copied()
        .collect();

    for vault in vaults_to_register.iter() {
        let result = register_vault(handler, vault).await;

        if let Err(err) = result {
            log::error!(
                "Failed to register vault: {:?} with error: {:?}",
                vault,
                err
            );
        }
    }

    Ok(())
}

pub async fn crank_snapshot(handler: &CliHandler, epoch: u64) -> Result<()> {
    let vault_registry = get_vault_registry(handler).await?;

    let operators = get_all_operators_in_ncn(handler).await?;
    let all_vaults: Vec<Pubkey> = vault_registry
        .get_valid_vault_entries()
        .iter()
        .map(|entry| *entry.vault())
        .collect();

    let epoch_snapshot = get_or_create_epoch_snapshot(handler, epoch).await?;
    if !epoch_snapshot.finalized() {
        for operator in operators.iter() {
            // Create Vault Operator Delegation
            let result = get_or_create_operator_snapshot(handler, operator, epoch).await;

            if result.is_err() {
                log::error!(
                "Failed to get or create operator snapshot for operator: {:?} in epoch: {:?} with error: {:?}",
                operator,
                epoch,
                result.err().unwrap()
            );
                continue;
            };

            let operator_snapshot = result?;

            let vaults_to_run: Vec<Pubkey> = all_vaults
                .iter()
                .filter(|vault| !operator_snapshot.contains_vault(vault))
                .cloned()
                .collect();

            for vault in vaults_to_run.iter() {
                let result = full_vault_update(handler, vault).await;

                if let Err(err) = result {
                    log::error!(
                        "Failed to update the vault: {:?} with error: {:?}",
                        vault,
                        err
                    );
                }

                let result =
                    snapshot_vault_operator_delegation(handler, vault, operator, epoch).await;

                if let Err(err) = result {
                    log::error!(
                    "Failed to snapshot vault operator delegation for vault: {:?} and operator: {:?} in epoch: {:?} with error: {:?}",
                    vault,
                    operator,
                    epoch,
                    err
                );
                }
            }
        }
    }

    let result = get_or_create_ballot_box(handler, epoch).await;
    if let Err(err) = result {
        log::error!(
            "Failed to get or create ballot box for epoch: {:?} with error: {:?}",
            epoch,
            err
        );
    }

    Ok(())
}

#[derive(Deserialize, Debug)]
struct WeatherInfo {
    main: String,
}

#[derive(Deserialize, Debug)]
struct WeatherResponse {
    weather: Vec<WeatherInfo>,
}

async fn get_weather_status(api_key: &str, city_name: &str) -> Result<u8> {
    let url = format!(
        "http://api.openweathermap.org/data/2.5/weather?q={}&appid={}&units=metric",
        city_name, api_key
    );

    let response = reqwest::get(&url).await?.json::<WeatherResponse>().await?;

    if let Some(weather_condition) = response.weather.get(0) {
        match weather_condition.main.as_str() {
            "Clear" => Ok(0),                                      // Sunny
            "Rain" | "Snow" | "Drizzle" | "Thunderstorm" => Ok(2), // Raining/Snowing
            _ => Ok(1),                                            // Anything else
        }
    } else {
        Ok(1) // Default to "Anything else" if no weather info is available
    }
}

/// Casts a vote for an operator based on the current weather in Solana Beach
///
/// # Arguments
/// * `handler` - CLI handler for RPC communication
/// * `epoch` - Current epoch number
/// * `operator` - Public key of the operator voting
///
/// # Returns
/// * `Result<u8>` - Weather value that was voted (0:Sunny, 1:Other, 2:Rain/Snow)
pub async fn operator_crank_vote(
    handler: &CliHandler,
    epoch: u64,
    operator: &Pubkey,
) -> Result<u8> {
    // Get API key for weather service
    let api_key = handler.open_weather_api_key()?;

    // Fetch current weather status from OpenWeather API
    let weather_value = get_weather_status(&api_key, "Solana Beach").await?;
    info!(
        "Current weather in Solana Beach (0:Sunny, 1:Other, 2:Rain/Snow): {}",
        weather_value
    );

    // Cast the vote with the weather value
    operator_cast_vote(handler, operator, epoch, weather_value).await?;
    Ok(weather_value)
}

/// Logs detailed information about an operator's vote and ballot box state
///
/// This function retrieves the ballot box for the current epoch and logs:
/// - Whether the operator voted
/// - Details of the operator's vote if they voted
/// - Current consensus status
/// - Winning ballot if consensus is reached
///
/// # Arguments
/// * `handler` - CLI handler for RPC communication
/// * `epoch` - Current epoch number
/// * `operator` - Public key of the operator to check
///
/// # Returns
/// * `Result<()>` - Success or failure
pub async fn operator_crank_post_vote(
    handler: &CliHandler,
    epoch: u64,
    operator: &Pubkey,
) -> Result<()> {
    // Get the ballot box for the current epoch
    let ballot_box = get_ballot_box(handler, epoch).await?;

    // Check if operator voted and get their vote details
    let did_operator_vote = ballot_box.did_operator_vote(operator);
    let operator_vote = if did_operator_vote {
        ballot_box
            .operator_votes()
            .iter()
            .find(|v| v.operator().eq(&operator))
    } else {
        None
    };

    // Build detailed log message
    let mut log_message = format!("\n----- Post Vote Status -----\n");
    log_message.push_str(&format!("Epoch: {}\n", epoch));
    log_message.push_str(&format!("Did Operator Vote: {}\n", did_operator_vote));

    // Add operator vote details if they voted
    if let Some(vote) = operator_vote {
        let operator_ballot = ballot_box.ballot_tallies()[vote.ballot_index() as usize];
        let operator_ballot_weight = operator_ballot.stake_weights();
        log_message.push_str("Operator Vote Details:\n");
        log_message.push_str(&format!("  Operator: {}\n", vote.operator()));
        log_message.push_str(&format!("  Slot Voted: {}\n", vote.slot_voted()));
        log_message.push_str(&format!("  Ballot Index: {}\n", vote.ballot_index()));
        log_message.push_str(&format!(
            "  Operator Ballot Weight: {}\n",
            operator_ballot_weight.stake_weight()
        ));
        log_message.push_str(&format!("  Operator Vote: {}\n", operator_ballot.ballot()));
    } else {
        log_message.push_str("No operator vote found\n");
    }

    // Add consensus information
    log_message.push_str(&format!(
        "Consensus Reached: {}\n",
        ballot_box.is_consensus_reached()
    ));

    // Add winning ballot if consensus is reached
    if ballot_box.is_consensus_reached() {
        log_message.push_str(&format!(
            "Winning Ballot: {}\n",
            ballot_box.get_winning_ballot()?
        ));
    }
    log_message.push_str("--------------------------\n");

    // Log the complete message
    log::info!("{}", log_message);

    Ok(())
}

#[allow(clippy::large_stack_frames)]
pub async fn crank_test_vote(handler: &CliHandler, epoch: u64) -> Result<()> {
    let voter = handler.keypair()?.pubkey();
    let weather_status = 0;
    let operators = get_all_operators_in_ncn(handler).await?;

    for operator in operators.iter() {
        let operator_account = get_operator(handler, operator).await?;

        if operator_account.voter.ne(&voter) {
            continue;
        }

        let result = operator_cast_vote(handler, operator, epoch, weather_status).await;

        if let Err(err) = result {
            log::error!(
                "Failed to cast vote for operator: {:?} in epoch: {:?} with error: {:?}",
                operator,
                epoch,
                err
            );
        }
    }

    Ok(())
}

pub async fn crank_close_epoch_accounts(handler: &CliHandler, epoch: u64) -> Result<()> {
    let ncn = *handler.ncn()?;

    // Close NCN Reward Routers
    let operators = get_all_operators_in_ncn(handler).await?;

    // Close Ballot Box
    let (ballot_box, _, _) = BallotBox::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let result = close_epoch_account(handler, ncn, epoch, ballot_box).await;

    if let Err(err) = result {
        log::error!(
            "Failed to close ballot box: {:?} in epoch: {:?} with error: {:?}",
            ballot_box,
            epoch,
            err
        );
    }

    // Close Operator Snapshots
    for operator in operators.iter() {
        let (operator_snapshot, _, _) =
            OperatorSnapshot::find_program_address(&handler.ncn_program_id, operator, &ncn, epoch);

        let result = close_epoch_account(handler, ncn, epoch, operator_snapshot).await;

        if let Err(err) = result {
            log::error!(
                "Failed to close operator snapshot: {:?} in epoch: {:?} with error: {:?}",
                operator_snapshot,
                epoch,
                err
            );
        }
    }

    // Close Epoch Snapshot
    let (epoch_snapshot, _, _) =
        EpochSnapshot::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let result = close_epoch_account(handler, ncn, epoch, epoch_snapshot).await;

    if let Err(err) = result {
        log::error!(
            "Failed to close epoch snapshot: {:?} in epoch: {:?} with error: {:?}",
            epoch_snapshot,
            epoch,
            err
        );
    }

    // Close Weight Table
    let (weight_table, _, _) =
        WeightTable::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let result = close_epoch_account(handler, ncn, epoch, weight_table).await;

    if let Err(err) = result {
        log::error!(
            "Failed to close weight table: {:?} in epoch: {:?} with error: {:?}",
            weight_table,
            epoch,
            err
        );
    }

    // Close Epoch State
    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let result = close_epoch_account(handler, ncn, epoch, epoch_state).await;

    if let Err(err) = result {
        log::error!(
            "Failed to close epoch state: {:?} in epoch: {:?} with error: {:?}",
            epoch_state,
            epoch,
            err
        );
    }

    Ok(())
}

pub async fn crank_set_weight(handler: &CliHandler, epoch: u64) -> Result<()> {
    create_weight_table(handler, epoch).await?;
    set_epoch_weights(handler, epoch).await?;
    Ok(())
}

pub async fn crank_post_vote_cooldown(handler: &CliHandler, epoch: u64) -> Result<()> {
    let result = get_consensus_result(handler, epoch).await?;

    info!(
        "\n\n--- Consensus Result for epoch {} is: \n {} ---",
        epoch, result
    );
    Ok(())
}

pub async fn crank_distribute(handler: &CliHandler, epoch: u64) -> Result<()> {
    let operators = get_all_operators_in_ncn(handler).await?;

    let ncn_reward_router = get_or_create_ncn_reward_router(handler, epoch).await?;

    let ncn_reward_receiver_rewards = get_ncn_reward_receiver_rewards(handler, epoch).await?;
    info!(
        "NCN Reward Receiver Rewards for epoch {}: {}",
        epoch, ncn_reward_receiver_rewards
    );
    if ncn_reward_receiver_rewards > 0 {
        route_ncn_rewards(handler, epoch).await?;
    }

    // NCN Rewards Distribution
    {
        let result = distribute_ncn_rewards(handler, epoch).await;

        if let Err(err) = result {
            log::error!(
                "Failed to distribute base rewards for  in epoch: {:?} with error: {:?}",
                epoch,
                err
            );
        }
    }

    // Protocol Rewards Distribution
    {
        let result = distribute_protocol_rewards(handler, epoch).await;
        if let Err(err) = result {
            log::error!(
                "Failed to distribute protocol rewards for in epoch: {:?} with error: {:?}",
                epoch,
                err
            );
        }
    }

    for operator in operators.iter() {
        let result = get_or_create_operator_vault_reward_router(handler, operator, epoch).await;
        if let Err(err) = result {
            log::info!(
                "Skipping ncn reward router: {:?} in epoch: {:?} ( {:?} )",
                operator,
                epoch,
                err
            );
            continue;
        }

        let operator_route_result = ncn_reward_router.oprtator_vault_reward_route(operator);
        if operator_route_result.is_err() {
            log::info!(
                "Skipping route for operator: {:?} for in epoch: {:?} (No Route)",
                operator,
                epoch,
            );
            continue;
        }

        let operator_route = operator_route_result.unwrap();
        let has_rewards = operator_route.has_rewards().unwrap_or(false);

        if has_rewards {
            let result = distribute_operator_vault_rewards(handler, operator, epoch).await;

            if let Err(err) = result {
                log::error!(
                    "Failed to distribute base ncn rewards for operator: {:?} in epoch: {:?} with error: {:?}",
                    operator,
                    epoch,
                    err
                );
                continue;
            }
        }

        let operator_vault_reward_receiver_rewards =
            get_operator_vault_reward_receiver_rewards(handler, operator, epoch).await?;

        if operator_vault_reward_receiver_rewards > 0 {
            let result = route_operator_vault_rewards(handler, operator, epoch).await;

            if let Err(err) = result {
                log::error!(
                    "Failed to route ncn rewards for operator: {:?} in epoch: {:?} with error: {:?}",
                    operator,
                    epoch,
                    err
                );
                continue;
            }
        }

        let result = get_or_create_operator_vault_reward_router(handler, operator, epoch).await;
        if let Err(err) = result {
            log::info!(
                "Skipping ncn reward router: {:?} in epoch: {:?} ( {:?} )",
                operator,
                epoch,
                err
            );
            continue;
        }
        let operator_vault_reward_router = result?;

        if operator_vault_reward_router.operator_rewards() != 0 {
            let result = distribute_ncn_operator_rewards(handler, operator, epoch).await;

            if let Err(err) = result {
                log::error!(
                    "Failed to distribute ncn operator rewards for operator: {:?} in epoch: {:?} with error: {:?}",
                    operator,
                    epoch,
                    err
                );
                continue;
            }
        }

        let vaults_to_route = operator_vault_reward_router
            .vault_reward_routes()
            .iter()
            .filter(|route| !route.is_empty() && route.has_rewards())
            .map(|route| route.vault())
            .collect::<Vec<Pubkey>>();

        for vault in vaults_to_route {
            let result: std::result::Result<(), anyhow::Error> =
                distribute_ncn_vault_rewards(handler, &vault, operator, epoch).await;

            if let Err(err) = result {
                log::error!(
                        "Failed to distribute ncn vault rewards for vault: {:?} and operator: {:?} in epoch: {:?} with error: {:?}",
                        vault,
                        operator,
                        epoch,
                        err
                    );
            }
        }
    }

    Ok(())
}

// --------------------- Reward Routing Functions ------------------------------

pub async fn create_ncn_reward_router(handler: &CliHandler, epoch: u64) -> Result<()> {
    let ncn = *handler.ncn()?;

    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (ncn_reward_router, _, _) =
        NCNRewardRouter::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (ncn_reward_receiver, _, _) =
        NCNRewardReceiver::find_program_address(&handler.ncn_program_id, handler.ncn()?, epoch);

    let (account_payer, _, _) = AccountPayer::find_program_address(&handler.ncn_program_id, &ncn);
    let (epoch_marker, _, _) = EpochMarker::find_program_address(&ncn_program::id(), &ncn, epoch);

    let ncn_reward_router_account = get_account(handler, &ncn_reward_router).await?;

    // Skip if reward router already exists
    if ncn_reward_router_account.is_none() {
        // Initialize reward router
        let initialize_ncn_reward_router_ix = InitializeNCNRewardRouterBuilder::new()
            .epoch_marker(epoch_marker)
            .epoch_state(epoch_state)
            .ncn_reward_router(ncn_reward_router)
            .ncn_reward_receiver(ncn_reward_receiver)
            .ncn(ncn)
            .epoch(epoch)
            .account_payer(account_payer)
            .system_program(system_program::id())
            .instruction();

        send_and_log_transaction(
            handler,
            &[initialize_ncn_reward_router_ix],
            &[],
            "Initialized NCN Reward Router",
            &[format!("NCN: {:?}", ncn), format!("Epoch: {:?}", epoch)],
        )
        .await?;
    }

    // Number of reallocations needed based on NCNRewardRouter::SIZE
    let num_reallocs = (NCNRewardRouter::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1;

    let (config, _, _) = NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn);

    // Realloc reward router
    let realloc_ncn_reward_router_ix = ReallocNCNRewardRouterBuilder::new()
        .ncn_reward_router(ncn_reward_router)
        .config(config)
        .ncn(ncn)
        .epoch_state(epoch_state)
        .epoch(epoch)
        .account_payer(account_payer)
        .system_program(system_program::id())
        .instruction();

    let mut realloc_ixs = Vec::with_capacity(num_reallocs as usize);
    realloc_ixs.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000));
    for _ in 0..num_reallocs {
        realloc_ixs.push(realloc_ncn_reward_router_ix.clone());
    }

    send_and_log_transaction(
        handler,
        &realloc_ixs,
        &[],
        "Reallocated NCN Reward Router",
        &[
            format!("NCN: {:?}", ncn),
            format!("Epoch: {:?}", epoch),
            format!("Number of reallocations: {:?}", num_reallocs),
        ],
    )
    .await?;

    Ok(())
}

pub async fn create_operator_vault_reward_router(
    handler: &CliHandler,
    operator: &Pubkey,
    epoch: u64,
) -> Result<()> {
    let ncn = *handler.ncn()?;

    let operator = *operator;

    let (operator_snapshot, _, _) =
        OperatorSnapshot::find_program_address(&handler.ncn_program_id, &operator, &ncn, epoch);

    let (operator_vault_reward_router, _, _) = OperatorVaultRewardRouter::find_program_address(
        &handler.ncn_program_id,
        &operator,
        &ncn,
        epoch,
    );

    let (account_payer, _, _) = AccountPayer::find_program_address(&handler.ncn_program_id, &ncn);
    let (epoch_marker, _, _) = EpochMarker::find_program_address(&ncn_program::id(), &ncn, epoch);

    let operator_vault_reward_router_account =
        get_account(handler, &operator_vault_reward_router).await?;

    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (operator_vault_reward_receiver, _, _) = OperatorVaultRewardReceiver::find_program_address(
        &handler.ncn_program_id,
        &operator,
        &ncn,
        epoch,
    );

    // Skip if reward router already exists
    if operator_vault_reward_router_account.is_none() {
        // Initialize reward router
        let initialize_operator_vault_reward_router_ix =
            InitializeOperatorVaultRewardRouterBuilder::new()
                .epoch_marker(epoch_marker)
                .epoch_state(epoch_state)
                .operator_snapshot(operator_snapshot)
                .operator_vault_reward_router(operator_vault_reward_router)
                .operator_vault_reward_receiver(operator_vault_reward_receiver)
                .ncn(ncn)
                .operator(operator)
                .epoch(epoch)
                .account_payer(account_payer)
                .system_program(system_program::id())
                .instruction();

        send_and_log_transaction(
            handler,
            &[initialize_operator_vault_reward_router_ix],
            &[],
            "Initialized Operator Vault Reward Router",
            &[
                format!("NCN: {:?}", ncn),
                format!("Operator: {:?}", operator),
                format!("Epoch: {:?}", epoch),
            ],
        )
        .await?;
    }

    Ok(())
}

pub async fn route_ncn_rewards(handler: &CliHandler, epoch: u64) -> Result<()> {
    let mut still_routing = true;

    while still_routing {
        process_route_ncn_rewards(handler, epoch).await?;

        let ncn_reward_router_account = get_ncn_reward_router(handler, epoch).await?;

        still_routing = ncn_reward_router_account.still_routing();
    }

    Ok(())
}

pub async fn process_route_ncn_rewards(handler: &CliHandler, epoch: u64) -> Result<()> {
    let ncn = *handler.ncn()?;

    let (config, _, _) = NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn);

    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (ncn_reward_router, _, _) =
        NCNRewardRouter::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (ncn_reward_receiver, _, _) =
        NCNRewardReceiver::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (ballot_box, _, _) = BallotBox::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (epoch_snapshot, _, _) =
        EpochSnapshot::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let route_ncn_rewards_ix = RouteNCNRewardsBuilder::new()
        .epoch_state(epoch_state)
        .config(config)
        .ncn(ncn)
        .epoch_snapshot(epoch_snapshot)
        .ballot_box(ballot_box)
        .ncn_reward_router(ncn_reward_router)
        .ncn_reward_receiver(ncn_reward_receiver)
        .max_iterations(NCNRewardRouter::MAX_ROUTE_BASE_ITERATIONS)
        .epoch(epoch)
        .instruction();

    let cul_ix = ComputeBudgetInstruction::set_compute_unit_limit(1_400_000);

    send_and_log_transaction(
        handler,
        &[cul_ix, route_ncn_rewards_ix],
        &[],
        "Routed NCN Rewards",
        &[format!("NCN: {:?}", ncn), format!("Epoch: {:?}", epoch)],
    )
    .await?;

    Ok(())
}

pub async fn distribute_ncn_rewards(handler: &CliHandler, epoch: u64) -> Result<()> {
    let ncn = *handler.ncn()?;

    let (ncn_config_address, _, _) =
        NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn);

    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (ncn_reward_router, _, _) =
        NCNRewardRouter::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (ncn_reward_receiver, _, _) =
        NCNRewardReceiver::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let ncn_config = get_ncn_program_config(handler).await?;

    let distribute_ncn_rewards_ix = DistributeNCNRewardsBuilder::new()
        .epoch_state(epoch_state)
        .config(ncn_config_address)
        .ncn(ncn)
        .ncn_reward_router(ncn_reward_router)
        .ncn_reward_receiver(ncn_reward_receiver)
        .ncn_fee_wallet(*ncn_config.fee_config.ncn_fee_wallet())
        .epoch(epoch)
        .instruction();

    send_and_log_transaction(
        handler,
        &[distribute_ncn_rewards_ix],
        &[],
        "Distributed NCN Rewards",
        &[format!("NCN: {:?}", ncn), format!("Epoch: {:?}", epoch)],
    )
    .await?;

    Ok(())
}

pub async fn distribute_protocol_rewards(handler: &CliHandler, epoch: u64) -> Result<()> {
    let ncn = *handler.ncn()?;

    let (ncn_config_address, _, _) =
        NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn);

    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (ncn_reward_router, _, _) =
        NCNRewardRouter::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (ncn_reward_receiver, _, _) =
        NCNRewardReceiver::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let ncn_config = get_ncn_program_config(handler).await?;

    let distribute_protocol_rewards_ix = DistributeProtocolRewardsBuilder::new()
        .epoch_state(epoch_state)
        .config(ncn_config_address)
        .ncn(ncn)
        .ncn_reward_router(ncn_reward_router)
        .ncn_reward_receiver(ncn_reward_receiver)
        .protocol_fee_wallet(*ncn_config.fee_config.protocol_fee_wallet())
        .epoch(epoch)
        .system_program(system_program::id())
        .instruction();

    send_and_log_transaction(
        handler,
        &[distribute_protocol_rewards_ix],
        &[],
        "Distributed Protocol Rewards",
        &[format!("NCN: {:?}", ncn), format!("Epoch: {:?}", epoch)],
    )
    .await?;

    Ok(())
}

pub async fn route_operator_vault_rewards(
    handler: &CliHandler,
    operator: &Pubkey,
    epoch: u64,
) -> Result<()> {
    let mut still_routing = true;

    while still_routing {
        process_route_operator_vault_rewards(handler, operator, epoch).await?;

        let ncn_reward_router_account =
            get_operator_vault_reward_router(handler, operator, epoch).await?;

        still_routing = ncn_reward_router_account.still_routing();
    }

    Ok(())
}

pub async fn process_route_operator_vault_rewards(
    handler: &CliHandler,
    operator: &Pubkey,
    epoch: u64,
) -> Result<()> {
    let ncn = *handler.ncn()?;

    let operator = *operator;

    let (operator_snapshot, _, _) =
        OperatorSnapshot::find_program_address(&handler.ncn_program_id, &operator, &ncn, epoch);

    let (operator_vault_reward_router, _, _) = OperatorVaultRewardRouter::find_program_address(
        &handler.ncn_program_id,
        &operator,
        &ncn,
        epoch,
    );

    let (operator_vault_reward_receiver, _, _) = OperatorVaultRewardReceiver::find_program_address(
        &handler.ncn_program_id,
        &operator,
        &ncn,
        epoch,
    );

    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let route_operator_vault_rewards_ix = RouteOperatorVaultRewardsBuilder::new()
        .ncn(ncn)
        .epoch_state(epoch_state)
        .operator(operator)
        .operator_snapshot(operator_snapshot)
        .operator_vault_reward_router(operator_vault_reward_router)
        .operator_vault_reward_receiver(operator_vault_reward_receiver)
        .epoch(epoch)
        .max_iterations(OperatorVaultRewardRouter::MAX_ROUTE_NCN_ITERATIONS)
        .instruction();

    let cul_ix = ComputeBudgetInstruction::set_compute_unit_limit(1_400_000);

    send_and_log_transaction(
        handler,
        &[cul_ix, route_operator_vault_rewards_ix],
        &[],
        "Routed Operator Vault Rewards",
        &[
            format!("NCN: {:?}", ncn),
            format!("Operator: {:?}", operator),
            format!("Epoch: {:?}", epoch),
        ],
    )
    .await?;

    Ok(())
}

pub async fn distribute_operator_vault_rewards(
    handler: &CliHandler,
    operator: &Pubkey,
    epoch: u64,
) -> Result<()> {
    let ncn = *handler.ncn()?;

    let operator = *operator;

    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (config, _, _) = NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn);

    let (ncn_reward_router, _, _) =
        NCNRewardRouter::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (ncn_reward_receiver, _, _) =
        NCNRewardReceiver::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (operator_vault_reward_router, _, _) = OperatorVaultRewardRouter::find_program_address(
        &handler.ncn_program_id,
        &operator,
        &ncn,
        epoch,
    );

    let (operator_vault_reward_receiver, _, _) = OperatorVaultRewardReceiver::find_program_address(
        &handler.ncn_program_id,
        &operator,
        &ncn,
        epoch,
    );

    let distribute_operator_vault_reward_route_ix =
        DistributeOperatorVaultRewardRouteBuilder::new()
            .epoch_state(epoch_state)
            .ncn(ncn)
            .operator(operator)
            .config(config)
            .ncn_reward_router(ncn_reward_router)
            .ncn_reward_receiver(ncn_reward_receiver)
            .operator_vault_reward_router(operator_vault_reward_router)
            .operator_vault_reward_receiver(operator_vault_reward_receiver)
            .system_program(system_program::id())
            .epoch(epoch)
            .instruction();

    send_and_log_transaction(
        handler,
        &[distribute_operator_vault_reward_route_ix],
        &[],
        "Distributed Operator Vault Reward Route",
        &[
            format!("NCN: {:?}", ncn),
            format!("Operator: {:?}", operator),
            format!("Epoch: {:?}", epoch),
        ],
    )
    .await?;

    Ok(())
}

pub async fn distribute_ncn_operator_rewards(
    handler: &CliHandler,
    operator: &Pubkey,
    epoch: u64,
) -> Result<()> {
    let ncn = *handler.ncn()?;

    let operator = *operator;

    let (config, _, _) = NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn);

    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (operator_snapshot, _, _) =
        OperatorSnapshot::find_program_address(&handler.ncn_program_id, &operator, &ncn, epoch);

    let (operator_vault_reward_router, _, _) = OperatorVaultRewardRouter::find_program_address(
        &handler.ncn_program_id,
        &operator,
        &ncn,
        epoch,
    );

    let (operator_vault_reward_receiver, _, _) = OperatorVaultRewardReceiver::find_program_address(
        &handler.ncn_program_id,
        &operator,
        &ncn,
        epoch,
    );

    let distribute_operator_rewards_ix = DistributeOperatorRewardsBuilder::new()
        .epoch_state(epoch_state)
        .config(config)
        .ncn(ncn)
        .operator(operator)
        .operator_snapshot(operator_snapshot)
        .operator_vault_reward_router(operator_vault_reward_router)
        .operator_vault_reward_receiver(operator_vault_reward_receiver)
        .system_program(system_program::id())
        .epoch(epoch)
        .instruction();

    send_and_log_transaction(
        handler,
        &[distribute_operator_rewards_ix],
        &[],
        "Distributed Operator Rewards",
        &[
            format!("NCN: {:?}", ncn),
            format!("Operator: {:?}", operator),
            format!("Epoch: {:?}", epoch),
        ],
    )
    .await?;

    Ok(())
}

pub async fn distribute_ncn_vault_rewards(
    handler: &CliHandler,
    vault: &Pubkey,
    operator: &Pubkey,
    epoch: u64,
) -> Result<()> {
    let ncn = *handler.ncn()?;

    let vault = *vault;
    let operator = *operator;

    let (config, _, _) = NCNProgramConfig::find_program_address(&handler.ncn_program_id, &ncn);

    let (epoch_state, _, _) =
        EpochState::find_program_address(&handler.ncn_program_id, &ncn, epoch);

    let (operator_snapshot, _, _) =
        OperatorSnapshot::find_program_address(&handler.ncn_program_id, &operator, &ncn, epoch);

    let (operator_vault_reward_router, _, _) = OperatorVaultRewardRouter::find_program_address(
        &handler.ncn_program_id,
        &operator,
        &ncn,
        epoch,
    );

    let (operator_vault_reward_receiver, _, _) = OperatorVaultRewardReceiver::find_program_address(
        &handler.ncn_program_id,
        &operator,
        &ncn,
        epoch,
    );

    let distribute_vault_rewards_ix = DistributeVaultRewardsBuilder::new()
        .epoch_state(epoch_state)
        .config(config)
        .ncn(ncn)
        .operator(operator)
        .vault(vault)
        .operator_snapshot(operator_snapshot)
        .operator_vault_reward_router(operator_vault_reward_router)
        .operator_vault_reward_receiver(operator_vault_reward_receiver)
        .epoch(epoch)
        .system_program(system_program::id())
        .instruction();

    send_and_log_transaction(
        handler,
        &[distribute_vault_rewards_ix],
        &[],
        "Distributed Vault Rewards",
        &[
            format!("NCN: {:?}", ncn),
            format!("Vault: {:?}", vault),
            format!("Operator: {:?}", operator),
            format!("Epoch: {:?}", epoch),
        ],
    )
    .await?;

    Ok(())
}

// --------------------- HELPERS -------------------------

pub async fn send_and_log_transaction(
    handler: &CliHandler,
    instructions: &[Instruction],
    signing_keypairs: &[&Keypair],
    title: &str,
    log_items: &[String],
) -> Result<()> {
    sleep(Duration::from_secs(1)).await;

    let signature = send_transactions(handler, instructions, signing_keypairs).await?;

    log_transaction(title, signature, log_items);

    Ok(())
}

pub async fn send_transactions(
    handler: &CliHandler,
    instructions: &[Instruction],
    signing_keypairs: &[&Keypair],
) -> Result<Signature> {
    let client = handler.rpc_client();
    let keypair = handler.keypair()?;
    let retries = handler.retries;
    let priority_fee_micro_lamports = handler.priority_fee_micro_lamports;

    let mut all_instructions = vec![];

    all_instructions.push(ComputeBudgetInstruction::set_compute_unit_price(
        priority_fee_micro_lamports,
    ));

    all_instructions.extend_from_slice(instructions);

    for iteration in 0..retries {
        let blockhash = client.get_latest_blockhash().await?;

        // Create a vector that combines all signing keypairs
        let mut all_signers = vec![keypair];
        all_signers.extend(signing_keypairs.iter());

        let tx = Transaction::new_signed_with_payer(
            &all_instructions,
            Some(&keypair.pubkey()),
            &all_signers, // Pass the reference to the vector of keypair references
            blockhash,
        );

        let config = RpcSendTransactionConfig {
            skip_preflight: true,
            ..RpcSendTransactionConfig::default()
        };
        let result = client
            .send_and_confirm_transaction_with_spinner_and_config(&tx, client.commitment(), config)
            .await;

        if result.is_err() {
            info!(
                "Retrying transaction after {}s {}/{}",
                (1 + iteration),
                iteration,
                retries
            );

            boring_progress_bar((1 + iteration) * 1000).await;
            continue;
        }

        return Ok(result?);
    }

    // last retry
    let blockhash = client.get_latest_blockhash().await?;

    // Create a vector that combines all signing keypairs
    let mut all_signers = vec![keypair];
    all_signers.extend(signing_keypairs.iter());

    let tx = Transaction::new_signed_with_payer(
        instructions,
        Some(&keypair.pubkey()),
        &all_signers, // Pass the reference to the vector of keypair references
        blockhash,
    );

    let result = client.send_and_confirm_transaction(&tx).await;

    if let Err(e) = result {
        return Err(anyhow!("\nError: \n\n{:?}\n\n", e));
    }

    Ok(result?)
}

pub fn log_transaction(title: &str, signature: Signature, log_items: &[String]) {
    let mut log_message = format!(
        "\n\n---------- {} ----------\nSignature: {:?}",
        title, signature
    );

    for item in log_items {
        log_message.push_str(&format!("\n{}", item));
    }

    // msg!(log_message.clone());

    log_message.push('\n');
    info!("{}", log_message);
}
