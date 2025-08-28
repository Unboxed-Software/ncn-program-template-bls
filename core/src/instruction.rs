use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankInstruction;

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
        /// Minimum stake for a validator to be considered valid
        minimum_stake: u128,
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

    /// Registers an operator by creating an individual operator PDA
    #[account(0, name = "config")]
    #[account(1, writable, name = "ncn_operator_account")]
    #[account(2, name = "ncn")]
    #[account(3, name = "operator")]
    #[account(4, signer, name = "operator_admin")]
    #[account(5, name = "ncn_operator_state")]
    #[account(6, writable, name = "snapshot")]
    #[account(7, name = "restaking_config")]
    #[account(8, writable, name = "account_payer")]
    #[account(9, name = "system_program")]
    RegisterOperator {
        /// G1 public key (compressed, 32 bytes)
        g1_pubkey: [u8; 32],
        /// G2 public key (compressed, 64 bytes)  
        g2_pubkey: [u8; 64],
        /// BLS signature of G1 pubkey by G2 private key (uncompressed G1 point, 64 bytes)
        signature: [u8; 64],
    },

    /// Updates an operator's BLS keys in their individual operator PDA
    #[account(0, name = "config")]
    #[account(1, writable, name = "ncn_operator_account")]
    #[account(2, name = "ncn")]
    #[account(3, name = "operator")]
    #[account(4, signer, name = "operator_admin")]
    #[account(5, writable, name = "snapshot")]
    UpdateOperatorBN128Keys {
        /// New G1 public key (compressed, 32 bytes)
        g1_pubkey: [u8; 32],
        /// New G2 public key (compressed, 64 bytes)  
        g2_pubkey: [u8; 64],
        /// BLS signature of the new G1 pubkey signed by the new G2 private key (uncompressed G1 point, 64 bytes)
        signature: [u8; 64],
    },

    /// Updates an operator's IP address and socket in their individual operator PDA
    #[account(0, name = "config")]
    #[account(1, writable, name = "ncn_operator_account")]
    #[account(2, name = "ncn")]
    #[account(3, name = "operator")]
    #[account(4, signer, name = "operator_admin")]
    UpdateOperatorIpSocket {
        /// New IP address (IPv4 format, 16 bytes)
        ip_address: [u8; 16],
        /// New socket (16 bytes)
        socket: [u8; 16],
    },

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




    /// Initializes the Snapshot
    #[account(0, name = "ncn")]
    #[account(1, writable, name = "snapshot")]
    #[account(2, writable, name = "account_payer")]
    #[account(3, name = "system_program")]
    InitializeSnapshot{},

    /// Reallocates the snapshot account to its full size
    #[account(0, name = "ncn")]
    #[account(1, name = "config")]
    #[account(2, writable, name = "snapshot")]
    #[account(3, writable, name = "account_payer")]
    #[account(4, name = "system_program")]
    ReallocSnapshot {},

    /// Snapshots the vault operator delegation
    #[account(0, name = "config")]
    #[account(1, name = "restaking_config")]
    #[account(2, name = "ncn")]
    #[account(3, name = "operator")]
    #[account(4, name = "vault")]
    #[account(5, name = "vault_ncn_ticket")]
    #[account(6, name = "ncn_vault_ticket")]
    #[account(7, name = "ncn_operator_state")]
    #[account(8, name = "vault_operator_delegation")]
    #[account(9, writable, name = "snapshot")]
    SnapshotVaultOperatorDelegation{},

    // ---------------------------------------------------- //
    //                         VOTE                         //
    // ---------------------------------------------------- //
    /// Cast a vote
    #[account(0, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, name = "snapshot")]
    #[account(3, name = "restaking_config")]
    #[account(4, writable, name = "vote_counter")]
    CastVote {
        aggregated_signature: [u8; 32],
        aggregated_g2: [u8; 64],
        operators_signature_bitmap: Vec<u8>,
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
        minimum_stake: Option<u128>,
    },


    /// Sets a new secondary admin for the NCN
    #[account(0, writable, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, signer, name = "ncn_admin")]
    #[account(3, name = "new_admin")]
    AdminSetNewAdmin {
        role: ConfigAdminRole,
    },

    /// Registers a new ST mint in the Vault Registry
    #[account(0, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, name = "st_mint")]
    #[account(3, writable, name = "vault_registry")]
    #[account(4, signer, writable, name = "admin")]
    AdminRegisterStMint{ },
}
