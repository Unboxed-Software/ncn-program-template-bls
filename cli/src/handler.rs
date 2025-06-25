#![allow(clippy::integer_division)]
use std::{collections::HashMap, mem::size_of, str::FromStr};

use crate::{
    args::{Args, ProgramCommand},
    getters::{
        get_account_payer, get_all_operators_in_ncn, get_all_tickets, get_all_vaults,
        get_all_vaults_in_ncn, get_ballot_box, get_consensus_result, get_current_slot,
        get_epoch_snapshot, get_epoch_state, get_is_epoch_completed, get_ncn,
        get_ncn_operator_state, get_ncn_program_config, get_ncn_reward_receiver,
        get_ncn_reward_router, get_ncn_vault_ticket, get_operator_snapshot,
        get_operator_vault_reward_router, get_total_epoch_rent_cost, get_vault_ncn_ticket,
        get_vault_operator_delegation, get_vault_registry, get_weight_table,
    },
    instructions::{
        admin_create_config, admin_fund_account_payer, admin_register_st_mint, admin_set_new_admin,
        admin_set_parameters, admin_set_tie_breaker, admin_set_weight, crank_close_epoch_accounts,
        crank_distribute, crank_register_vaults, crank_snapshot, create_ballot_box,
        create_epoch_snapshot, create_epoch_state, create_ncn_reward_router,
        create_operator_snapshot, create_operator_vault_reward_router, create_vault_registry,
        create_weight_table, distribute_operator_vault_rewards, full_vault_update,
        operator_cast_vote, register_vault, route_ncn_rewards, route_operator_vault_rewards,
        set_epoch_weights, snapshot_vault_operator_delegation, update_all_vaults_in_network,
    },
    keeper::keeper_loop::startup_ncn_keeper,
    operator::operator_loop::startup_operator_loop,
};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine};
use log::info;
use ncn_program_core::account_payer::AccountPayer;
use solana_account_decoder::{UiAccountEncoding, UiDataSliceConfig};
use solana_client::{
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    native_token::lamports_to_sol,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
};

pub struct CliHandler {
    pub rpc_url: String,
    pub commitment: CommitmentConfig,
    pub keypair: Option<Keypair>,
    pub restaking_program_id: Pubkey,
    pub vault_program_id: Pubkey,
    pub ncn_program_id: Pubkey,
    pub token_program_id: Pubkey,
    pub ncn: Option<Pubkey>,
    pub epoch: u64,
    pub rpc_client: RpcClient,
    pub retries: u64,
    pub priority_fee_micro_lamports: u64,
    pub open_weather_api_key: Option<String>,
}

impl CliHandler {
    pub async fn from_args(args: &Args) -> Result<Self> {
        let rpc_url = args.rpc_url.clone();
        CommitmentConfig::confirmed();

        let commitment = CommitmentConfig::from_str(&args.commitment)?;

        let keypair = match &args.keypair_path {
            Some(path) => Some(
                read_keypair_file(path)
                    .map_err(|e| anyhow!("Failed to read keypair file: {}", e))?,
            ),
            None => None,
        };

        let restaking_program_id = Pubkey::from_str(&args.restaking_program_id)?;

        let vault_program_id = Pubkey::from_str(&args.vault_program_id)?;

        let ncn_program_id = Pubkey::from_str(&args.ncn_program_id)?;

        let token_program_id = Pubkey::from_str(&args.token_program_id)?;

        let open_weather_api_key = args.open_weather_api_key.clone();

        let ncn = args
            .ncn
            .clone()
            .map(|id| Pubkey::from_str(&id))
            .transpose()?;

        let rpc_client = RpcClient::new_with_commitment(rpc_url.clone(), commitment);

        let mut handler = Self {
            rpc_url,
            commitment,
            keypair,
            restaking_program_id,
            vault_program_id,
            ncn_program_id,
            token_program_id,
            ncn,
            epoch: u64::MAX,
            rpc_client,
            retries: args.transaction_retries,
            priority_fee_micro_lamports: args.priority_fee_micro_lamports,
            open_weather_api_key,
        };

        handler.epoch = {
            if let Some(epoch) = args.epoch {
                epoch
            } else {
                let client = handler.rpc_client();
                let epoch_info = client.get_epoch_info().await?;
                epoch_info.epoch
            }
        };

        Ok(handler)
    }

    pub const fn rpc_client(&self) -> &RpcClient {
        &self.rpc_client
    }

    pub fn get_rpc_program_accounts_with_config<T: jito_bytemuck::Discriminator>(
        &self,
        account_pubkey: &Pubkey,
    ) -> anyhow::Result<RpcProgramAccountsConfig> {
        let data_size = size_of::<T>() + 8;
        let encoded_discriminator = general_purpose::STANDARD.encode(account_pubkey.to_bytes());
        let size_filter = RpcFilterType::DataSize(data_size as u64);
        let ncn_filter = RpcFilterType::Memcmp(Memcmp::new(
            8,                                                 // offset
            MemcmpEncodedBytes::Base64(encoded_discriminator), // encoded bytes
        ));

        let config = RpcProgramAccountsConfig {
            filters: Some(vec![size_filter, ncn_filter]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                data_slice: Some(UiDataSliceConfig {
                    offset: 0,
                    length: data_size,
                }),
                commitment: Some(self.commitment),
                min_context_slot: None,
            },
            with_context: Some(false),
            sort_results: Some(false),
        };

        Ok(config)
    }

    pub fn open_weather_api_key(&self) -> Result<String> {
        self.open_weather_api_key.clone().ok_or_else(|| {
            anyhow!("No Open Weather API key provided. Set the OPENWEATHER_API_KEY environment variable or pass it as an argument.")
        })
    }

    pub fn keypair(&self) -> Result<&Keypair> {
        self.keypair.as_ref().ok_or_else(|| anyhow!("No keypair"))
    }

    pub fn ncn(&self) -> Result<&Pubkey> {
        self.ncn.as_ref().ok_or_else(|| anyhow!("No NCN address"))
    }

    #[allow(clippy::large_stack_frames)]
    pub async fn handle(&self, action: ProgramCommand) -> Result<()> {
        match action {
            // Keepers
            // Ncn Keeper
            ProgramCommand::RunKeeper {
                loop_timeout_ms,
                error_timeout_ms,
            } => startup_ncn_keeper(self, loop_timeout_ms, error_timeout_ms).await,

            // Operator Keeper
            ProgramCommand::RunOperator {
                loop_timeout_ms,
                error_timeout_ms,
                operator,
            } => {
                let operator = Pubkey::from_str(&operator)
                    .map_err(|e| anyhow!("Error parsing operator: {}", e))?;
                startup_operator_loop(self, loop_timeout_ms, error_timeout_ms, operator).await
            }
            // Cranks
            ProgramCommand::CrankRegisterVaults {} => crank_register_vaults(self).await,
            ProgramCommand::CrankUpdateAllVaults {} => update_all_vaults_in_network(self).await,
            ProgramCommand::CrankDistribute {} => crank_distribute(self, self.epoch).await,

            ProgramCommand::CrankSnapshot {} => crank_snapshot(self, self.epoch).await,
            ProgramCommand::CrankCloseEpochAccounts {} => {
                crank_close_epoch_accounts(self, self.epoch).await
            }

            ProgramCommand::SetEpochWeights {} => set_epoch_weights(self, self.epoch).await,

            // Admin
            ProgramCommand::AdminCreateConfig {
                ncn_fee_wallet,
                ncn_fee_bps,
                epochs_before_stall,
                valid_slots_after_consensus,
                epochs_after_consensus_before_close,
                tie_breaker_admin,
            } => {
                let tie_breaker = if let Some(admin_str) = tie_breaker_admin {
                    Some(
                        Pubkey::from_str(&admin_str)
                            .map_err(|e| anyhow!("Error parsing tie breaker admin: {}", e))?,
                    )
                } else {
                    None
                };

                let ncn_fee_wallet = Pubkey::from_str(&ncn_fee_wallet)
                    .map_err(|e| anyhow!("Error parsing NCN fee wallet: {}", e))?;

                admin_create_config(
                    self,
                    ncn_fee_wallet,
                    ncn_fee_bps.try_into()?,
                    tie_breaker,
                    epochs_before_stall,
                    valid_slots_after_consensus,
                    epochs_after_consensus_before_close,
                )
                .await
            }
            ProgramCommand::AdminRegisterStMint { vault, weight } => {
                let vault =
                    Pubkey::from_str(&vault).map_err(|e| anyhow!("Error parsing vault: {}", e))?;
                admin_register_st_mint(self, &vault, weight).await
            }
            ProgramCommand::AdminSetWeight { vault, weight } => {
                let vault =
                    Pubkey::from_str(&vault).map_err(|e| anyhow!("Error parsing vault: {}", e))?;
                admin_set_weight(self, &vault, self.epoch, weight).await
            }
            ProgramCommand::AdminSetTieBreaker { weather_status } => {
                admin_set_tie_breaker(self, self.epoch, weather_status).await
            }
            ProgramCommand::AdminSetParameters {
                epochs_before_stall,
                epochs_after_consensus_before_close,
                valid_slots_after_consensus,
                starting_valid_epoch,
            } => {
                admin_set_parameters(
                    self,
                    epochs_before_stall,
                    epochs_after_consensus_before_close,
                    valid_slots_after_consensus,
                    starting_valid_epoch,
                )
                .await?;
                let config = get_ncn_program_config(self).await?;
                info!("\n\n--- Parameters Set ---\nepochs_before_stall: {}\nepochs_after_consensus_before_close: {}\nvalid_slots_after_consensus: {}\nstarting_valid_epoch: {}\n",
                    config.epochs_before_stall(),
                    config.epochs_after_consensus_before_close(),
                    config.valid_slots_after_consensus(),
                    config.starting_valid_epoch()
                );

                Ok(())
            }
            ProgramCommand::AdminSetNewAdmin {
                new_admin,
                set_tie_breaker_admin,
            } => {
                let new_admin = Pubkey::from_str(&new_admin)
                    .map_err(|e| anyhow!("Error parsing new admin: {}", e))?;
                admin_set_new_admin(self, &new_admin, set_tie_breaker_admin).await
            }
            ProgramCommand::AdminFundAccountPayer { amount_in_sol } => {
                admin_fund_account_payer(self, amount_in_sol).await
            }

            // Instructions
            ProgramCommand::CreateVaultRegistry {} => create_vault_registry(self).await,

            ProgramCommand::RegisterVault { vault } => {
                let vault =
                    Pubkey::from_str(&vault).map_err(|e| anyhow!("Error parsing vault: {}", e))?;
                register_vault(self, &vault).await
            }

            ProgramCommand::CreateEpochState {} => create_epoch_state(self, self.epoch).await,

            ProgramCommand::CreateWeightTable {} => create_weight_table(self, self.epoch).await,

            ProgramCommand::CreateEpochSnapshot {} => create_epoch_snapshot(self, self.epoch).await,
            ProgramCommand::CreateOperatorSnapshot { operator } => {
                let operator = Pubkey::from_str(&operator)
                    .map_err(|e| anyhow!("Error parsing operator: {}", e))?;
                create_operator_snapshot(self, &operator, self.epoch).await
            }
            ProgramCommand::SnapshotVaultOperatorDelegation { vault, operator } => {
                let vault =
                    Pubkey::from_str(&vault).map_err(|e| anyhow!("Error parsing vault: {}", e))?;
                let operator = Pubkey::from_str(&operator)
                    .map_err(|e| anyhow!("Error parsing operator: {}", e))?;
                snapshot_vault_operator_delegation(self, &vault, &operator, self.epoch).await
            }

            ProgramCommand::CreateBallotBox {} => create_ballot_box(self, self.epoch).await,
            ProgramCommand::OperatorCastVote {
                operator,
                weather_status,
            } => {
                let operator = Pubkey::from_str(&operator)
                    .map_err(|e| anyhow!("Error parsing operator: {}", e))?;

                operator_cast_vote(self, &operator, self.epoch, weather_status).await
            }

            // Getters
            ProgramCommand::GetNcn {} => {
                let ncn = get_ncn(self).await?;
                info!("NCN: {:?}", ncn);
                Ok(())
            }
            ProgramCommand::GetNcnOperatorState { operator } => {
                let operator = Pubkey::from_str(&operator)
                    .map_err(|e| anyhow!("Error parsing operator: {}", e))?;
                let ncn_operator_state = get_ncn_operator_state(self, &operator).await?;
                info!("NCN Operator State: {:?}", ncn_operator_state);
                Ok(())
            }
            ProgramCommand::GetVaultNcnTicket { vault } => {
                let vault =
                    Pubkey::from_str(&vault).map_err(|e| anyhow!("Error parsing vault: {}", e))?;
                let ncn_ticket = get_vault_ncn_ticket(self, &vault).await?;
                info!("Vault NCN Ticket: {:?}", ncn_ticket);
                Ok(())
            }
            ProgramCommand::GetNcnVaultTicket { vault } => {
                let vault =
                    Pubkey::from_str(&vault).map_err(|e| anyhow!("Error parsing vault: {}", e))?;
                let ncn_ticket = get_ncn_vault_ticket(self, &vault).await?;
                info!("NCN Vault Ticket: {:?}", ncn_ticket);
                Ok(())
            }
            ProgramCommand::GetVaultOperatorDelegation { vault, operator } => {
                let vault =
                    Pubkey::from_str(&vault).map_err(|e| anyhow!("Error parsing vault: {}", e))?;
                let operator = Pubkey::from_str(&operator)
                    .map_err(|e| anyhow!("Error parsing operator: {}", e))?;

                let vault_operator_delegation =
                    get_vault_operator_delegation(self, &vault, &operator).await?;

                info!("Vault Operator Delegation: {:?}", vault_operator_delegation);
                Ok(())
            }
            ProgramCommand::GetAllOperatorsInNcn {} => {
                let operators = get_all_operators_in_ncn(self).await?;

                info!("Operators: {:?}", operators);
                Ok(())
            }
            ProgramCommand::GetAllVaultsInNcn {} => {
                let vaults = get_all_vaults_in_ncn(self).await?;
                info!("Vaults: {:?}", vaults);
                Ok(())
            }
            ProgramCommand::GetAllTickets {} => {
                let all_tickets = get_all_tickets(self).await?;

                for tickets in all_tickets.iter() {
                    info!("Tickets: {}", tickets);
                }

                Ok(())
            }
            ProgramCommand::GetNCNProgramConfig {} => {
                let config = get_ncn_program_config(self).await?;
                info!("{}", config);
                Ok(())
            }
            ProgramCommand::GetVaultRegistry {} => {
                let vault_registry = get_vault_registry(self).await?;
                info!("{}", vault_registry);
                Ok(())
            }
            ProgramCommand::GetWeightTable {} => {
                let weight_table = get_weight_table(self, self.epoch).await?;
                info!("{}", weight_table);
                Ok(())
            }
            ProgramCommand::GetEpochState {} => {
                let is_epoch_complete = get_is_epoch_completed(self, self.epoch).await?;

                if is_epoch_complete {
                    info!("\n\nEpoch {} is complete", self.epoch);
                    return Ok(());
                }

                let epoch_state = get_epoch_state(self, self.epoch).await?;
                let current_slot = get_current_slot(self).await?;
                let current_state = {
                    let (valid_slots_after_consensus, epochs_after_consensus_before_close) = {
                        let config = get_ncn_program_config(self).await?;
                        (
                            config.valid_slots_after_consensus(),
                            config.epochs_after_consensus_before_close(),
                        )
                    };
                    let epoch_schedule = self.rpc_client().get_epoch_schedule().await?;

                    if epoch_state.set_weight_progress().tally() > 0 {
                        let weight_table = get_weight_table(self, self.epoch).await?;
                        epoch_state.current_state_patched(
                            &epoch_schedule,
                            valid_slots_after_consensus,
                            epochs_after_consensus_before_close,
                            weight_table.st_mint_count() as u64,
                            current_slot,
                        )
                    } else {
                        epoch_state.current_state(
                            &epoch_schedule,
                            valid_slots_after_consensus,
                            epochs_after_consensus_before_close,
                            current_slot,
                        )
                    }
                };

                info!("{}\nCurrent State: {:?}\n", epoch_state, current_state);

                Ok(())
            }
            ProgramCommand::GetEpochSnapshot {} => {
                let epoch_snapshot = get_epoch_snapshot(self, self.epoch).await?;
                info!("{}", epoch_snapshot);
                Ok(())
            }
            ProgramCommand::GetOperatorSnapshot { operator } => {
                let operator = Pubkey::from_str(&operator)
                    .map_err(|e| anyhow!("Error parsing operator: {}", e))?;
                let operator_snapshot = get_operator_snapshot(self, &operator, self.epoch).await?;
                info!("{}", operator_snapshot);
                Ok(())
            }
            ProgramCommand::GetBallotBox {} => {
                let ballot_box = get_ballot_box(self, self.epoch).await?;
                info!("{}", ballot_box);
                Ok(())
            }
            ProgramCommand::GetAccountPayer {} => {
                let account_payer = get_account_payer(self).await?;
                let (account_payer_address, _, _) =
                    AccountPayer::find_program_address(&self.ncn_program_id, self.ncn()?);
                info!(
                    "\n\n--- Account Payer ---\n{}\nBalance: {}\n",
                    account_payer_address,
                    lamports_to_sol(account_payer.lamports)
                );
                Ok(())
            }
            ProgramCommand::GetTotalEpochRentCost {} => {
                let total_epoch_rent_cost = get_total_epoch_rent_cost(self).await?;
                info!(
                    "\n\n--- Total Epoch Rent Cost ---\nCost: {}\n",
                    lamports_to_sol(total_epoch_rent_cost)
                );
                Ok(())
            }
            ProgramCommand::GetConsensusResult {} => {
                let result = get_consensus_result(self, self.epoch).await?;

                info!(
                    "\n\n--- Consensus Result for epoch {} is: \n {} ---",
                    self.epoch, result
                );
                Ok(())
            }

            ProgramCommand::GetOperatorStakes {} => {
                // Get epoch snapshot for total stake
                let epoch_snapshot = get_epoch_snapshot(self, self.epoch).await?;

                let operators = get_all_operators_in_ncn(self).await?;
                // For each fully activated operator, get their operator snapshot
                let mut operator_stakes = Vec::new();
                for operator in operators.iter() {
                    let operator_snapshot = get_operator_snapshot(self, operator, self.epoch).await;
                    if let Ok(operator_snapshot) = operator_snapshot {
                        operator_stakes
                            .push((operator, operator_snapshot.stake_weights().stake_weight()));
                    } else if let Err(e) = operator_snapshot {
                        log::warn!("Failed to get operator snapshot for {}: {}", operator, e);
                    }
                }

                // Sort operator stakes by stake weight descending
                operator_stakes.sort_by(|(_, a), (_, b)| b.cmp(a));

                for (operator, stake_weight) in operator_stakes.iter() {
                    println!(
                        "Operator: {}, Stake Weight: {}.{:02}%",
                        operator,
                        stake_weight * 10000 / epoch_snapshot.stake_weights().stake_weight() / 100,
                        stake_weight * 10000 / epoch_snapshot.stake_weights().stake_weight() % 100
                    );
                }

                Ok(())
            }

            ProgramCommand::GetVaultStakes {} => {
                let operators = get_all_operators_in_ncn(self).await?;
                let epoch_snapshot = get_epoch_snapshot(self, self.epoch).await?;
                let mut vault_stakes = HashMap::new();
                for operator in operators.iter() {
                    let operator_snapshot = get_operator_snapshot(self, operator, self.epoch).await;
                    if let Ok(operator_snapshot) = operator_snapshot {
                        for vault_operator_stake_weight in
                            operator_snapshot.vault_operator_stake_weight()
                        {
                            let vault = vault_operator_stake_weight.vault();

                            if *vault == Pubkey::default() {
                                continue;
                            }

                            let stake_weight =
                                vault_operator_stake_weight.stake_weights().stake_weight();

                            vault_stakes
                                .entry(*vault)
                                .and_modify(|w| *w += stake_weight)
                                .or_insert(stake_weight);
                        }
                    } else if let Err(e) = operator_snapshot {
                        log::warn!("Failed to get operator snapshot for {}: {}", operator, e);
                    }
                }

                let mut vault_stakes = vault_stakes.into_iter().collect::<Vec<_>>();
                vault_stakes.sort_by(|(_, a), (_, b)| b.cmp(a));

                for (vault, stake_weight) in vault_stakes.iter() {
                    println!(
                        "Vault: {}, Stake Weight: {}.{:02}%",
                        vault,
                        stake_weight * 10000 / epoch_snapshot.stake_weights().stake_weight() / 100,
                        stake_weight * 10000 / epoch_snapshot.stake_weights().stake_weight() % 100
                    );
                }

                Ok(())
            }

            ProgramCommand::GetVaultOperatorStakes {} => {
                let operators = get_all_operators_in_ncn(self).await?;
                let epoch_snapshot = get_epoch_snapshot(self, self.epoch).await?;
                let mut vault_operator_stakes: HashMap<Pubkey, HashMap<Pubkey, u128>> =
                    HashMap::new();

                // Collect stakes for each vault-operator pair
                for operator in operators.iter() {
                    let operator_snapshot = get_operator_snapshot(self, operator, self.epoch).await;
                    if let Ok(operator_snapshot) = operator_snapshot {
                        for vault_operator_stake_weight in
                            operator_snapshot.vault_operator_stake_weight()
                        {
                            let vault = vault_operator_stake_weight.vault();
                            if *vault == Pubkey::default() {
                                continue;
                            }
                            let stake_weight =
                                vault_operator_stake_weight.stake_weights().stake_weight();

                            vault_operator_stakes
                                .entry(*vault)
                                .or_default()
                                .insert(*operator, stake_weight);
                        }
                    } else if let Err(e) = operator_snapshot {
                        log::warn!("Failed to get operator snapshot for {}: {}", operator, e);
                    }
                }

                // Calculate total stake weight for percentage calculations
                let total_stake_weight = epoch_snapshot.stake_weights().stake_weight();

                // Sort vaults by total stake
                let mut vaults: Vec<_> = vault_operator_stakes.iter().collect();
                vaults.sort_by(|(_, a_ops), (_, b_ops)| {
                    let a_total: u128 = a_ops.values().sum();
                    let b_total: u128 = b_ops.values().sum();
                    b_total.cmp(&a_total)
                });

                for (vault, operator_stakes) in vaults {
                    let vault_total: u128 = operator_stakes.values().sum();
                    if vault_total == 0 {
                        continue;
                    }
                    println!(
                        "Vault: {}, % of Total Stake: {}.{:02}%",
                        vault,
                        vault_total * 10000 / total_stake_weight / 100,
                        vault_total * 10000 / total_stake_weight % 100
                    );

                    let mut operators: Vec<_> = operator_stakes.iter().collect();
                    operators.sort_by(|(_, a), (_, b)| b.cmp(a));

                    for (operator, stake) in operators {
                        if *stake == 0 {
                            continue;
                        }
                        println!(
                            "  Operator: {}, Stake: {}.{:02}%",
                            operator,
                            stake * 10000 / vault_total / 100,
                            stake * 10000 / vault_total % 100
                        );
                    }
                    println!();
                }

                Ok(())
            }
            ProgramCommand::FullUpdateVaults { vault } => {
                let mut vaults_to_update = vec![];

                if let Some(vault) = vault {
                    let vault = Pubkey::from_str(&vault)
                        .map_err(|e| anyhow!("Error parsing vault: {}", e))?;
                    vaults_to_update.push(vault);
                } else {
                    let vaults = get_all_vaults(self).await?;
                    println!("Updating {:?} Vaults", vaults.len());
                    vaults_to_update.extend(vaults.iter().cloned());
                }

                for vault in vaults_to_update.iter() {
                    println!("Updating {:?}", vault);
                    full_vault_update(self, vault).await?;
                }
                Ok(())
            }

            ProgramCommand::CreateNCNRewardRouter {} => {
                create_ncn_reward_router(self, self.epoch).await
            }

            ProgramCommand::CreateOperatorVaultRewardRouter { operator } => {
                let operator = Pubkey::from_str(&operator)
                    .map_err(|e| anyhow!("Error parsing operator: {}", e))?;
                create_operator_vault_reward_router(self, &operator, self.epoch).await
            }

            ProgramCommand::RouteNCNRewards {} => route_ncn_rewards(self, self.epoch).await,

            ProgramCommand::RouteOperatorVaultRewards { operator } => {
                let operator = Pubkey::from_str(&operator)
                    .map_err(|e| anyhow!("Error parsing operator: {}", e))?;
                route_operator_vault_rewards(self, &operator, self.epoch).await
            }

            ProgramCommand::DistributeBaseOperatorVaultRewards { operator } => {
                let operator = Pubkey::from_str(&operator)
                    .map_err(|e| anyhow!("Error parsing operator: {}", e))?;
                distribute_operator_vault_rewards(self, &operator, self.epoch).await
            }

            ProgramCommand::GetNCNRewardRouter {} => {
                let ncn_reward_router = get_ncn_reward_router(self, self.epoch).await?;
                info!("{}", ncn_reward_router);
                Ok(())
            }

            ProgramCommand::GetNCNRewardReceiverAddress {} => {
                let (address, _) = get_ncn_reward_receiver(self, self.epoch).await?;
                info!("NCN Reward Receiver Address: {:?}", address);
                Ok(())
            }

            ProgramCommand::GetOperatorVaultRewardRouter { operator } => {
                let operator = Pubkey::from_str(&operator)
                    .map_err(|e| anyhow!("Error parsing operator: {}", e))?;
                let router = get_operator_vault_reward_router(self, &operator, self.epoch).await?;
                info!("{}", router);
                Ok(())
            }

            ProgramCommand::GetAllOperatorVaultRewardRouters {} => {
                let operators = get_all_operators_in_ncn(self).await?;
                for operator in operators {
                    match get_operator_vault_reward_router(self, &operator, self.epoch).await {
                        Ok(router) => info!("Operator: {}, Router: {}", operator, router),
                        Err(e) => info!(
                            "Failed to get operator vault reward router for {:?}: {:?}",
                            operator, e
                        ),
                    }
                }
                Ok(())
            }
        }
    }
}
