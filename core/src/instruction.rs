use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankInstruction;
use solana_program::pubkey::Pubkey;

use crate::config::ConfigAdminRole;

/// Represents all instructions supported by the NCN Program
/// Each instruction specifies the accounts it requires and any parameters
/// The instruction variants are organized into logical sections:
/// - Global: Program initialization and configuration
/// - Snapshot: Creating snapshots of validator and operator state
/// - Vote: Consensus voting mechanism
/// - Admin: Administrative operations
#[rustfmt::skip]
#[derive(Debug, BorshSerialize, BorshDeserialize, ShankInstruction)]
pub enum NCNProgramInstruction {

    // ---------------------------------------------------- //
    //                         GLOBAL                       //
    // ---------------------------------------------------- //
    /// Initialize the config account for the NCN program
    /// Sets up the basic program parameters
    #[account(0, writable, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, name = "ncn_fee_wallet")]
    #[account(3, signer, name = "ncn_admin")]
    #[account(4, name = "tie_breaker_admin")]
    #[account(5, writable, name = "account_payer")]
    #[account(6, name = "system_program")]
    InitializeConfig {
        /// Number of epochs before voting is considered stalled
        epochs_before_stall: u64,
        /// Number of epochs after consensus before accounts can be closed
        epochs_after_consensus_before_close: u64,
        /// Number of slots after consensus where voting is still valid
        valid_slots_after_consensus: u64,
        /// Minimum stake weight for a validator to be considered valid
        minimum_stake_weight: u128,
        /// NCN fee basis points (bps) for the NCN program
        ncn_fee_bps: u16,
    },

    /// Initializes the vault registry account to track validator vaults
    #[account(0, name = "config")]
    #[account(1, writable, name = "vault_registry")]
    #[account(2, name = "ncn")]
    #[account(3, writable, name = "account_payer")]
    #[account(4, name = "system_program")]
    InitializeVaultRegistry,


    /// Registers a vault to the vault registry
    #[account(0, name = "config")]
    #[account(1, writable, name = "vault_registry")]
    #[account(2, name = "ncn")]
    #[account(3, name = "vault")]
    #[account(4, name = "ncn_vault_ticket")]
    RegisterVault,

    /// Initializes the operator registry account to track operators
    #[account(0, name = "config")]
    #[account(1, writable, name = "operator_registry")]
    #[account(2, name = "ncn")]
    #[account(3, writable, name = "account_payer")]
    #[account(4, name = "system_program")]
    InitializeOperatorRegistry,

    /// Registers an operator to the operator registry
    #[account(0, name = "config")]
    #[account(1, writable, name = "operator_registry")]
    #[account(2, name = "ncn")]
    #[account(3, name = "operator")]
    #[account(4, signer, name = "operator_admin")]
    #[account(5, name = "ncn_operator_state")]
    #[account(6, name = "restaking_config")]
    RegisterOperator {
        /// G1 public key (compressed, 32 bytes)
        g1_pubkey: [u8; 32],
        /// G2 public key (compressed, 64 bytes)  
        g2_pubkey: [u8; 64],
        /// BLS signature of G1 pubkey by G2 private key (uncompressed G1 point, 64 bytes)
        signature: [u8; 64],
    },

    /// Updates an operator's BLS keys in the operator registry
    #[account(0, name = "config")]
    #[account(1, writable, name = "operator_registry")]
    #[account(2, name = "ncn")]
    #[account(3, name = "operator")]
    #[account(4, signer, name = "operator_admin")]
    UpdateOperatorBN128Keys {
        /// New G1 public key (compressed, 32 bytes)
        g1_pubkey: [u8; 32],
        /// New G2 public key (compressed, 64 bytes)  
        g2_pubkey: [u8; 64],
        /// BLS signature of the new G1 pubkey signed by the new G2 private key (uncompressed G1 point, 64 bytes)
        signature: [u8; 64],
    },

    /// Resizes the operator registry account
    #[account(0, name = "config")]
    #[account(1, writable, name = "operator_registry")]
    #[account(2, name = "ncn")]
    #[account(3, writable, name = "account_payer")]
    #[account(4, name = "system_program")]
    ReallocOperatorRegistry,

    /// Initializes the vote counter PDA for tracking successful votes
    /// This should be called after InitializeConfig to set up vote tracking
    #[account(0, name = "config")]
    #[account(1, writable, name = "vote_counter")]
    #[account(2, name = "ncn")]
    #[account(3, writable, name = "account_payer")]
    #[account(4, name = "system_program")]
    InitializeVoteCounter,

    // ---------------------------------------------------- //
    //                       SNAPSHOT                       //
    // ---------------------------------------------------- //
    /// Initializes the Epoch State account for a specific epoch
    /// The epoch state tracks the status of an epoch
    #[account(0, name = "epoch_marker")]
    #[account(1, writable, name = "epoch_state")]
    #[account(2, name = "config")]
    #[account(3, name = "ncn")]
    #[account(4, writable, name = "account_payer")]
    #[account(5, name = "system_program")]
    InitializeEpochState {
        /// Target epoch for initialization
        epoch: u64,
    },


    /// Initializes the weight table for a given epoch
    #[account(0, name = "epoch_marker")]
    #[account(1, writable, name = "epoch_state")]
    #[account(2, name = "vault_registry")]
    #[account(3, name = "ncn")]
    #[account(4, writable, name = "weight_table")]
    #[account(5, writable, name = "account_payer")]
    #[account(6, name = "system_program")]
    InitializeWeightTable{
        /// Target epoch for the weight table
        epoch: u64,
    },


    /// Set weights for the weight table using the vault registry
    #[account(0, writable, name = "epoch_state")]
    #[account(1, name = "ncn")]
    #[account(2, name = "vault_registry")]
    #[account(3, writable, name = "weight_table")]
    SetEpochWeights{
        epoch: u64,
    },



    /// Initializes the Epoch Snapshot
    #[account(0, name = "epoch_marker")]
    #[account(1, writable, name = "epoch_state")]
    #[account(2, name = "ncn")]
    #[account(3, writable, name = "epoch_snapshot")]
    #[account(4, writable, name = "account_payer")]
    #[account(5, name = "system_program")]
    InitializeEpochSnapshot{
        epoch: u64,
    },

    /// Reallocates the epoch snapshot account to its full size
    #[account(0, writable, name = "epoch_state")]
    #[account(1, name = "ncn")]
    #[account(2, name = "config")]
    #[account(3, name = "weight_table")]
    #[account(4, writable, name = "epoch_snapshot")]
    #[account(5, writable, name = "account_payer")]
    #[account(6, name = "system_program")]
    ReallocEpochSnapshot {
        epoch: u64,
    },

    /// Initializes the Operator Snapshot within the epoch snapshot
    #[account(0, name = "epoch_marker")]
    #[account(1, writable, name = "epoch_state")]
    #[account(2, name = "restaking_config")]
    #[account(3, name = "ncn")]
    #[account(4, name = "operator")]
    #[account(5, name = "ncn_operator_state")]
    #[account(6, name = "operator_registry")]
    #[account(7, writable, name = "epoch_snapshot")]
    #[account(8, writable, name = "account_payer")]
    #[account(9, name = "system_program")]
    InitializeOperatorSnapshot{
        epoch: u64,
    },
    
    /// Snapshots the vault operator delegation
    #[account(0, writable, name = "epoch_state")]
    #[account(1, name = "config")]
    #[account(2, name = "restaking_config")]
    #[account(3, name = "ncn")]
    #[account(4, name = "operator")]
    #[account(5, name = "vault")]
    #[account(6, name = "vault_ncn_ticket")]
    #[account(7, name = "ncn_vault_ticket")]
    #[account(8, name = "vault_operator_delegation")]
    #[account(9, name = "weight_table")]
    #[account(10, writable, name = "epoch_snapshot")]
    SnapshotVaultOperatorDelegation{
        epoch: u64,
    },

    // ---------------------------------------------------- //
    //                         VOTE                         //
    // ---------------------------------------------------- //
    /// Cast a vote
    #[account(0, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, name = "epoch_snapshot")]
    #[account(3, name = "restaking_config")]
    #[account(4, writable, name = "vote_counter")]
    CastVote {
        aggregated_signature: [u8; 32],
        aggregated_g2: [u8; 64],
        operators_signature_bitmap: Vec<u8>,
    },

    /// Close an epoch account
    #[account(0, writable, name = "epoch_marker")]
    #[account(1, writable, name = "epoch_state")]
    #[account(2, name = "config")]
    #[account(3, name = "ncn")]
    #[account(4, writable, name = "account_to_close")]
    #[account(5, writable, name = "account_payer")]
    #[account(6, name = "system_program")]
    CloseEpochAccount {
        epoch: u64,
    },

    // ---------------------------------------------------- //
    //                        ADMIN                         //
    // ---------------------------------------------------- //
    /// Updates NCN Config parameters
    #[account(0, writable, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, signer, name = "ncn_admin")]
    AdminSetParameters {
        starting_valid_epoch: Option<u64>,
        epochs_before_stall: Option<u64>,
        epochs_after_consensus_before_close: Option<u64>,
        valid_slots_after_consensus: Option<u64>,
        minimum_stake_weight: Option<u128>,
    },


    /// Sets a new secondary admin for the NCN
    #[account(0, writable, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, signer, name = "ncn_admin")]
    #[account(3, name = "new_admin")]
    AdminSetNewAdmin {
        role: ConfigAdminRole,
    },

    /// Sets a weight
    #[account(0, writable, name = "epoch_state")]
    #[account(1, name = "ncn")]
    #[account(2, writable, name = "weight_table")]
    #[account(3, signer, name = "weight_table_admin")]
    AdminSetWeight{
        st_mint: Pubkey,
        weight: u128,
        epoch: u64,
    },

    /// Registers a new ST mint in the Vault Registry
    #[account(0, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, name = "st_mint")]
    #[account(3, writable, name = "vault_registry")]
    #[account(4, signer, writable, name = "admin")]
    AdminRegisterStMint{
        weight: Option<u128>,
    },

    /// Updates an ST mint in the Vault Registry
    #[account(0, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, writable, name = "vault_registry")]
    #[account(3, signer, writable, name = "admin")]
    AdminSetStMint{
        st_mint: Pubkey,
        weight: Option<u128>,
    },
}
