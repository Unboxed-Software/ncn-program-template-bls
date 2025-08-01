use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{
    config::Config, ncn::Ncn, ncn_operator_state::NcnOperatorState,
    ncn_vault_slasher_ticket::NcnVaultSlasherTicket, ncn_vault_ticket::NcnVaultTicket,
    operator::Operator, operator_vault_ticket::OperatorVaultTicket,
};
use jito_restaking_sdk::{
    error::RestakingError,
    instruction::OperatorAdminRole,
    sdk::{
        cooldown_ncn_vault_ticket, initialize_config, initialize_ncn,
        initialize_ncn_operator_state, initialize_ncn_vault_slasher_ticket,
        initialize_ncn_vault_ticket, initialize_operator, initialize_operator_vault_ticket,
        ncn_cooldown_operator, ncn_set_admin, ncn_warmup_operator, operator_cooldown_ncn,
        operator_set_admin, operator_set_fee, operator_set_secondary_admin, operator_warmup_ncn,
        set_config_admin, warmup_ncn_vault_slasher_ticket, warmup_ncn_vault_ticket,
        warmup_operator_vault_ticket,
    },
};
use ncn_program_core::{g1_point::G1Point, g2_point::G2Point, privkey::PrivKey};
use solana_program::{
    instruction::InstructionError, native_token::sol_to_lamports, program_error::ProgramError,
    pubkey::Pubkey, system_instruction::transfer,
};
use solana_program_test::{BanksClient, ProgramTestBanksClientExt};
use solana_sdk::{
    commitment_config::CommitmentLevel,
    signature::{Keypair, Signer},
    transaction::{Transaction, TransactionError},
};

use crate::fixtures::{TestError, TestResult};

/// Represents the root information for an NCN (Node Control Network) in tests.
#[derive(Debug)]
pub struct NcnRoot {
    pub ncn_pubkey: Pubkey,
    pub ncn_admin: Keypair,
}

impl Clone for NcnRoot {
    fn clone(&self) -> Self {
        Self {
            ncn_pubkey: self.ncn_pubkey,
            ncn_admin: self.ncn_admin.insecure_clone(),
        }
    }
}

/// Represents the root information for an Operator in tests.
#[derive(Debug)]
pub struct OperatorRoot {
    pub operator_pubkey: Pubkey,
    pub operator_admin: Keypair,
    pub bn128_privkey: PrivKey,
    pub bn128_g1_pubkey: G1Point,
    pub bn128_g2_pubkey: G2Point,
}

impl Clone for OperatorRoot {
    fn clone(&self) -> Self {
        Self {
            operator_pubkey: self.operator_pubkey,
            operator_admin: self.operator_admin.insecure_clone(),
            bn128_privkey: self.bn128_privkey,
            bn128_g1_pubkey: self.bn128_g1_pubkey,
            bn128_g2_pubkey: self.bn128_g2_pubkey,
        }
    }
}

/// A client for interacting with the Restaking program in integration tests.
/// Provides helper methods for initializing accounts, fetching state, and sending transactions.
pub struct RestakingProgramClient {
    banks_client: BanksClient,
    payer: Keypair,
}

impl RestakingProgramClient {
    /// Creates a new Restaking program client.
    pub const fn new(banks_client: BanksClient, payer: Keypair) -> Self {
        Self {
            banks_client,
            payer,
        }
    }

    /// Fetches the Ncn account for a given NCN pubkey.
    #[allow(dead_code)]
    pub async fn get_ncn(&mut self, ncn: &Pubkey) -> TestResult<Ncn> {
        let account = self
            .banks_client
            .get_account_with_commitment(*ncn, CommitmentLevel::Processed)
            .await?
            .unwrap();

        Ok(*Ncn::try_from_slice_unchecked(account.data.as_slice())?)
    }

    /// Fetches the Config account for the Restaking program.
    pub async fn get_config(&mut self, account: &Pubkey) -> TestResult<Config> {
        let account = self.banks_client.get_account(*account).await?.unwrap();
        Ok(*Config::try_from_slice_unchecked(account.data.as_slice())?)
    }

    /// Fetches the NcnVaultTicket account for a given NCN and vault.
    #[allow(dead_code)]
    pub async fn get_ncn_vault_ticket(
        &mut self,
        ncn: &Pubkey,
        vault: &Pubkey,
    ) -> TestResult<NcnVaultTicket> {
        let account =
            NcnVaultTicket::find_program_address(&jito_restaking_program::id(), ncn, vault).0;
        let account = self.banks_client.get_account(account).await?.unwrap();
        Ok(*NcnVaultTicket::try_from_slice_unchecked(
            account.data.as_slice(),
        )?)
    }

    /// Fetches the NcnOperatorState account for a given NCN and operator.
    #[allow(dead_code)]
    pub async fn get_ncn_operator_state(
        &mut self,
        ncn: &Pubkey,
        operator: &Pubkey,
    ) -> TestResult<NcnOperatorState> {
        let account =
            NcnOperatorState::find_program_address(&jito_restaking_program::id(), ncn, operator).0;
        let account = self.banks_client.get_account(account).await?.unwrap();
        Ok(*NcnOperatorState::try_from_slice_unchecked(
            account.data.as_slice(),
        )?)
    }

    /// Fetches the Operator account for a given operator pubkey.
    #[allow(dead_code)]
    pub async fn get_operator(&mut self, account: &Pubkey) -> TestResult<Operator> {
        let account = self.banks_client.get_account(*account).await?.unwrap();
        Ok(*Operator::try_from_slice_unchecked(
            account.data.as_slice(),
        )?)
    }

    /// Fetches the OperatorVaultTicket account for a given operator and vault.
    #[allow(dead_code)]
    pub async fn get_operator_vault_ticket(
        &mut self,
        operator: &Pubkey,
        vault: &Pubkey,
    ) -> TestResult<OperatorVaultTicket> {
        let account = OperatorVaultTicket::find_program_address(
            &jito_restaking_program::id(),
            operator,
            vault,
        )
        .0;
        let account = self.banks_client.get_account(account).await?.unwrap();
        Ok(*OperatorVaultTicket::try_from_slice_unchecked(
            account.data.as_slice(),
        )?)
    }

    /// Fetches the NcnOperatorState account (viewed from the operator's perspective) for a given operator and NCN.
    #[allow(dead_code)]
    pub async fn get_operator_ncn_ticket(
        &mut self,
        operator: &Pubkey,
        ncn: &Pubkey,
    ) -> TestResult<NcnOperatorState> {
        let account =
            NcnOperatorState::find_program_address(&jito_restaking_program::id(), operator, ncn).0;
        let account = self.banks_client.get_account(account).await?.unwrap();
        Ok(*NcnOperatorState::try_from_slice_unchecked(
            account.data.as_slice(),
        )?)
    }

    /// Initializes the Restaking program config account.
    pub async fn do_initialize_config(&mut self) -> TestResult<Keypair> {
        let restaking_config_pubkey = Config::find_program_address(&jito_restaking_program::id()).0;
        let restaking_config_admin = Keypair::new();

        self.airdrop(&restaking_config_admin.pubkey(), 1.0).await?;
        self.initialize_config(&restaking_config_pubkey, &restaking_config_admin)
            .await?;

        Ok(restaking_config_admin)
    }

    /// Initializes an Operator account.
    pub async fn do_initialize_operator(
        &mut self,
        operator_fee_bps: Option<u16>,
    ) -> TestResult<OperatorRoot> {
        // create operator + add operator vault
        let operator_base = Keypair::new();
        let operator_pubkey =
            Operator::find_program_address(&jito_restaking_program::id(), &operator_base.pubkey())
                .0;
        let operator_admin = Keypair::new();
        self.airdrop(&operator_admin.pubkey(), 1.0).await?;
        self.initialize_operator(
            &Config::find_program_address(&jito_restaking_program::id()).0,
            &operator_pubkey,
            &operator_admin,
            &operator_base,
            operator_fee_bps.unwrap_or(0),
        )
        .await?;

        // Generate BN128 keypair
        let bn128_privkey = PrivKey::from_random();
        let bn128_g1_pubkey = G1Point::try_from(bn128_privkey)
            .map_err(|e| TestError::ProgramError(ProgramError::from(e)))?;
        let bn128_g2_pubkey = G2Point::try_from(&bn128_privkey)
            .map_err(|e| TestError::ProgramError(ProgramError::from(e)))?;

        Ok(OperatorRoot {
            operator_pubkey,
            operator_admin,
            bn128_privkey,
            bn128_g1_pubkey,
            bn128_g2_pubkey,
        })
    }

    /// Initializes an OperatorVaultTicket account, linking an operator and a vault.
    pub async fn do_initialize_operator_vault_ticket(
        &mut self,
        operator_root: &OperatorRoot,
        vault_pubkey: &Pubkey,
    ) -> TestResult<()> {
        let operator_vault_ticket = OperatorVaultTicket::find_program_address(
            &jito_restaking_program::id(),
            &operator_root.operator_pubkey,
            vault_pubkey,
        )
        .0;
        self.initialize_operator_vault_ticket(
            &Config::find_program_address(&jito_restaking_program::id()).0,
            &operator_root.operator_pubkey,
            vault_pubkey,
            &operator_vault_ticket,
            &operator_root.operator_admin,
            &operator_root.operator_admin,
        )
        .await?;

        Ok(())
    }

    /// Warms up an OperatorVaultTicket, making the link active.
    pub async fn do_warmup_operator_vault_ticket(
        &mut self,
        operator_root: &OperatorRoot,
        vault_pubkey: &Pubkey,
    ) -> TestResult<()> {
        let operator_vault_ticket = OperatorVaultTicket::find_program_address(
            &jito_restaking_program::id(),
            &operator_root.operator_pubkey,
            vault_pubkey,
        )
        .0;
        self.warmup_operator_vault_ticket(
            &Config::find_program_address(&jito_restaking_program::id()).0,
            &operator_root.operator_pubkey,
            vault_pubkey,
            &operator_vault_ticket,
            &operator_root.operator_admin,
        )
        .await
    }

    /// Sends a transaction to warm up an OperatorVaultTicket.
    pub async fn warmup_operator_vault_ticket(
        &mut self,
        config: &Pubkey,
        operator: &Pubkey,
        vault: &Pubkey,
        operator_vault_ticket: &Pubkey,
        admin: &Keypair,
    ) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;

        self.process_transaction(&Transaction::new_signed_with_payer(
            &[warmup_operator_vault_ticket(
                &jito_restaking_program::id(),
                config,
                operator,
                vault,
                operator_vault_ticket,
                &admin.pubkey(),
            )],
            Some(&self.payer.pubkey()),
            &[admin, &self.payer],
            blockhash,
        ))
        .await
    }

    /// Sends a transaction to initialize the Restaking program config account.
    pub async fn initialize_config(
        &mut self,
        config: &Pubkey,
        config_admin: &Keypair,
    ) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[initialize_config(
                &jito_restaking_program::id(),
                config,
                &config_admin.pubkey(),
                &jito_vault_program::id(),
            )],
            Some(&config_admin.pubkey()),
            &[config_admin],
            blockhash,
        ))
        .await
    }

    /// Initializes an NCN account.
    pub async fn do_initialize_ncn(&mut self, ncn_admin: Option<Keypair>) -> TestResult<NcnRoot> {
        let ncn_admin = {
            if let Some(ncn_admin) = ncn_admin {
                ncn_admin
            } else {
                self.payer.insecure_clone()
            }
        };
        let ncn_base = Keypair::new();

        self.airdrop(&ncn_admin.pubkey(), 1.0).await?;

        let ncn_pubkey =
            Ncn::find_program_address(&jito_restaking_program::id(), &ncn_base.pubkey()).0;
        self.initialize_ncn(
            &Config::find_program_address(&jito_restaking_program::id()).0,
            &ncn_pubkey,
            &ncn_admin,
            &ncn_base,
        )
        .await?;

        Ok(NcnRoot {
            ncn_pubkey,
            ncn_admin,
        })
    }

    /// Initializes an NcnVaultTicket account, linking an NCN and a vault.
    pub async fn do_initialize_ncn_vault_ticket(
        &mut self,
        ncn_root: &NcnRoot,
        vault: &Pubkey,
    ) -> TestResult<()> {
        let ncn_vault_ticket = NcnVaultTicket::find_program_address(
            &jito_restaking_program::id(),
            &ncn_root.ncn_pubkey,
            vault,
        )
        .0;

        self.initialize_ncn_vault_ticket(
            &Config::find_program_address(&jito_restaking_program::id()).0,
            &ncn_root.ncn_pubkey,
            vault,
            &ncn_vault_ticket,
            &ncn_root.ncn_admin,
            &self.payer.insecure_clone(),
        )
        .await
    }

    /// Warms up an NcnVaultTicket, making the link active.
    pub async fn do_warmup_ncn_vault_ticket(
        &mut self,
        ncn_root: &NcnRoot,
        vault: &Pubkey,
    ) -> TestResult<()> {
        let ncn_vault_ticket = NcnVaultTicket::find_program_address(
            &jito_restaking_program::id(),
            &ncn_root.ncn_pubkey,
            vault,
        )
        .0;
        self.warmup_ncn_vault_ticket(
            &Config::find_program_address(&jito_restaking_program::id()).0,
            &ncn_root.ncn_pubkey,
            vault,
            &ncn_vault_ticket,
            &ncn_root.ncn_admin,
        )
        .await
    }

    /// Sends a transaction to warm up an NcnVaultTicket.
    pub async fn warmup_ncn_vault_ticket(
        &mut self,
        config: &Pubkey,
        ncn: &Pubkey,
        vault: &Pubkey,
        ncn_vault_ticket: &Pubkey,
        admin: &Keypair,
    ) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;

        self.process_transaction(&Transaction::new_signed_with_payer(
            &[warmup_ncn_vault_ticket(
                &jito_restaking_program::id(),
                config,
                ncn,
                vault,
                ncn_vault_ticket,
                &admin.pubkey(),
            )],
            Some(&self.payer.pubkey()),
            &[admin, &self.payer],
            blockhash,
        ))
        .await
    }

    /// Cools down an NcnVaultTicket, starting the deactivation process.
    #[allow(dead_code)]
    pub async fn do_cooldown_ncn_vault_ticket(
        &mut self,
        ncn_root: &NcnRoot,
        vault: &Pubkey,
    ) -> TestResult<()> {
        let ncn_vault_ticket = NcnVaultTicket::find_program_address(
            &jito_restaking_program::id(),
            &ncn_root.ncn_pubkey,
            vault,
        )
        .0;
        self.cooldown_ncn_vault_ticket(
            &Config::find_program_address(&jito_restaking_program::id()).0,
            &ncn_root.ncn_pubkey,
            vault,
            &ncn_vault_ticket,
            &ncn_root.ncn_admin,
        )
        .await
    }

    /// Sends a transaction to cool down an NcnVaultTicket.
    #[allow(dead_code)]
    pub async fn cooldown_ncn_vault_ticket(
        &mut self,
        config: &Pubkey,
        ncn: &Pubkey,
        vault: &Pubkey,
        ncn_vault_ticket: &Pubkey,
        admin: &Keypair,
    ) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;

        self.process_transaction(&Transaction::new_signed_with_payer(
            &[cooldown_ncn_vault_ticket(
                &jito_restaking_program::id(),
                config,
                ncn,
                vault,
                ncn_vault_ticket,
                &admin.pubkey(),
            )],
            Some(&self.payer.pubkey()),
            &[admin, &self.payer],
            blockhash,
        ))
        .await
    }

    /// Warms up the NCN-Operator link from the NCN's perspective.
    pub async fn do_ncn_warmup_operator(
        &mut self,
        ncn_root: &NcnRoot,
        operator_pubkey: &Pubkey,
    ) -> TestResult<()> {
        self.ncn_warmup_operator(
            &Config::find_program_address(&jito_restaking_program::id()).0,
            &ncn_root.ncn_pubkey,
            operator_pubkey,
            &NcnOperatorState::find_program_address(
                &jito_restaking_program::id(),
                &ncn_root.ncn_pubkey,
                operator_pubkey,
            )
            .0,
            &ncn_root.ncn_admin,
        )
        .await
    }

    /// Cools down the NCN-Operator link from the NCN's perspective.
    #[allow(dead_code)]
    pub async fn do_ncn_cooldown_operator(
        &mut self,
        ncn_root: &NcnRoot,
        operator_pubkey: &Pubkey,
    ) -> TestResult<()> {
        self.ncn_cooldown_operator(
            &Config::find_program_address(&jito_restaking_program::id()).0,
            &ncn_root.ncn_pubkey,
            operator_pubkey,
            &NcnOperatorState::find_program_address(
                &jito_restaking_program::id(),
                &ncn_root.ncn_pubkey,
                operator_pubkey,
            )
            .0,
            &ncn_root.ncn_admin,
        )
        .await
    }

    /// Sends a transaction to cool down the NCN-Operator link (NCN admin initiated).
    pub async fn ncn_cooldown_operator(
        &mut self,
        config: &Pubkey,
        ncn: &Pubkey,
        operator_pubkey: &Pubkey,
        ncn_operator_state: &Pubkey,
        admin: &Keypair,
    ) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;

        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ncn_cooldown_operator(
                &jito_restaking_program::id(),
                config,
                ncn,
                operator_pubkey,
                ncn_operator_state,
                &admin.pubkey(),
            )],
            Some(&self.payer.pubkey()),
            &[admin, &self.payer],
            blockhash,
        ))
        .await
    }

    /// Sends a transaction to warm up the NCN-Operator link (NCN admin initiated).
    pub async fn ncn_warmup_operator(
        &mut self,
        config: &Pubkey,
        ncn: &Pubkey,
        operator_pubkey: &Pubkey,
        ncn_operator_state: &Pubkey,
        admin: &Keypair,
    ) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;

        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ncn_warmup_operator(
                &jito_restaking_program::id(),
                config,
                ncn,
                operator_pubkey,
                ncn_operator_state,
                &admin.pubkey(),
            )],
            Some(&self.payer.pubkey()),
            &[admin, &self.payer],
            blockhash,
        ))
        .await
    }

    /// Warms up the NCN-Operator link from the Operator's perspective.
    pub async fn do_operator_warmup_ncn(
        &mut self,
        operator_root: &OperatorRoot,
        ncn_pubkey: &Pubkey,
    ) -> TestResult<()> {
        self.operator_warmup_ncn(
            &Config::find_program_address(&jito_restaking_program::id()).0,
            ncn_pubkey,
            &operator_root.operator_pubkey,
            &NcnOperatorState::find_program_address(
                &jito_restaking_program::id(),
                ncn_pubkey,
                &operator_root.operator_pubkey,
            )
            .0,
            &operator_root.operator_admin,
        )
        .await
    }

    /// Sends a transaction to warm up the NCN-Operator link (Operator admin initiated).
    pub async fn operator_warmup_ncn(
        &mut self,
        config: &Pubkey,
        ncn_pubkey: &Pubkey,
        operator_pubkey: &Pubkey,
        ncn_operator_state: &Pubkey,
        admin: &Keypair,
    ) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;

        self.process_transaction(&Transaction::new_signed_with_payer(
            &[operator_warmup_ncn(
                &jito_restaking_program::id(),
                config,
                ncn_pubkey,
                operator_pubkey,
                ncn_operator_state,
                &admin.pubkey(),
            )],
            Some(&self.payer.pubkey()),
            &[admin, &self.payer],
            blockhash,
        ))
        .await
    }

    /// Cools down the NCN-Operator link from the Operator's perspective.
    #[allow(dead_code)]
    pub async fn do_operator_cooldown_ncn(
        &mut self,
        operator_root: &OperatorRoot,
        ncn_pubkey: &Pubkey,
    ) -> TestResult<()> {
        self.operator_cooldown_ncn(
            &Config::find_program_address(&jito_restaking_program::id()).0,
            ncn_pubkey,
            &operator_root.operator_pubkey,
            &NcnOperatorState::find_program_address(
                &jito_restaking_program::id(),
                ncn_pubkey,
                &operator_root.operator_pubkey,
            )
            .0,
            &operator_root.operator_admin,
        )
        .await
    }

    /// Sends a transaction to cool down the NCN-Operator link (Operator admin initiated).
    pub async fn operator_cooldown_ncn(
        &mut self,
        config: &Pubkey,
        ncn_pubkey: &Pubkey,
        operator_pubkey: &Pubkey,
        ncn_operator_state: &Pubkey,
        admin: &Keypair,
    ) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;

        self.process_transaction(&Transaction::new_signed_with_payer(
            &[operator_cooldown_ncn(
                &jito_restaking_program::id(),
                config,
                ncn_pubkey,
                operator_pubkey,
                ncn_operator_state,
                &admin.pubkey(),
            )],
            Some(&self.payer.pubkey()),
            &[admin, &self.payer],
            blockhash,
        ))
        .await
    }

    /// Initializes an NcnOperatorState account, linking an NCN and an operator.
    pub async fn do_initialize_ncn_operator_state(
        &mut self,
        ncn_root: &NcnRoot,
        operator: &Pubkey,
    ) -> TestResult<()> {
        let ncn_operator_state = NcnOperatorState::find_program_address(
            &jito_restaking_program::id(),
            &ncn_root.ncn_pubkey,
            operator,
        )
        .0;

        self.initialize_ncn_operator_state(
            &Config::find_program_address(&jito_restaking_program::id()).0,
            &ncn_root.ncn_pubkey,
            operator,
            &ncn_operator_state,
            &ncn_root.ncn_admin,
            &self.payer.insecure_clone(),
        )
        .await
    }

    /// Initializes an NcnVaultSlasherTicket account, linking an NCN, vault, and slasher.
    #[allow(dead_code)]
    pub async fn do_initialize_ncn_vault_slasher_ticket(
        &mut self,
        ncn_root: &NcnRoot,
        vault: &Pubkey,
        slasher: &Pubkey,
        max_slash_amount: u64,
    ) -> TestResult<()> {
        let ncn_vault_ticket = NcnVaultTicket::find_program_address(
            &jito_restaking_program::id(),
            &ncn_root.ncn_pubkey,
            vault,
        )
        .0;
        let ncn_slasher_ticket = NcnVaultSlasherTicket::find_program_address(
            &jito_restaking_program::id(),
            &ncn_root.ncn_pubkey,
            vault,
            slasher,
        )
        .0;

        self.initialize_ncn_vault_slasher_ticket(
            &Config::find_program_address(&jito_restaking_program::id()).0,
            &ncn_root.ncn_pubkey,
            vault,
            slasher,
            &ncn_vault_ticket,
            &ncn_slasher_ticket,
            &ncn_root.ncn_admin,
            &self.payer.insecure_clone(),
            max_slash_amount,
        )
        .await
    }

    /// Warms up an NcnVaultSlasherTicket.
    #[allow(dead_code)]
    pub async fn do_warmup_ncn_vault_slasher_ticket(
        &mut self,
        ncn_root: &NcnRoot,
        vault: &Pubkey,
        slasher: &Pubkey,
    ) -> TestResult<()> {
        let ncn_vault_ticket = NcnVaultTicket::find_program_address(
            &jito_restaking_program::id(),
            &ncn_root.ncn_pubkey,
            vault,
        )
        .0;
        let ncn_slasher_ticket = NcnVaultSlasherTicket::find_program_address(
            &jito_restaking_program::id(),
            &ncn_root.ncn_pubkey,
            vault,
            slasher,
        )
        .0;

        self.warmup_ncn_vault_slasher_ticket(
            &Config::find_program_address(&jito_restaking_program::id()).0,
            &ncn_root.ncn_pubkey,
            vault,
            slasher,
            &ncn_vault_ticket,
            &ncn_slasher_ticket,
            &ncn_root.ncn_admin,
        )
        .await
    }

    /// Sends a transaction to warm up an NcnVaultSlasherTicket.
    #[allow(clippy::too_many_arguments)]
    pub async fn warmup_ncn_vault_slasher_ticket(
        &mut self,
        config: &Pubkey,
        ncn: &Pubkey,
        vault: &Pubkey,
        slasher: &Pubkey,
        ncn_vault_ticket: &Pubkey,
        ncn_slasher_ticket: &Pubkey,
        admin: &Keypair,
    ) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;

        self.process_transaction(&Transaction::new_signed_with_payer(
            &[warmup_ncn_vault_slasher_ticket(
                &jito_restaking_program::id(),
                config,
                ncn,
                vault,
                slasher,
                ncn_vault_ticket,
                ncn_slasher_ticket,
                &admin.pubkey(),
            )],
            Some(&self.payer.pubkey()),
            &[admin, &self.payer],
            blockhash,
        ))
        .await
    }

    /// Sends a transaction to initialize an NCN account.
    pub async fn initialize_ncn(
        &mut self,
        config: &Pubkey,
        ncn: &Pubkey,
        ncn_admin: &Keypair,
        ncn_base: &Keypair,
    ) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;

        self.process_transaction(&Transaction::new_signed_with_payer(
            &[initialize_ncn(
                &jito_restaking_program::id(),
                config,
                ncn,
                &ncn_admin.pubkey(),
                &ncn_base.pubkey(),
            )],
            Some(&ncn_admin.pubkey()),
            &[&ncn_admin, &ncn_base],
            blockhash,
        ))
        .await
    }

    /// Sends a transaction to initialize an NcnVaultTicket account.
    pub async fn initialize_ncn_vault_ticket(
        &mut self,
        config: &Pubkey,
        ncn: &Pubkey,
        vault: &Pubkey,
        ncn_vault_ticket: &Pubkey,
        ncn_admin: &Keypair,
        payer: &Keypair,
    ) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;

        self.process_transaction(&Transaction::new_signed_with_payer(
            &[initialize_ncn_vault_ticket(
                &jito_restaking_program::id(),
                config,
                ncn,
                vault,
                ncn_vault_ticket,
                &ncn_admin.pubkey(),
                &payer.pubkey(),
            )],
            Some(&payer.pubkey()),
            &[ncn_admin, payer],
            blockhash,
        ))
        .await
    }

    /// Sends a transaction to initialize an NcnOperatorState account.
    pub async fn initialize_ncn_operator_state(
        &mut self,
        config: &Pubkey,
        ncn: &Pubkey,
        operator: &Pubkey,
        ncn_operator_state: &Pubkey,
        ncn_admin: &Keypair,
        payer: &Keypair,
    ) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;

        self.process_transaction(&Transaction::new_signed_with_payer(
            &[initialize_ncn_operator_state(
                &jito_restaking_program::id(),
                config,
                ncn,
                operator,
                ncn_operator_state,
                &ncn_admin.pubkey(),
                &payer.pubkey(),
            )],
            Some(&payer.pubkey()),
            &[ncn_admin, payer],
            blockhash,
        ))
        .await
    }

    /// Sends a transaction to initialize an NcnVaultSlasherTicket account.
    #[allow(clippy::too_many_arguments)]
    pub async fn initialize_ncn_vault_slasher_ticket(
        &mut self,
        config: &Pubkey,
        ncn: &Pubkey,
        vault: &Pubkey,
        slasher: &Pubkey,
        ncn_vault_ticket: &Pubkey,
        ncn_slasher_ticket: &Pubkey,
        ncn_admin: &Keypair,
        payer: &Keypair,
        max_slash_amount: u64,
    ) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;

        self.process_transaction(&Transaction::new_signed_with_payer(
            &[initialize_ncn_vault_slasher_ticket(
                &jito_restaking_program::id(),
                config,
                ncn,
                vault,
                slasher,
                ncn_vault_ticket,
                ncn_slasher_ticket,
                &ncn_admin.pubkey(),
                &payer.pubkey(),
                max_slash_amount,
            )],
            Some(&payer.pubkey()),
            &[ncn_admin, payer],
            blockhash,
        ))
        .await
    }

    /// Sends a transaction to change the admin of an NCN account.
    #[allow(dead_code)]
    pub async fn ncn_set_admin(
        &mut self,
        ncn: &Pubkey,
        old_admin: &Keypair,
        new_admin: &Keypair,
    ) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;

        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ncn_set_admin(
                &jito_restaking_program::id(),
                ncn,
                &old_admin.pubkey(),
                &new_admin.pubkey(),
            )],
            Some(&old_admin.pubkey()),
            &[old_admin, new_admin],
            blockhash,
        ))
        .await
    }

    /// Sends a transaction to change the admin of an Operator account.
    #[allow(dead_code)]
    pub async fn operator_set_admin(
        &mut self,
        operator: &Pubkey,
        old_admin: &Keypair,
        new_admin: &Keypair,
    ) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;

        self.process_transaction(&Transaction::new_signed_with_payer(
            &[operator_set_admin(
                &jito_restaking_program::id(),
                operator,
                &old_admin.pubkey(),
                &new_admin.pubkey(),
            )],
            Some(&old_admin.pubkey()),
            &[old_admin, new_admin],
            blockhash,
        ))
        .await
    }

    /// Sends a transaction to set a secondary admin for an Operator account with a specific role.
    #[allow(dead_code)]
    pub async fn operator_set_secondary_admin(
        &mut self,
        operator: &Pubkey,
        old_admin: &Keypair,
        new_admin: &Keypair,
        operator_admin_role: OperatorAdminRole,
    ) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;

        self.process_transaction(&Transaction::new_signed_with_payer(
            &[operator_set_secondary_admin(
                &jito_restaking_program::id(),
                operator,
                &old_admin.pubkey(),
                &new_admin.pubkey(),
                operator_admin_role,
            )],
            Some(&old_admin.pubkey()),
            &[old_admin],
            blockhash,
        ))
        .await
    }

    /// Sends a transaction to initialize an Operator account.
    pub async fn initialize_operator(
        &mut self,
        config: &Pubkey,
        operator: &Pubkey,
        admin: &Keypair,
        base: &Keypair,
        operator_fee_bps: u16,
    ) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;

        self.process_transaction(&Transaction::new_signed_with_payer(
            &[initialize_operator(
                &jito_restaking_program::id(),
                config,
                operator,
                &admin.pubkey(),
                &base.pubkey(),
                operator_fee_bps,
            )],
            Some(&admin.pubkey()),
            &[admin, base],
            blockhash,
        ))
        .await
    }

    /// Sends a transaction to initialize an OperatorVaultTicket account.
    pub async fn initialize_operator_vault_ticket(
        &mut self,
        config: &Pubkey,
        operator: &Pubkey,
        vault: &Pubkey,
        operator_vault_ticket: &Pubkey,
        admin: &Keypair,
        payer: &Keypair,
    ) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;

        self.process_transaction(&Transaction::new_signed_with_payer(
            &[initialize_operator_vault_ticket(
                &jito_restaking_program::id(),
                config,
                operator,
                vault,
                operator_vault_ticket,
                &admin.pubkey(),
                &payer.pubkey(),
            )],
            Some(&payer.pubkey()),
            &[admin, payer],
            blockhash,
        ))
        .await
    }

    /// Sends a transaction to set the fee for an Operator account.
    #[allow(dead_code)]
    pub async fn operator_set_fee(
        &mut self,
        config: &Pubkey,
        operator: &Pubkey,
        admin: &Keypair,
        new_fee_bps: u16,
    ) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;

        self.process_transaction(&Transaction::new_signed_with_payer(
            &[operator_set_fee(
                &jito_restaking_program::id(),
                config,
                operator,
                &admin.pubkey(),
                new_fee_bps,
            )],
            Some(&self.payer.pubkey()),
            &[admin, &self.payer],
            blockhash,
        ))
        .await
    }

    /// Delegates token account authority from an NCN.
    #[allow(dead_code)]
    pub async fn ncn_delegate_token_account(
        &mut self,
        ncn_pubkey: &Pubkey,
        delegate_admin: &Keypair,
        token_mint: &Pubkey,
        token_account: &Pubkey,
        delegate: &Pubkey,
        token_program_id: &Pubkey,
    ) -> Result<(), TestError> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[jito_restaking_sdk::sdk::ncn_delegate_token_account(
                &jito_restaking_program::id(),
                ncn_pubkey,
                &delegate_admin.pubkey(),
                token_mint,
                token_account,
                delegate,
                token_program_id,
            )],
            Some(&self.payer.pubkey()),
            &[&self.payer, delegate_admin],
            blockhash,
        ))
        .await
    }

    /// Delegates token account authority from an Operator.
    #[allow(dead_code)]
    pub async fn operator_delegate_token_account(
        &mut self,
        operator_pubkey: &Pubkey,
        delegate_admin: &Keypair,
        token_mint: &Pubkey,
        token_account: &Pubkey,
        delegate: &Pubkey,
        token_program_id: &Pubkey,
    ) -> Result<(), TestError> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[jito_restaking_sdk::sdk::operator_delegate_token_account(
                &jito_restaking_program::id(),
                operator_pubkey,
                &delegate_admin.pubkey(),
                token_mint,
                token_account,
                delegate,
                token_program_id,
            )],
            Some(&self.payer.pubkey()),
            &[&self.payer, delegate_admin],
            blockhash,
        ))
        .await
    }

    /// Processes a transaction using the BanksClient.
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

    /// Sets the admin for the Restaking program config.
    #[allow(dead_code)]
    pub async fn set_config_admin(
        &mut self,
        config: &Pubkey,
        old_admin: &Keypair,
        new_admin: &Keypair,
    ) -> Result<(), TestError> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[set_config_admin(
                &jito_restaking_program::id(),
                config,
                &old_admin.pubkey(),
                &new_admin.pubkey(),
            )],
            Some(&old_admin.pubkey()),
            &[old_admin],
            blockhash,
        ))
        .await
    }
}

/// Asserts that a TestResult contains a specific RestakingError.
#[track_caller]
#[inline(always)]
#[allow(dead_code)]
pub fn assert_restaking_error<T>(
    test_error: Result<T, TestError>,
    restaking_error: RestakingError,
) {
    assert!(test_error.is_err());
    assert_eq!(
        test_error.err().unwrap().to_transaction_error().unwrap(),
        TransactionError::InstructionError(0, InstructionError::Custom(restaking_error as u32))
    );
}
