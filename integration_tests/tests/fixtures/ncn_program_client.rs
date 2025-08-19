use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{
    config::Config, ncn_operator_state::NcnOperatorState, ncn_vault_ticket::NcnVaultTicket,
};
use jito_restaking_program;
use jito_vault_core::{
    vault_ncn_ticket::VaultNcnTicket, vault_operator_delegation::VaultOperatorDelegation,
};
use ncn_program_client::{
    instructions::{
        AdminRegisterStMintBuilder, AdminSetNewAdminBuilder, AdminSetParametersBuilder,
        AdminSetStMintBuilder, AdminSetWeightBuilder, CastVoteBuilder, CloseEpochAccountBuilder,
        InitializeConfigBuilder, InitializeEpochSnapshotBuilder, InitializeEpochStateBuilder,
        InitializeOperatorSnapshotBuilder, InitializeVaultRegistryBuilder,
        InitializeVoteCounterBuilder, InitializeWeightTableBuilder, ReallocEpochSnapshotBuilder,
        RegisterOperatorBuilder, RegisterVaultBuilder, SetEpochWeightsBuilder,
        SnapshotVaultOperatorDelegationBuilder, UpdateOperatorBN128KeysBuilder,
    },
    types::ConfigAdminRole,
};
use ncn_program_core::{
    account_payer::AccountPayer,
    config::Config as NcnConfig,
    constants::{G1_COMPRESSED_POINT_SIZE, G2_COMPRESSED_POINT_SIZE, MAX_REALLOC_BYTES},
    epoch_marker::EpochMarker,
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    epoch_state::EpochState,
    error::NCNProgramError,
    fees::FeeConfig,
    operator_registry::OperatorEntry,
    vault_registry::VaultRegistry,
    vote_counter::VoteCounter,
    weight_table::WeightTable,
};
use solana_program::{
    instruction::InstructionError, native_token::sol_to_lamports, pubkey::Pubkey,
    system_instruction::transfer,
};
use solana_program_test::{BanksClient, ProgramTestBanksClientExt};
use solana_sdk::{
    commitment_config::CommitmentLevel,
    compute_budget::ComputeBudgetInstruction,
    signature::{Keypair, Signer},
    system_program,
    transaction::{Transaction, TransactionError},
};

use super::restaking_client::NcnRoot;
use crate::fixtures::{TestError, TestResult};

/// A client for interacting with the NCN program in integration tests.
/// Provides helper methods for initializing accounts, fetching state, and sending transactions.
pub struct NCNProgramClient {
    banks_client: BanksClient,
    payer: Keypair,
}

impl NCNProgramClient {
    /// Creates a new NCN program client.
    pub const fn new(banks_client: BanksClient, payer: Keypair) -> Self {
        Self {
            banks_client,
            payer,
        }
    }

    /// Processes a transaction using the BanksClient with processed commitment level.
    pub async fn process_transaction(&mut self, tx: &Transaction) -> TestResult<()> {
        self.banks_client
            .process_transaction_with_preflight_and_commitment(
                tx.clone(),
                CommitmentLevel::Processed,
            )
            .await?;
        Ok(())
    }

    /// Airdrops SOL to a specified public key.
    pub async fn airdrop(&mut self, to: &Pubkey, sol: f64) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;
        let new_blockhash = self
            .banks_client
            .get_new_latest_blockhash(&blockhash)
            .await
            .unwrap();
        self.banks_client
            .process_transaction_with_preflight_and_commitment(
                Transaction::new_signed_with_payer(
                    &[transfer(&self.payer.pubkey(), to, sol_to_lamports(sol))],
                    Some(&self.payer.pubkey()),
                    &[&self.payer],
                    new_blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await?;
        Ok(())
    }

    /// Sets up the NCN program by initializing the config and vault registry.
    pub async fn setup_ncn_program(&mut self, ncn_root: &NcnRoot) -> TestResult<()> {
        self.do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin, None)
            .await?;

        self.do_initialize_vote_counter(ncn_root.ncn_pubkey).await?;

        self.do_full_initialize_vault_registry(ncn_root.ncn_pubkey)
            .await?;

        Ok(())
    }

    /// Initializes the vote counter account for a given NCN.
    pub async fn do_initialize_vote_counter(&mut self, ncn: Pubkey) -> TestResult<()> {
        let config = NcnConfig::find_program_address(&ncn_program::id(), &ncn).0;
        let (vote_counter, _, _) = VoteCounter::find_program_address(&ncn_program::id(), &ncn);
        let (account_payer, _, _) = AccountPayer::find_program_address(&ncn_program::id(), &ncn);

        self.initialize_vote_counter(config, vote_counter, ncn, account_payer)
            .await
    }

    /// Sends a transaction to initialize the vote counter account.
    pub async fn initialize_vote_counter(
        &mut self,
        config: Pubkey,
        vote_counter: Pubkey,
        ncn: Pubkey,
        account_payer: Pubkey,
    ) -> TestResult<()> {
        let ix = InitializeVoteCounterBuilder::new()
            .config(config)
            .vote_counter(vote_counter)
            .ncn(ncn)
            .account_payer(account_payer)
            .system_program(system_program::id())
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    /// Fetches the EpochMarker account for a given NCN and epoch.
    pub async fn get_epoch_marker(&mut self, ncn: Pubkey, epoch: u64) -> TestResult<EpochMarker> {
        let epoch_marker = EpochMarker::find_program_address(&ncn_program::id(), &ncn, epoch).0;
        let raw_account = self.banks_client.get_account(epoch_marker).await?.unwrap();
        Ok(*EpochMarker::try_from_slice_unchecked(raw_account.data.as_slice()).unwrap())
    }

    /// Fetches the NCN Config account for a given NCN pubkey.
    pub async fn get_ncn_config(&mut self, ncn_pubkey: Pubkey) -> TestResult<NcnConfig> {
        let config_pda = NcnConfig::find_program_address(&ncn_program::id(), &ncn_pubkey).0;
        let config = self.banks_client.get_account(config_pda).await?.unwrap();
        Ok(*NcnConfig::try_from_slice_unchecked(config.data.as_slice()).unwrap())
    }

    /// Fetches the VoteCounter account for a given NCN pubkey.
    pub async fn get_vote_counter(&mut self, ncn_pubkey: Pubkey) -> TestResult<VoteCounter> {
        let vote_counter_pda = VoteCounter::find_program_address(&ncn_program::id(), &ncn_pubkey).0;
        let vote_counter = self
            .banks_client
            .get_account(vote_counter_pda)
            .await?
            .unwrap();
        Ok(*VoteCounter::try_from_slice_unchecked(vote_counter.data.as_slice()).unwrap())
    }

    /// Fetches the VaultRegistry account for a given NCN pubkey.
    pub async fn get_vault_registry(&mut self, ncn_pubkey: Pubkey) -> TestResult<VaultRegistry> {
        let vault_registry_pda =
            VaultRegistry::find_program_address(&ncn_program::id(), &ncn_pubkey).0;
        let vault_registry = self
            .banks_client
            .get_account(vault_registry_pda)
            .await?
            .unwrap();
        Ok(*VaultRegistry::try_from_slice_unchecked(vault_registry.data.as_slice()).unwrap())
    }

    /// Fetches the EpochState account for a given NCN and epoch.
    pub async fn get_epoch_state(&mut self, ncn: Pubkey, epoch: u64) -> TestResult<EpochState> {
        let epoch_state = EpochState::find_program_address(&ncn_program::id(), &ncn, epoch).0;
        let raw_account = self.banks_client.get_account(epoch_state).await?.unwrap();
        Ok(*EpochState::try_from_slice_unchecked(raw_account.data.as_slice()).unwrap())
    }

    /// Fetches the WeightTable account for a given NCN and epoch.
    #[allow(dead_code)]
    pub async fn get_weight_table(
        &mut self,
        ncn: Pubkey,
        ncn_epoch: u64,
    ) -> TestResult<WeightTable> {
        let address = WeightTable::find_program_address(&ncn_program::id(), &ncn, ncn_epoch).0;

        let raw_account = self.banks_client.get_account(address).await?.unwrap();

        let account = WeightTable::try_from_slice_unchecked(raw_account.data.as_slice()).unwrap();

        Ok(*account)
    }

    /// Fetches the EpochSnapshot account for a given NCN and epoch.
    pub async fn get_epoch_snapshot(&mut self, ncn: Pubkey) -> TestResult<Box<EpochSnapshot>> {
        let address = EpochSnapshot::find_program_address(&ncn_program::id(), &ncn).0;

        let raw_account = Box::new(self.banks_client.get_account(address).await?.unwrap());

        let account = Box::new(
            *EpochSnapshot::try_from_slice_unchecked(raw_account.data.as_slice()).unwrap(),
        );

        Ok(account)
    }

    /// Fetches the OperatorEntry account for a given NCN and operator.
    pub async fn get_operator_entry(
        &mut self,
        ncn: Pubkey,
        operator: Pubkey,
    ) -> TestResult<OperatorEntry> {
        let operator_entry =
            OperatorEntry::find_program_address(&ncn_program::id(), &ncn, &operator).0;
        let raw_account = self
            .banks_client
            .get_account(operator_entry)
            .await?
            .unwrap();
        Ok(*OperatorEntry::try_from_slice_unchecked(raw_account.data.as_slice()).unwrap())
    }

    /// Fetches the OperatorSnapshot from the EpochSnapshot for a given operator, NCN, and epoch.
    #[allow(dead_code)]
    pub async fn get_operator_snapshot(
        &mut self,
        operator: Pubkey,
        ncn: Pubkey,
    ) -> TestResult<OperatorSnapshot> {
        // Get the epoch snapshot which contains the operator snapshots
        let epoch_snapshot = self.get_epoch_snapshot(ncn).await?;

        // Find the operator snapshot by operator pubkey
        let operator_snapshot = epoch_snapshot.find_operator_snapshot(&operator);

        if operator_snapshot.is_none() {
            return Err(TestError::ProgramError(
                NCNProgramError::OperatorIsNotInSnapshot.into(),
            ));
        }
        Ok(*operator_snapshot.unwrap())
    }

    /// Initializes the NCN config account and airdrops funds to the account payer.
    pub async fn do_initialize_config(
        &mut self,
        ncn: Pubkey,
        ncn_admin: &Keypair,
        minimum_stake_weight: Option<u128>,
    ) -> TestResult<()> {
        // Setup Payer
        self.airdrop(&self.payer.pubkey(), 1.0).await?;

        // Setup account payer
        let (account_payer, _, _) = AccountPayer::find_program_address(&ncn_program::id(), &ncn);
        self.airdrop(&account_payer, 100.0).await?;

        let ncn_admin_pubkey = ncn_admin.pubkey();

        let ncn_fee_wallet = Keypair::new();
        self.airdrop(&ncn_fee_wallet.pubkey(), 0.1).await?;

        // Airdroping some SOL to Protocol fee wallet to get it started.
        let jito_fee_wallet = FeeConfig::PROTOCOL_FEE_WALLET;
        self.airdrop(&jito_fee_wallet, 0.1).await?;

        self.initialize_config(
            ncn,
            ncn_admin,
            &ncn_admin_pubkey,
            3,
            10,
            10000,
            &ncn_fee_wallet.pubkey(),
            400,
            minimum_stake_weight.unwrap_or(1000),
        )
        .await
    }

    /// Sends a transaction to initialize the NCN config account.
    #[allow(clippy::too_many_arguments)]
    pub async fn initialize_config(
        &mut self,
        ncn: Pubkey,
        ncn_admin: &Keypair,
        tie_breaker_admin: &Pubkey,
        epochs_before_stall: u64,
        epochs_after_consensus_before_close: u64,
        valid_slots_after_consensus: u64,
        ncn_fee_wallet: &Pubkey,
        ncn_fee_bps: u16,
        minimum_stake_weight: u128,
    ) -> TestResult<()> {
        let config = NcnConfig::find_program_address(&ncn_program::id(), &ncn).0;

        let (account_payer, _, _) = AccountPayer::find_program_address(&ncn_program::id(), &ncn);

        let ix = InitializeConfigBuilder::new()
            .config(config)
            .ncn(ncn)
            .ncn_fee_wallet(*ncn_fee_wallet)
            .ncn_admin(ncn_admin.pubkey())
            .account_payer(account_payer)
            .tie_breaker_admin(*tie_breaker_admin)
            .epochs_before_stall(epochs_before_stall)
            .epochs_after_consensus_before_close(epochs_after_consensus_before_close)
            .valid_slots_after_consensus(valid_slots_after_consensus)
            .minimum_stake_weight(minimum_stake_weight)
            .ncn_fee_bps(ncn_fee_bps)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&ncn_admin.pubkey()),
            &[&ncn_admin],
            blockhash,
        ))
        .await
    }

    /// Sets a new admin for a specific role in the NCN config.
    pub async fn do_set_new_admin(
        &mut self,
        role: ConfigAdminRole,
        new_admin: Pubkey,
        ncn_root: &NcnRoot,
    ) -> TestResult<()> {
        let config_pda =
            NcnConfig::find_program_address(&ncn_program::id(), &ncn_root.ncn_pubkey).0;
        self.airdrop(&ncn_root.ncn_admin.pubkey(), 1.0).await?;
        self.set_new_admin(config_pda, role, new_admin, ncn_root)
            .await
    }

    /// Sends a transaction to set a new admin in the NCN config.
    pub async fn set_new_admin(
        &mut self,
        config_pda: Pubkey,
        role: ConfigAdminRole,
        new_admin: Pubkey,
        ncn_root: &NcnRoot,
    ) -> TestResult<()> {
        let ix = AdminSetNewAdminBuilder::new()
            .config(config_pda)
            .ncn(ncn_root.ncn_pubkey)
            .ncn_admin(ncn_root.ncn_admin.pubkey())
            .new_admin(new_admin)
            .role(role)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&ncn_root.ncn_admin.pubkey()),
            &[&ncn_root.ncn_admin],
            blockhash,
        ))
        .await
    }

    /// Initializes the epoch state account for a given NCN and epoch.
    pub async fn do_intialize_epoch_state(&mut self, ncn: Pubkey, epoch: u64) -> TestResult<()> {
        self.initialize_epoch_state(ncn, epoch).await
    }

    /// Sends a transaction to initialize the epoch state account.
    pub async fn initialize_epoch_state(&mut self, ncn: Pubkey, epoch: u64) -> TestResult<()> {
        let (epoch_marker, _, _) =
            EpochMarker::find_program_address(&ncn_program::id(), &ncn, epoch);
        let epoch_state = EpochState::find_program_address(&ncn_program::id(), &ncn, epoch).0;

        let config = NcnConfig::find_program_address(&ncn_program::id(), &ncn).0;

        let (account_payer, _, _) = AccountPayer::find_program_address(&ncn_program::id(), &ncn);

        let ix = InitializeEpochStateBuilder::new()
            .epoch_marker(epoch_marker)
            .epoch_state(epoch_state)
            .config(config)
            .ncn(ncn)
            .account_payer(account_payer)
            .system_program(system_program::id())
            .epoch(epoch)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    /// Initializes and fully reallocates the weight table account for a given NCN and epoch.
    pub async fn do_full_initialize_weight_table(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        self.do_initialize_weight_table(ncn, epoch).await?;
        Ok(())
    }

    /// Initializes the weight table account for a given NCN and epoch.
    pub async fn do_initialize_weight_table(&mut self, ncn: Pubkey, epoch: u64) -> TestResult<()> {
        self.initialize_weight_table(ncn, epoch).await
    }

    /// Sends a transaction to initialize the weight table account.
    pub async fn initialize_weight_table(&mut self, ncn: Pubkey, epoch: u64) -> TestResult<()> {
        let (epoch_marker, _, _) =
            EpochMarker::find_program_address(&ncn_program::id(), &ncn, epoch);
        let epoch_state = EpochState::find_program_address(&ncn_program::id(), &ncn, epoch).0;
        let vault_registry = VaultRegistry::find_program_address(&ncn_program::id(), &ncn).0;
        let weight_table = WeightTable::find_program_address(&ncn_program::id(), &ncn, epoch).0;

        let (account_payer, _, _) = AccountPayer::find_program_address(&ncn_program::id(), &ncn);

        let ix = InitializeWeightTableBuilder::new()
            .epoch_marker(epoch_marker)
            .epoch_state(epoch_state)
            .vault_registry(vault_registry)
            .ncn(ncn)
            .weight_table(weight_table)
            .account_payer(account_payer)
            .system_program(system_program::id())
            .epoch(epoch)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    /// Sets the epoch weights in the weight table based on the vault registry.
    pub async fn do_set_epoch_weights(&mut self, ncn: Pubkey, epoch: u64) -> TestResult<()> {
        self.set_epoch_weights(ncn, epoch).await
    }

    /// Sends a transaction to set the epoch weights in the weight table.
    pub async fn set_epoch_weights(&mut self, ncn: Pubkey, epoch: u64) -> TestResult<()> {
        let weight_table = WeightTable::find_program_address(&ncn_program::id(), &ncn, epoch).0;
        let epoch_state = EpochState::find_program_address(&ncn_program::id(), &ncn, epoch).0;
        let vault_registry = VaultRegistry::find_program_address(&ncn_program::id(), &ncn).0;

        let ix = SetEpochWeightsBuilder::new()
            .epoch_state(epoch_state)
            .ncn(ncn)
            .weight_table(weight_table)
            .vault_registry(vault_registry)
            .epoch(epoch)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    /// Sets the weight for a specific st_mint in the weight table (admin operation).
    pub async fn do_admin_set_weight(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
        st_mint: Pubkey,
        weight: u128,
    ) -> TestResult<()> {
        self.admin_set_weight(ncn, epoch, st_mint, weight).await
    }

    /// Sends a transaction to set the weight for a specific st_mint in the weight table (admin operation).
    pub async fn admin_set_weight(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
        st_mint: Pubkey,
        weight: u128,
    ) -> TestResult<()> {
        let weight_table = WeightTable::find_program_address(&ncn_program::id(), &ncn, epoch).0;
        let epoch_state = EpochState::find_program_address(&ncn_program::id(), &ncn, epoch).0;

        let ix = AdminSetWeightBuilder::new()
            .epoch_state(epoch_state)
            .ncn(ncn)
            .weight_table(weight_table)
            .weight_table_admin(self.payer.pubkey())
            .st_mint(st_mint)
            .weight(weight)
            .epoch(epoch)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    /// Initializes and fully reallocates the vault registry account for a given NCN.
    pub async fn do_full_initialize_vault_registry(&mut self, ncn: Pubkey) -> TestResult<()> {
        self.do_initialize_vault_registry(ncn).await?;
        // let num_reallocs = (WeightTable::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1;
        // self.do_realloc_vault_registry(ncn, num_reallocs).await?;
        Ok(())
    }

    /// Initializes the vault registry account for a given NCN.
    pub async fn do_initialize_vault_registry(&mut self, ncn: Pubkey) -> TestResult<()> {
        let ncn_config = NcnConfig::find_program_address(&ncn_program::id(), &ncn).0;
        let vault_registry = VaultRegistry::find_program_address(&ncn_program::id(), &ncn).0;

        self.initialize_vault_registry(&ncn_config, &vault_registry, &ncn)
            .await
    }

    /// Sends a transaction to initialize the vault registry account.
    pub async fn initialize_vault_registry(
        &mut self,
        ncn_config: &Pubkey,
        vault_registry: &Pubkey,
        ncn: &Pubkey,
    ) -> TestResult<()> {
        let (account_payer, _, _) = AccountPayer::find_program_address(&ncn_program::id(), ncn);

        let ix = InitializeVaultRegistryBuilder::new()
            .config(*ncn_config)
            .vault_registry(*vault_registry)
            .ncn(*ncn)
            .account_payer(account_payer)
            .system_program(system_program::id())
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    /// Registers a vault with the NCN program.
    pub async fn do_register_vault(
        &mut self,
        ncn: Pubkey,
        vault: Pubkey,
        ncn_vault_ticket: Pubkey,
    ) -> TestResult<()> {
        let ncn_config = NcnConfig::find_program_address(&ncn_program::id(), &ncn).0;

        let vault_registry = VaultRegistry::find_program_address(&ncn_program::id(), &ncn).0;

        self.register_vault(ncn_config, vault_registry, ncn, vault, ncn_vault_ticket)
            .await
    }

    /// Sends a transaction to register a vault.
    pub async fn register_vault(
        &mut self,
        config: Pubkey,
        vault_registry: Pubkey,
        ncn: Pubkey,
        vault: Pubkey,
        ncn_vault_ticket: Pubkey,
    ) -> TestResult<()> {
        let ix = RegisterVaultBuilder::new()
            .config(config)
            .vault_registry(vault_registry)
            .ncn(ncn)
            .vault(vault)
            .ncn_vault_ticket(ncn_vault_ticket)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    /// Registers an st_mint with a specific weight in the vault registry (admin operation).
    pub async fn do_admin_register_st_mint(
        &mut self,
        ncn: Pubkey,
        st_mint: Pubkey,
        weight: u128,
    ) -> TestResult<()> {
        let vault_registry = VaultRegistry::find_program_address(&ncn_program::id(), &ncn).0;

        let (ncn_config, _, _) = NcnConfig::find_program_address(&ncn_program::id(), &ncn);

        let admin = self.payer.pubkey();

        self.admin_register_st_mint(ncn, ncn_config, vault_registry, admin, st_mint, weight)
            .await
    }

    /// Sends a transaction to register an st_mint in the vault registry (admin operation).
    #[allow(clippy::too_many_arguments)]
    pub async fn admin_register_st_mint(
        &mut self,
        ncn: Pubkey,
        ncn_config: Pubkey,
        vault_registry: Pubkey,
        admin: Pubkey,
        st_mint: Pubkey,
        weight: u128,
    ) -> TestResult<()> {
        let ix = {
            let mut builder = AdminRegisterStMintBuilder::new();
            builder
                .config(ncn_config)
                .ncn(ncn)
                .vault_registry(vault_registry)
                .admin(admin)
                .st_mint(st_mint)
                .weight(weight);

            builder.instruction()
        };

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    /// Sets the weight for an existing st_mint in the vault registry (admin operation).
    pub async fn do_admin_set_st_mint(
        &mut self,
        ncn: Pubkey,
        st_mint: Pubkey,
        weight: u128,
    ) -> TestResult<()> {
        let vault_registry = VaultRegistry::find_program_address(&ncn_program::id(), &ncn).0;

        let (ncn_config, _, _) = NcnConfig::find_program_address(&ncn_program::id(), &ncn);

        let admin = self.payer.pubkey();

        self.admin_set_st_mint(ncn, ncn_config, vault_registry, admin, st_mint, weight)
            .await
    }

    /// Sends a transaction to set the weight for an st_mint in the vault registry (admin operation).
    #[allow(clippy::too_many_arguments)]
    pub async fn admin_set_st_mint(
        &mut self,
        ncn: Pubkey,
        ncn_config: Pubkey,
        vault_registry: Pubkey,
        admin: Pubkey,
        st_mint: Pubkey,
        weight: u128,
    ) -> TestResult<()> {
        let ix = {
            let mut builder = AdminSetStMintBuilder::new();
            builder
                .config(ncn_config)
                .ncn(ncn)
                .vault_registry(vault_registry)
                .admin(admin)
                .st_mint(st_mint)
                .weight(weight);

            builder.instruction()
        };

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    /// Initializes the epoch snapshot account for a given NCN and epoch.
    pub async fn do_initialize_epoch_snapshot(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        self.initialize_epoch_snapshot(ncn, epoch).await
    }

    /// Sends a transaction to initialize the epoch snapshot account.
    pub async fn initialize_epoch_snapshot(&mut self, ncn: Pubkey, epoch: u64) -> TestResult<()> {
        let (epoch_marker, _, _) =
            EpochMarker::find_program_address(&ncn_program::id(), &ncn, epoch);
        let epoch_state = EpochState::find_program_address(&ncn_program::id(), &ncn, epoch).0;
        let epoch_snapshot = EpochSnapshot::find_program_address(&ncn_program::id(), &ncn).0;

        let (account_payer, _, _) = AccountPayer::find_program_address(&ncn_program::id(), &ncn);

        let ix = InitializeEpochSnapshotBuilder::new()
            .epoch_marker(epoch_marker)
            .epoch_state(epoch_state)
            .ncn(ncn)
            .epoch_snapshot(epoch_snapshot)
            .account_payer(account_payer)
            .system_program(system_program::id())
            .epoch(epoch)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    /// Initializes and fully reallocates the epoch snapshot account for a given NCN and epoch.
    pub async fn do_full_initialize_epoch_snapshot(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        self.do_initialize_epoch_snapshot(ncn, epoch).await?;
        let num_reallocs =
            (EpochSnapshot::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1;
        self.do_realloc_epoch_snapshot(ncn, epoch, num_reallocs)
            .await?;
        Ok(())
    }

    /// Reallocates the epoch snapshot account multiple times.
    pub async fn do_realloc_epoch_snapshot(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
        num_reallocations: u64,
    ) -> TestResult<()> {
        let epoch_state = EpochState::find_program_address(&ncn_program::id(), &ncn, epoch).0;
        let weight_table = WeightTable::find_program_address(&ncn_program::id(), &ncn, epoch).0;
        let epoch_snapshot = EpochSnapshot::find_program_address(&ncn_program::id(), &ncn).0;
        let config = NcnConfig::find_program_address(&ncn_program::id(), &ncn).0;

        self.realloc_epoch_snapshot(
            &ncn,
            &epoch_state,
            &weight_table,
            &epoch_snapshot,
            &config,
            epoch,
            num_reallocations,
        )
        .await
    }

    /// Sends transactions to reallocate the epoch snapshot account.
    #[allow(clippy::too_many_arguments)]
    pub async fn realloc_epoch_snapshot(
        &mut self,
        ncn: &Pubkey,
        epoch_state: &Pubkey,
        weight_table: &Pubkey,
        epoch_snapshot: &Pubkey,
        config: &Pubkey,
        epoch: u64,
        num_reallocations: u64,
    ) -> TestResult<()> {
        let (account_payer, _, _) = AccountPayer::find_program_address(&ncn_program::id(), ncn);

        let ix = ReallocEpochSnapshotBuilder::new()
            .epoch_state(*epoch_state)
            .ncn(*ncn)
            .weight_table(*weight_table)
            .epoch_snapshot(*epoch_snapshot)
            .account_payer(account_payer)
            .system_program(system_program::id())
            .config(*config)
            .epoch(epoch)
            .instruction();

        let ixs = vec![ix; num_reallocations as usize];

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &ixs,
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    /// Initializes the operator snapshot within the epoch snapshot for a given operator, NCN, and epoch.
    pub async fn do_initialize_operator_snapshot(
        &mut self,
        operator: Pubkey,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        self.initialize_operator_snapshot(operator, ncn, epoch)
            .await
    }

    /// Sends a transaction to initialize the operator snapshot account.
    pub async fn initialize_operator_snapshot(
        &mut self,
        operator: Pubkey,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let (epoch_marker, _, _) =
            EpochMarker::find_program_address(&ncn_program::id(), &ncn, epoch);
        let epoch_state = EpochState::find_program_address(&ncn_program::id(), &ncn, epoch).0;
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;
        let ncn_operator_state =
            NcnOperatorState::find_program_address(&jito_restaking_program::id(), &ncn, &operator)
                .0;
        let epoch_snapshot = EpochSnapshot::find_program_address(&ncn_program::id(), &ncn).0;

        let (account_payer, _, _) = AccountPayer::find_program_address(&ncn_program::id(), &ncn);

        let operator_entry =
            OperatorEntry::find_program_address(&ncn_program::id(), &ncn, &operator).0;

        let ix = InitializeOperatorSnapshotBuilder::new()
            .epoch_marker(epoch_marker)
            .epoch_state(epoch_state)
            .restaking_config(restaking_config)
            .ncn(ncn)
            .operator(operator)
            .ncn_operator_state(ncn_operator_state)
            .operator_entry(operator_entry)
            .epoch_snapshot(epoch_snapshot)
            .account_payer(account_payer)
            .system_program(system_program::id())
            .epoch(epoch)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    /// Snapshots the delegation information from a vault to an operator for a given NCN and epoch.
    pub async fn do_snapshot_vault_operator_delegation(
        &mut self,
        vault: Pubkey,
        operator: Pubkey,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        self.snapshot_vault_operator_delegation(vault, operator, ncn, epoch)
            .await
    }

    /// Sends a transaction to snapshot the vault operator delegation.
    pub async fn snapshot_vault_operator_delegation(
        &mut self,
        vault: Pubkey,
        operator: Pubkey,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let epoch_state = EpochState::find_program_address(&ncn_program::id(), &ncn, epoch).0;
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;

        let config_pda = NcnConfig::find_program_address(&ncn_program::id(), &ncn).0;

        let epoch_snapshot = EpochSnapshot::find_program_address(&ncn_program::id(), &ncn).0;

        let vault_ncn_ticket =
            VaultNcnTicket::find_program_address(&jito_vault_program::id(), &vault, &ncn).0;

        let ncn_vault_ticket =
            NcnVaultTicket::find_program_address(&jito_restaking_program::id(), &ncn, &vault).0;

        let vault_operator_delegation = VaultOperatorDelegation::find_program_address(
            &jito_vault_program::id(),
            &vault,
            &operator,
        )
        .0;

        let weight_table = WeightTable::find_program_address(&ncn_program::id(), &ncn, epoch).0;

        let ix = SnapshotVaultOperatorDelegationBuilder::new()
            .epoch_state(epoch_state)
            .config(config_pda)
            .restaking_config(restaking_config)
            .ncn(ncn)
            .operator(operator)
            .vault(vault)
            .vault_ncn_ticket(vault_ncn_ticket)
            .ncn_vault_ticket(ncn_vault_ticket)
            .vault_operator_delegation(vault_operator_delegation)
            .weight_table(weight_table)
            .epoch_snapshot(epoch_snapshot)
            .epoch(epoch)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    /// Casts a vote using BLS signature aggregation for a given epoch.
    pub async fn do_cast_vote(
        &mut self,
        ncn: Pubkey,
        agg_sig: [u8; 32],
        apk2: [u8; 64],
        signers_bitmap: Vec<u8>,
    ) -> Result<(), TestError> {
        let ncn_config = NcnConfig::find_program_address(&ncn_program::id(), &ncn).0;
        let epoch_snapshot = EpochSnapshot::find_program_address(&ncn_program::id(), &ncn).0;
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;
        let vote_counter = VoteCounter::find_program_address(&ncn_program::id(), &ncn).0;

        self.cast_vote(
            ncn_config,
            ncn,
            epoch_snapshot,
            restaking_config,
            vote_counter,
            agg_sig,
            apk2,
            signers_bitmap,
        )
        .await
    }

    /// Sends a transaction to cast a vote using BLS signature verification.
    #[allow(clippy::too_many_arguments)]
    pub async fn cast_vote(
        &mut self,
        ncn_config: Pubkey,
        ncn: Pubkey,
        epoch_snapshot: Pubkey,
        restaking_config: Pubkey,
        vote_counter: Pubkey,
        agg_sig: [u8; 32],
        apk2: [u8; 64],
        signers_bitmap: Vec<u8>,
    ) -> Result<(), TestError> {
        let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(2_000_000);

        let ix = CastVoteBuilder::new()
            .config(ncn_config)
            .ncn(ncn)
            .epoch_snapshot(epoch_snapshot)
            .restaking_config(restaking_config)
            .vote_counter(vote_counter)
            .aggregated_signature(agg_sig)
            .aggregated_g2(apk2)
            .operators_signature_bitmap(signers_bitmap)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[compute_budget_ix, ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    /// Closes an epoch-specific account (e.g., BallotBox, EpochSnapshot) after the epoch is finished.
    pub async fn do_close_epoch_account(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
        account_to_close: Pubkey,
    ) -> TestResult<()> {
        let (epoch_marker, _, _) =
            EpochMarker::find_program_address(&ncn_program::id(), &ncn, epoch);

        let epoch_state = EpochState::find_program_address(&ncn_program::id(), &ncn, epoch).0;

        let (account_payer, _, _) = AccountPayer::find_program_address(&ncn_program::id(), &ncn);

        let (config, _, _) = NcnConfig::find_program_address(&ncn_program::id(), &ncn);

        self.close_epoch_account(
            epoch_marker,
            epoch_state,
            ncn,
            config,
            account_to_close,
            account_payer,
            epoch,
        )
        .await
    }

    /// Sends a transaction to close an epoch-specific account.
    #[allow(clippy::too_many_arguments)]
    pub async fn close_epoch_account(
        &mut self,
        epoch_marker: Pubkey,
        epoch_state: Pubkey,
        ncn: Pubkey,
        config: Pubkey,
        account_to_close: Pubkey,
        account_payer: Pubkey,

        epoch: u64,
    ) -> TestResult<()> {
        let mut ix = CloseEpochAccountBuilder::new();

        ix.account_payer(account_payer)
            .epoch_marker(epoch_marker)
            .config(config)
            .account_to_close(account_to_close)
            .epoch_state(epoch_state)
            .ncn(ncn)
            .system_program(system_program::id())
            .epoch(epoch);

        let ix = ix.instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        );

        self.process_transaction(&tx).await
    }

    /// Sets various parameters in the NCN config (admin operation).
    pub async fn do_set_parameters(
        &mut self,
        starting_valid_epoch: Option<u64>,
        epochs_before_stall: Option<u64>,
        epochs_after_consensus_before_close: Option<u64>,
        valid_slots_after_consensus: Option<u64>,
        minimum_stake_weight: Option<u128>,
        ncn_root: &NcnRoot,
    ) -> TestResult<()> {
        let config_pda =
            NcnConfig::find_program_address(&ncn_program::id(), &ncn_root.ncn_pubkey).0;

        let mut ix = AdminSetParametersBuilder::new();
        ix.config(config_pda)
            .ncn(ncn_root.ncn_pubkey)
            .ncn_admin(ncn_root.ncn_admin.pubkey());

        if let Some(epoch) = starting_valid_epoch {
            ix.starting_valid_epoch(epoch);
        }

        if let Some(epochs) = epochs_before_stall {
            ix.epochs_before_stall(epochs);
        }

        if let Some(epochs) = epochs_after_consensus_before_close {
            ix.epochs_after_consensus_before_close(epochs);
        }

        if let Some(slots) = valid_slots_after_consensus {
            ix.valid_slots_after_consensus(slots);
        }

        if let Some(minimum_stake_weight) = minimum_stake_weight {
            ix.minimum_stake_weight(minimum_stake_weight);
        }

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix.instruction()],
            Some(&ncn_root.ncn_admin.pubkey()),
            &[&ncn_root.ncn_admin],
            blockhash,
        ))
        .await
    }

    pub async fn do_register_operator(
        &mut self,
        ncn: Pubkey,
        operator_pubkey: Pubkey,
        operator_admin: &Keypair,
        g1_pubkey: [u8; G1_COMPRESSED_POINT_SIZE],
        g2_pubkey: [u8; G2_COMPRESSED_POINT_SIZE],
        signature: [u8; 64],
    ) -> TestResult<()> {
        let config = NcnConfig::find_program_address(&ncn_program::id(), &ncn).0;
        let operator_entry =
            OperatorEntry::find_program_address(&ncn_program::id(), &ncn, &operator_pubkey).0;
        let ncn_operator_state = NcnOperatorState::find_program_address(
            &jito_restaking_program::id(),
            &ncn,
            &operator_pubkey,
        )
        .0;
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;
        let (account_payer, _, _) = AccountPayer::find_program_address(&ncn_program::id(), &ncn);

        self.register_operator(
            config,
            operator_entry,
            ncn_operator_state,
            restaking_config,
            ncn,
            operator_pubkey,
            operator_admin,
            account_payer,
            g1_pubkey,
            g2_pubkey,
            signature,
        )
        .await
    }

    /// Sends a transaction to register an operator with BLS keys.
    #[allow(clippy::too_many_arguments)]
    pub async fn register_operator(
        &mut self,
        config: Pubkey,
        operator_entry: Pubkey,
        ncn_operator_state: Pubkey,
        restaking_config: Pubkey,
        ncn: Pubkey,
        operator_pubkey: Pubkey,
        operator_admin: &Keypair,
        account_payer: Pubkey,
        g1_pubkey: [u8; G1_COMPRESSED_POINT_SIZE],
        g2_pubkey: [u8; G2_COMPRESSED_POINT_SIZE],
        signature: [u8; 64],
    ) -> TestResult<()> {
        let ix = RegisterOperatorBuilder::new()
            .config(config)
            .operator_entry(operator_entry)
            .ncn(ncn)
            .operator(operator_pubkey)
            .operator_admin(operator_admin.pubkey())
            .ncn_operator_state(ncn_operator_state)
            .restaking_config(restaking_config)
            .account_payer(account_payer)
            .system_program(system_program::id())
            .g1_pubkey(g1_pubkey)
            .g2_pubkey(g2_pubkey)
            .signature(signature)
            .instruction();

        let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(1_000_000);

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix, compute_budget_ix],
            Some(&self.payer.pubkey()),
            &[&self.payer, operator_admin],
            blockhash,
        ))
        .await
    }

    /// Updates an operator's BLS keys with simplified parameters
    pub async fn do_update_operator_bn128_keys(
        &mut self,
        ncn: Pubkey,
        operator_pubkey: Pubkey,
        operator_admin: &Keypair,
        g1_pubkey: [u8; G1_COMPRESSED_POINT_SIZE],
        g2_pubkey: [u8; G2_COMPRESSED_POINT_SIZE],
        signature: [u8; 64],
    ) -> TestResult<()> {
        let config = NcnConfig::find_program_address(&ncn_program::id(), &ncn).0;
        let operator_entry =
            OperatorEntry::find_program_address(&ncn_program::id(), &ncn, &operator_pubkey).0;

        self.update_operator_bn128_keys(
            config,
            operator_entry,
            ncn,
            operator_pubkey,
            operator_admin,
            g1_pubkey,
            g2_pubkey,
            signature,
        )
        .await
    }

    /// Updates an operator's BLS keys in the operator registry with full parameter control
    #[allow(clippy::too_many_arguments)]
    pub async fn update_operator_bn128_keys(
        &mut self,
        config: Pubkey,
        operator_entry: Pubkey,
        ncn: Pubkey,
        operator_pubkey: Pubkey,
        operator_admin: &Keypair,
        g1_pubkey: [u8; G1_COMPRESSED_POINT_SIZE],
        g2_pubkey: [u8; G2_COMPRESSED_POINT_SIZE],
        signature: [u8; 64],
    ) -> TestResult<()> {
        let ix = UpdateOperatorBN128KeysBuilder::new()
            .config(config)
            .operator_entry(operator_entry)
            .ncn(ncn)
            .operator(operator_pubkey)
            .operator_admin(operator_admin.pubkey())
            .g1_pubkey(g1_pubkey)
            .g2_pubkey(g2_pubkey)
            .signature(signature)
            .instruction();

        let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(1_000_000);

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix, compute_budget_ix],
            Some(&self.payer.pubkey()),
            &[&self.payer, operator_admin],
            blockhash,
        ))
        .await
    }
}

/// Asserts that a TestResult contains a specific NCNProgramError.
#[inline(always)]
#[track_caller]
pub fn assert_ncn_program_error<T>(
    test_error: Result<T, TestError>,
    ncn_program_error: NCNProgramError,
    instruction_index: Option<u8>,
) {
    assert!(test_error.is_err());
    assert_eq!(
        test_error.err().unwrap().to_transaction_error().unwrap(),
        TransactionError::InstructionError(
            instruction_index.unwrap_or(0),
            InstructionError::Custom(ncn_program_error as u32)
        )
    );
}
