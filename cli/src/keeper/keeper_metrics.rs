use anyhow::Result;
use ncn_program_core::account_payer::AccountPayer;
use solana_metrics::datapoint_info;
use solana_sdk::{clock::DEFAULT_SLOTS_PER_EPOCH, native_token::lamports_to_sol};

use crate::{
    getters::{
        get_account_payer, get_all_operators_in_ncn, get_all_tickets, get_all_vaults_in_ncn,
        get_current_epoch_and_slot, get_ncn_program_config, get_operator, get_operator_snapshot,
        get_snapshot, get_vault, get_vault_config, get_vault_operator_delegation,
        get_vault_registry,
    },
    handler::CliHandler,
};

/// Formats stake weight values for metrics (converts u128 to f64)
///
/// Stake weights are stored as large integers but are more readable
/// as floating point values in metrics dashboards.
pub const fn format_stake_weight(value: u128) -> f64 {
    value as f64
}

/// Formats token amounts from lamports to SOL for metrics
///
/// Converts raw lamport values to human-readable SOL amounts
/// for better understanding in monitoring dashboards.
pub fn format_token_amount(value: u64) -> f64 {
    lamports_to_sol(value)
}

/// Emits error metrics for monitoring and alerting
///
/// This function standardizes error reporting across the keeper,
/// ensuring all errors are captured with consistent metadata for
/// monitoring, alerting, and debugging purposes.
///
/// # Arguments
/// * `title` - A descriptive title for the operation that failed
/// * `error` - The error message or description
/// * `message` - A formatted message with additional context
/// * `keeper_epoch` - The epoch being processed when the error occurred
pub async fn emit_error(title: String, error: String, message: String, keeper_epoch: u64) {
    datapoint_info!(
        "ncn-program-keeper-error",
        ("command-title", title, String),
        ("error", error, String),
        ("message", message, String),
        ("keeper-epoch", keeper_epoch, i64),
    );
}

/// Emits heartbeat metrics to indicate the keeper is alive and operational
///
/// Heartbeats are essential for monitoring system health and detecting
/// when the keeper has stopped or is experiencing issues.
///
/// # Arguments
/// * `tick` - A monotonically increasing counter indicating keeper activity
pub async fn emit_heartbeat(tick: u64) {
    datapoint_info!(
        "ncn-program-keeper-keeper-heartbeat-operations",
        ("tick", tick, i64),
    );

    datapoint_info!(
        "ncn-program-keeper-keeper-heartbeat-metrics",
        ("tick", tick, i64)
    );
}

/// Main entry point for emitting NCN (Network Coordinated Node) metrics
///
/// This function orchestrates the emission of various NCN-level metrics,
/// some of which are emitted only at the start of each loop to avoid
/// excessive data volume while maintaining adequate monitoring coverage.
///
/// # Arguments
/// * `handler` - CLI handler for blockchain interactions
/// * `start_of_loop` - Whether this is the first epoch in the processing loop
#[allow(clippy::large_stack_frames)]
pub async fn emit_ncn_metrics(handler: &CliHandler, start_of_loop: bool) -> Result<()> {
    // Always emit current epoch and slot information
    emit_ncn_metrics_epoch_slot(handler).await?;

    // Emit detailed metrics only at the start of the loop to manage data volume
    if start_of_loop {
        emit_ncn_metrics_tickets(handler).await?;
        emit_ncn_metrics_vault_operator_delegation(handler).await?;
        emit_ncn_metrics_operators(handler).await?;
        emit_ncn_metrics_vault_registry(handler).await?;
        emit_ncn_metrics_config(handler).await?;
        emit_ncn_metrics_account_payer(handler).await?;
    }

    Ok(())
}

/// Emits current epoch and slot metrics
///
/// Tracks the blockchain's current epoch and slot, along with the
/// percentage progress through the current epoch. This is fundamental
/// timing information for understanding the keeper's context.
pub async fn emit_ncn_metrics_epoch_slot(handler: &CliHandler) -> Result<()> {
    let ncn = handler.ncn()?;
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;
    let epoch_percentage =
        (current_slot as f64 % DEFAULT_SLOTS_PER_EPOCH as f64) / DEFAULT_SLOTS_PER_EPOCH as f64;

    datapoint_info!(
        "ncn-program-keeper-em-epoch-slot",
        ("current-epoch", current_epoch, i64),
        ("current-slot", current_slot, i64),
        ("epoch-percentage", epoch_percentage, f64),
        ("ncn", ncn.to_string(), String),
    );

    Ok(())
}

/// Emits account payer metrics
///
/// The account payer is responsible for funding transaction fees and
/// account rent. Monitoring its balance is crucial for ensuring the
/// keeper can continue operating.
pub async fn emit_ncn_metrics_account_payer(handler: &CliHandler) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;

    let (account_payer_address, _, _) =
        AccountPayer::find_program_address(&handler.ncn_program_id, handler.ncn()?);
    let account_payer = get_account_payer(handler).await?;

    datapoint_info!(
        "ncn-program-keeper-em-account-payer",
        ("current-epoch", current_epoch, i64),
        ("current-slot", current_slot, i64),
        ("account-payer", account_payer_address.to_string(), String),
        ("balance", account_payer.lamports, i64),
        ("balance-sol", lamports_to_sol(account_payer.lamports), f64),
    );

    Ok(())
}

/// Emits comprehensive ticket metrics
///
/// Tickets represent the relationship between NCNs, operators, and vaults.
/// This function emits detailed metrics about each ticket including:
/// - Delegation amounts (staked, cooling down, total security)
/// - Vault state and token information
/// - Relationship counts and statuses
///
/// This provides visibility into the staking and delegation ecosystem.
pub async fn emit_ncn_metrics_tickets(handler: &CliHandler) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;
    let vault_epoch_length = {
        let vault_config = get_vault_config(handler).await?;
        vault_config.epoch_length()
    };
    let all_tickets = get_all_tickets(handler).await?;

    for ticket in all_tickets {
        let (staked_amount, cooling_down_amount, total_security) = ticket.delegation();
        let vault_delegation_state = ticket.vault_account.delegation_state;

        datapoint_info!(
            "ncn-program-keeper-em-ticket",
            ("current-epoch", current_epoch, i64),
            ("current-slot", current_slot, i64),
            ("operator", ticket.operator.to_string(), String),
            ("vault", ticket.vault.to_string(), String),
            (
                "ticket-id",
                format!(
                    "{}-{}-{}",
                    ticket.ncn.to_string(),
                    ticket.vault.to_string(),
                    ticket.operator.to_string()
                ),
                String
            ),
            // Relationship indices for data analysis
            ("ncn-vault", ticket.ncn_vault(), i64),
            ("vault-ncn", ticket.vault_ncn(), i64),
            ("ncn-operator", ticket.ncn_operator(), i64),
            ("operator-ncn", ticket.operator_ncn(), i64),
            ("operator-vault", ticket.operator_vault(), i64),
            ("vault-operator", ticket.vault_operator(), i64),
            // Delegation amounts
            ("vod-staked-amount", format_token_amount(staked_amount), f64),
            (
                "vod-cooling-down-amount",
                format_token_amount(cooling_down_amount),
                f64
            ),
            (
                "vod-total-security",
                format_token_amount(total_security),
                f64
            ),
            // Vault information
            (
                "vault-st-mint",
                ticket.vault_account.supported_mint.to_string(),
                String
            ),
            (
                "vault-tokens-deposited",
                format_token_amount(ticket.vault_account.tokens_deposited()),
                f64
            ),
            ("vault-vrt-supply", ticket.vault_account.vrt_supply(), i64),
            (
                "vault-vrt-cooling-down-amount",
                format_token_amount(ticket.vault_account.vrt_cooling_down_amount()),
                f64
            ),
            (
                "vault-vrt-enqueued-for-cooldown-amount",
                format_token_amount(ticket.vault_account.vrt_enqueued_for_cooldown_amount()),
                f64
            ),
            (
                "vault-vrt-ready-to-claim-amount",
                format_token_amount(ticket.vault_account.vrt_ready_to_claim_amount()),
                f64
            ),
            (
                "vault-is-update-needed",
                ticket
                    .vault_account
                    .is_update_needed(current_slot, vault_epoch_length)?,
                bool
            ),
            (
                "vault-operator-count",
                ticket.vault_account.operator_count(),
                i64
            ),
            ("vault-ncn-count", ticket.vault_account.ncn_count(), i64),
            ("vault-config-epoch-length", vault_epoch_length, i64),
            // Vault total delegation state
            (
                "vault-total-staked-amount",
                format_token_amount(vault_delegation_state.staked_amount()),
                f64
            ),
            (
                "vod-total-cooling-down-amount",
                format_token_amount(vault_delegation_state.cooling_down_amount()),
                f64
            ),
            (
                "vod-total-total-security",
                format_token_amount(vault_delegation_state.total_security()?),
                f64
            ),
        );
    }

    Ok(())
}

/// Emits vault-operator delegation metrics
///
/// This tracks the delegation relationship between each vault and operator,
/// providing visibility into how stake is distributed across the network.
pub async fn emit_ncn_metrics_vault_operator_delegation(handler: &CliHandler) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;
    let all_operators = get_all_operators_in_ncn(handler).await?;
    let all_vaults = get_all_vaults_in_ncn(handler).await?;

    for operator in all_operators.iter() {
        for vault in all_vaults.iter() {
            let result = get_vault_operator_delegation(handler, vault, operator).await;

            if result.is_err() {
                continue;
            }
            let vault_operator_delegation = result?;

            datapoint_info!(
                "ncn-program-keeper-em-vault-operator-delegation",
                ("current-epoch", current_epoch, i64),
                ("current-slot", current_slot, i64),
                ("vault", vault.to_string(), String),
                ("operator", operator.to_string(), String),
                (
                    "delegation",
                    format_token_amount(
                        vault_operator_delegation
                            .delegation_state
                            .total_security()?
                    ),
                    f64
                ),
            );
        }
    }

    Ok(())
}

/// Emits operator metrics including voting status
///
/// Tracks each operator's configuration and participation in the network,
/// including fees, relationship counts, and whether they've voted in the
/// current epoch.
pub async fn emit_ncn_metrics_operators(handler: &CliHandler) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;
    let all_operators = get_all_operators_in_ncn(handler).await?;
    for operator in all_operators {
        let operator_account = get_operator(handler, &operator).await?;

        // Voting functionality has been removed
        let operator_has_voted = false;

        datapoint_info!(
            "ncn-program-keeper-em-operator",
            ("current-epoch", current_epoch, i64),
            ("current-slot", current_slot, i64),
            ("operator", operator.to_string(), String),
            (
                "fee",
                Into::<u16>::into(operator_account.operator_fee_bps) as i64,
                i64
            ),
            ("ncn-count", operator_account.ncn_count(), i64),
            ("has-voted", operator_has_voted as i64, i64)
        );
    }

    Ok(())
}

/// Emits vault registry metrics
///
/// The vault registry tracks all vaults and supported tokens in the system.
/// This function emits metrics about the registry state and detailed
/// information about each registered vault and supported token.
pub async fn emit_ncn_metrics_vault_registry(handler: &CliHandler) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;
    let vault_registry = get_vault_registry(handler).await?;

    // Overall registry statistics
    datapoint_info!(
        "ncn-program-keeper-em-vault-registry",
        ("current-epoch", current_epoch, i64),
        ("current-slot", current_slot, i64),
        ("st-mints", vault_registry.st_mint_count(), i64),
    );

    // Individual vault metrics
    for vault in vault_registry.vault_list {
        if vault.is_empty() {
            continue;
        }

        let vault_account = get_vault(handler, vault.vault()).await?;

        datapoint_info!(
            "ncn-program-keeper-em-vault-registry-vault",
            ("current-epoch", current_epoch, i64),
            ("current-slot", current_slot, i64),
            ("vault", vault.vault().to_string(), String),
            ("st-mint", vault.st_mint().to_string(), String),
            ("index", vault.vault_index(), i64),
            (
                "tokens-deposited",
                format_token_amount(vault_account.tokens_deposited()),
                f64
            ),
            (
                "vrt-supply",
                format_token_amount(vault_account.vrt_supply()),
                f64
            ),
            ("operator-count", vault_account.operator_count(), i64),
            ("ncn-count", vault_account.ncn_count(), i64),
        );
    }

    // Supported token (st_mint) metrics
    for st_mint in vault_registry.st_mint_list {
        datapoint_info!(
            "ncn-program-keeper-em-vault-registry-st-mint",
            ("current-epoch", current_epoch, i64),
            ("current-slot", current_slot, i64),
            ("st-mint", st_mint.st_mint().to_string(), String),
        );
    }

    Ok(())
}

/// Emits NCN program configuration metrics
///
/// Tracks the current configuration parameters that affect epoch timing,
/// consensus requirements, and other critical system behaviors.
pub async fn emit_ncn_metrics_config(handler: &CliHandler) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;

    let config = get_ncn_program_config(handler).await?;

    datapoint_info!(
        "ncn-program-keeper-em-config",
        ("current-epoch", current_epoch, i64),
        ("current-slot", current_slot, i64),
        (
            "epochs-after-consensus-before-close",
            config.epochs_after_consensus_before_close(),
            i64
        ),
        ("epochs-before-stall", config.epochs_before_stall(), i64),
        ("starting-valid-epoch", config.starting_valid_epoch(), i64),
        (
            "valid-slots-after-consensus",
            config.valid_slots_after_consensus(),
            i64
        ),
        (
            "tie-breaker-admin",
            config.tie_breaker_admin.to_string(),
            String
        ),
    );

    Ok(())
}

/// Macro to emit epoch metrics with optional "-current" suffix
///
/// This macro allows the same metric to be emitted twice:
/// 1. With the standard name for historical tracking
/// 2. With a "-current" suffix when it's the current epoch for real-time monitoring
///
/// This pattern enables both historical analysis and current-state alerting.
macro_rules! emit_epoch_datapoint {
    ($name:expr, $is_current_epoch:expr, $($fields:tt),*) => {
        // Always emit the standard metric
        datapoint_info!($name, $($fields),*);

        // If it's the current epoch, also emit with "-current" suffix
        if $is_current_epoch {
            datapoint_info!(
                concat!($name, "-current"),
                $($fields),*
            );
        }
    };
}

/// Main entry point for emitting epoch-specific metrics
///
/// This function orchestrates the emission of all metrics related to a
/// specific epoch's state and progress through the consensus process.
#[allow(clippy::large_stack_frames)]
pub async fn emit_epoch_metrics(handler: &CliHandler, epoch: u64) -> Result<()> {
    emit_epoch_metrics_state(handler, epoch).await?;
    emit_epoch_metrics_snapshot(handler, epoch).await?;
    emit_epoch_metrics_operator_snapshot(handler, epoch).await?;
    emit_epoch_metrics_ballot_box(handler, epoch).await?;

    Ok(())
}

/// Emits ballot box metrics showing voting progress and results
///
/// The ballot box tracks operator votes and consensus outcomes. This function
/// emits detailed metrics about individual votes, ballot tallies, and the
/// overall voting state for the epoch.
#[allow(clippy::large_stack_frames)]
pub async fn emit_epoch_metrics_ballot_box(handler: &CliHandler, epoch: u64) -> Result<()> {
    // Ballot box functionality has been removed
    log::info!("Ballot box metrics not available - functionality has been removed");
    Ok(())
}

/// Emits operator snapshot metrics for each operator in the epoch
///
/// Operator snapshots capture the state of each operator at the time of
/// epoch creation, including their stake weights, delegation counts, and
/// other relevant information.
pub async fn emit_epoch_metrics_operator_snapshot(handler: &CliHandler, epoch: u64) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;
    let is_current_epoch = current_epoch == epoch;

    let all_operators = get_all_operators_in_ncn(handler).await?;

    for operator in all_operators.iter() {
        let result = get_operator_snapshot(handler, operator, epoch).await;

        if let Ok(operator_snapshot) = result {
            emit_epoch_datapoint!(
                "ncn-program-keeper-ee-operator-snapshot",
                is_current_epoch,
                ("current-epoch", current_epoch, i64),
                ("current-slot", current_slot, i64),
                ("keeper-epoch", epoch, i64),
                ("operator", operator.to_string(), String),
                ("is-active", operator_snapshot.is_active(), bool),
                (
                    "ncn-operator-index",
                    operator_snapshot.ncn_operator_index(),
                    i64
                ),
                (
                    "operator-fee-bps",
                    0, // operator_fee_bps not available in this version
                    i64
                ),
                (
                    "stake",
                    format_stake_weight(operator_snapshot.stake_weight().stake_weight()),
                    f64
                )
            );
        }
    }

    Ok(())
}

/// Emits snapshot metrics showing overall epoch state
///
/// The snapshot provides a high-level view of the epoch including
/// total stake weights, operator counts, and other aggregate statistics.
pub async fn emit_epoch_metrics_snapshot(handler: &CliHandler, epoch: u64) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;
    let is_current_epoch = current_epoch == epoch;

    let result = get_snapshot(handler, epoch).await;

    if let Ok(snapshot) = result {
        emit_epoch_datapoint!(
            "ncn-program-keeper-ee-epoch-snapshot",
            is_current_epoch,
            ("current-epoch", current_epoch, i64),
            ("current-slot", current_slot, i64),
            ("keeper-epoch", epoch, i64),
            (
                "total-stake",
                format_stake_weight(snapshot.minimum_stake().stake_weight()),
                f64
            ),
            (
                "valid-operator-vault-delegations",
                snapshot.operators_can_vote_count(),
                i64
            ),
            ("operators-registered", snapshot.operators_registered(), i64),
            ("operator-count", snapshot.operator_count(), i64)
        );
    }

    Ok(())
}

/// Emits detailed epoch state metrics showing progress and account statuses
///
/// This function provides comprehensive visibility into the epoch's current
/// state, progress through each phase, and the status of all related accounts.
/// It handles both active epochs and completed epochs differently.
#[allow(clippy::large_stack_frames)]
pub async fn emit_epoch_metrics_state(handler: &CliHandler, epoch: u64) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;
    let is_current_epoch = current_epoch == epoch;

    // TODO: Implement epoch completion check for snapshot-focused keeper
    let is_epoch_completed = false;

    // Handle completed epochs with simplified metrics
    if is_epoch_completed {
        emit_epoch_datapoint!(
            "ncn-program-keeper-ee-state",
            is_current_epoch,
            ("current-epoch", current_epoch, i64),
            ("current-slot", current_slot, i64),
            ("keeper-epoch", epoch, i64),
            ("current-state-string", "Complete", String),
            ("current-state", -1, i64),
            ("is-complete", true, bool)
        );

        return Ok(());
    }

    // Handle active epochs with detailed state information
    // TODO: Implement epoch state retrieval for snapshot-focused keeper
    // For now, return early since we're focusing on snapshot operations
    return Ok(());
}
