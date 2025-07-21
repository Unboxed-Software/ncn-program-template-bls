use anyhow::Result;
// Ballot functionality has been removed
use solana_metrics::datapoint_info;
use solana_sdk::pubkey::Pubkey;

use crate::{getters::get_current_epoch_and_slot, handler::CliHandler};

/// Macro for emitting epoch-specific metrics
///
/// Emits metrics with two variants:
/// 1. Standard metric with the given name
/// 2. If this is the current epoch, also emits a "-current" suffixed metric
///
/// This allows tracking both historical and current epoch metrics separately
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

/// Emits error metrics for tracking operator failures
///
/// # Arguments
/// * `title` - The title/name of the operation that failed
/// * `error` - The error string
/// * `message` - Detailed error message
/// * `keeper_epoch` - The epoch in which the error occurred
pub async fn emit_error(title: String, error: String, message: String, keeper_epoch: u64) {
    datapoint_info!(
        "ncn-operator-keeper-error",
        ("command-title", title, String),
        ("error", error, String),
        ("message", message, String),
        ("keeper-epoch", keeper_epoch, i64),
    );
}

/// Emits heartbeat metrics to indicate the operator is alive
///
/// # Arguments
/// * `tick` - Counter representing the number of heartbeats
pub async fn emit_heartbeat(tick: u64) {
    datapoint_info!(
        "ncn-operator-keeper-keeper-heartbeat-operations",
        ("tick", tick, i64),
    );

    datapoint_info!(
        "ncn-operator-keeper-keeper-heartbeat-metrics",
        ("tick", tick, i64),
    );
}

/// Emits metrics when an operator submits a vote
///
/// # Arguments
/// * `handler` - CLI handler for RPC communication
/// * `vote` - The vote value submitted (weather status code)
/// * `epoch` - The epoch being voted on
/// * `operator` - The public key of the operator casting the vote
///
/// # Returns
/// * Result indicating success or failure
pub async fn emit_ncn_metrics_operator_vote(
    handler: &CliHandler,
    vote: u8,
    epoch: u64,
    operator: &Pubkey,
) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;

    let is_current_epoch = current_epoch == epoch;
    emit_epoch_datapoint!(
        "ncn-operator-keeper-operator-vote",
        is_current_epoch,
        ("current-epoch", current_epoch, i64),
        ("current-slot", current_slot, i64),
        ("keeper-epoch", epoch, i64),
        ("operator", operator.to_string(), String),
        ("vote", vote as i64, i64)
    );

    Ok(())
}

/// Emits comprehensive metrics after an operator has voted
///
/// Collects and reports detailed information about:
/// - The operator's vote status
/// - Vote weights
///
/// # Arguments
/// * `handler` - CLI handler for RPC communication
/// * `epoch` - The epoch being tracked
/// * `operator` - The public key of the operator
///
/// # Returns
/// * Result indicating success or failure
pub async fn emit_ncn_metrics_operator_post_vote(
    handler: &CliHandler,
    epoch: u64,
    operator: &Pubkey,
) -> Result<()> {
    Ok(())
}
