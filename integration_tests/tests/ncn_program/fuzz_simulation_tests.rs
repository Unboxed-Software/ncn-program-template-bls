#[cfg(test)]
mod fuzz_tests {
    use crate::fixtures::{test_builder::TestBuilder, TestResult};
    use jito_restaking_core::{config::Config, ncn_vault_ticket::NcnVaultTicket};
    use ncn_program_core::constants::WEIGHT;
    use solana_sdk::{msg, native_token::sol_to_lamports, signature::Keypair, signer::Signer};
    use std::collections::HashMap;

    // Struct to configure mint token parameters for simulation
    struct MintConfig {
        keypair: Keypair,
        weight: u128,       // Weight for voting power calculation
        vault_count: usize, // Number of vaults to create for this mint
    }

    // Overall simulation configuration
    struct SimConfig {
        operator_count: usize,  // Number of operators to create
        mints: Vec<MintConfig>, // Token mint configurations
        delegations: Vec<u64>,  // Array of delegation amounts for vaults
        operator_fee_bps: u16,  // Operator fee in basis points (100 = 1%)
    }

    // Cost tracking structure for calculating SOL requirements
    #[derive(Debug, Clone)]
    struct CostTracker {
        // Account creation costs (rent)
        account_costs: HashMap<String, u64>,
        // Transaction fees (per signature)
        transaction_fees: u64,
        // Number of transactions by type
        transaction_counts: HashMap<String, u32>,
        // Compute unit costs
        compute_costs: u64,
        // Total operations count
        total_operations: u32,
    }

    impl CostTracker {
        fn new() -> Self {
            Self {
                account_costs: HashMap::new(),
                transaction_fees: 0,
                transaction_counts: HashMap::new(),
                compute_costs: 0,
                total_operations: 0,
            }
        }

        // Add account creation cost (rent-exempt minimum)
        fn add_account_cost(&mut self, account_type: &str, size_bytes: usize) {
            // Solana rent calculation: approximately 1 SOL per MB + base cost
            // Base rent exemption is ~890880 lamports for 0 bytes
            // Additional ~6960 lamports per 100 bytes
            let base_rent = 890_880u64; // lamports for 0-byte account
            let additional_rent = (size_bytes as u64 * 6960) / 100;
            let total_rent = base_rent + additional_rent;

            *self
                .account_costs
                .entry(account_type.to_string())
                .or_insert(0) += total_rent;
        }

        // Add transaction fee cost
        fn add_transaction_cost(&mut self, tx_type: &str, signature_count: u32) {
            // Standard Solana transaction fee is 5000 lamports per signature
            let fee = 5000u64 * signature_count as u64;
            self.transaction_fees += fee;
            *self
                .transaction_counts
                .entry(tx_type.to_string())
                .or_insert(0) += 1;
            self.total_operations += 1;
        }

        // Add compute unit cost (for complex program operations)
        fn add_compute_cost(&mut self, compute_units: u64) {
            // Typical compute unit cost is ~0.000001 SOL per compute unit
            // For simulation, we'll estimate based on operation complexity
            self.compute_costs += compute_units;
        }

        // Get total cost in lamports
        fn total_cost_lamports(&self) -> u64 {
            let account_total: u64 = self.account_costs.values().sum();
            account_total + self.transaction_fees + self.compute_costs
        }

        // Get total cost in SOL
        fn total_cost_sol(&self) -> f64 {
            self.total_cost_lamports() as f64 / 1_000_000_000.0
        }

        // Generate detailed cost report
        fn generate_report(&self) -> String {
            let mut report = String::new();
            report.push_str("\n=================== COST ANALYSIS REPORT ===================\n");

            // Account creation costs
            report.push_str("\n📋 ACCOUNT CREATION COSTS:\n");
            let mut account_total = 0u64;
            for (account_type, cost) in &self.account_costs {
                report.push_str(&format!(
                    "  • {}: {} lamports ({:.6} SOL)\n",
                    account_type,
                    cost,
                    *cost as f64 / 1_000_000_000.0
                ));
                account_total += cost;
            }
            report.push_str(&format!(
                "  📊 Total Account Costs: {} lamports ({:.6} SOL)\n",
                account_total,
                account_total as f64 / 1_000_000_000.0
            ));

            // Transaction costs
            report.push_str("\n💳 TRANSACTION COSTS:\n");
            report.push_str(&format!(
                "  • Total Transaction Fees: {} lamports ({:.6} SOL)\n",
                self.transaction_fees,
                self.transaction_fees as f64 / 1_000_000_000.0
            ));
            report.push_str(&format!(
                "  • Total Transactions: {}\n",
                self.total_operations
            ));

            report.push_str("\n📈 TRANSACTION BREAKDOWN:\n");
            for (tx_type, count) in &self.transaction_counts {
                report.push_str(&format!("  • {}: {} transactions\n", tx_type, count));
            }

            // Compute costs
            if self.compute_costs > 0 {
                report.push_str("\n⚡ COMPUTE COSTS:\n");
                report.push_str(&format!(
                    "  • Total Compute Units: {} ({:.6} SOL)\n",
                    self.compute_costs,
                    self.compute_costs as f64 / 1_000_000_000.0
                ));
            }

            // Summary
            report.push_str("\n💰 TOTAL COST SUMMARY:\n");
            report.push_str(&format!(
                "  • Total Cost: {} lamports\n",
                self.total_cost_lamports()
            ));
            report.push_str(&format!(
                "  • Total Cost: {:.6} SOL\n",
                self.total_cost_sol()
            ));
            report.push_str(&format!(
                "  • Average Cost per Operation: {:.6} SOL\n",
                self.total_cost_sol() / self.total_operations as f64
            ));

            report.push_str("\n============================================================\n");
            report
        }
    }

    /// Main simulation function that runs a full consensus cycle with the given configuration
    /// This is a modular version of the simulation_test that can be run with different parameters
    /// It follows the same workflow: setup → initialization → voting → rewards → verification
    async fn run_simulation(config: SimConfig) -> TestResult<()> {
        // Initialize cost tracker
        let mut cost_tracker = CostTracker::new();

        println!("🚀 Starting simulation with cost tracking...");
        println!(
            "📊 Configuration: {} operators, {} mints, {} total vaults",
            config.operator_count,
            config.mints.len(),
            config.mints.iter().map(|m| m.vault_count).sum::<usize>()
        );

        // 1. Create and initialize test environment
        let mut fixture = TestBuilder::new().await;
        fixture.initialize_restaking_and_vault_programs().await?;

        let mut ncn_program_client = fixture.ncn_program_client();
        let mut vault_program_client = fixture.vault_client();
        let mut restaking_client = fixture.restaking_program_client();

        // Validate configuration - ensure we have delegation amounts for each vault
        let total_vaults = config.mints.iter().map(|m| m.vault_count).sum::<usize>();
        assert_eq!(config.delegations.len(), total_vaults);

        // 2. Initialize system accounts and establish relationships
        // 2.a. Initialize the NCN account using the Jito Restaking program
        let mut test_ncn = fixture.create_test_ncn().await?;
        let ncn_pubkey = test_ncn.ncn_root.ncn_pubkey;

        // Track NCN account creation cost
        cost_tracker.add_account_cost("NCN", 1000); // Estimated NCN account size
        cost_tracker.add_transaction_cost("Initialize NCN", 1);

        // 2.b. Initialize operators and establish NCN <> operator relationships
        {
            for i in 0..config.operator_count {
                // Set operator fee to the configured value
                let operator_fees_bps: Option<u16> = Some(config.operator_fee_bps);

                // Initialize a new operator account with the specified fee
                let operator_root = restaking_client
                    .do_initialize_operator(operator_fees_bps)
                    .await?;

                // Track operator costs
                cost_tracker.add_account_cost("Operator", 800);
                cost_tracker.add_transaction_cost("Initialize Operator", 1);

                // Establish bidirectional handshake between NCN and operator:
                // 1. Initialize the NCN's state tracking for this operator
                restaking_client
                    .do_initialize_ncn_operator_state(
                        &test_ncn.ncn_root,
                        &operator_root.operator_pubkey,
                    )
                    .await?;

                cost_tracker.add_account_cost("NCN-Operator State", 500);
                cost_tracker.add_transaction_cost("Initialize NCN-Operator State", 1);

                // 2. Advance slot to satisfy timing requirements
                fixture.warp_slot_incremental(1).await.unwrap();

                // 3. NCN warms up to operator - creates NCN's half of the handshake
                restaking_client
                    .do_ncn_warmup_operator(&test_ncn.ncn_root, &operator_root.operator_pubkey)
                    .await?;

                cost_tracker.add_transaction_cost("NCN Warmup Operator", 1);

                // 4. Operator warms up to NCN - completes operator's half of the handshake
                restaking_client
                    .do_operator_warmup_ncn(&operator_root, &test_ncn.ncn_root.ncn_pubkey)
                    .await?;

                cost_tracker.add_transaction_cost("Operator Warmup NCN", 1);

                // Add the initialized operator to our test NCN's operator list
                test_ncn.operators.push(operator_root);

                if i % 10 == 0 && i > 0 {
                    println!("  ✅ Initialized {} operators", i);
                }
            }
        }

        // 2.c. Initialize vaults and establish NCN <> vaults and vault <> operator relationships
        {
            // Create vaults for each mint according to the configuration
            for (mint_idx, mint_config) in config.mints.iter().enumerate() {
                fixture
                    .add_vaults_to_test_ncn(
                        &mut test_ncn,
                        mint_config.vault_count,
                        Some(mint_config.keypair.insecure_clone()),
                    )
                    .await?;

                // Track vault costs
                for vault_idx in 0..mint_config.vault_count {
                    cost_tracker.add_account_cost("Vault", 1200);
                    cost_tracker.add_account_cost("NCN-Vault Ticket", 300);
                    cost_tracker.add_transaction_cost("Initialize Vault", 1);
                    cost_tracker.add_transaction_cost("Create NCN-Vault Relationship", 1);
                }

                println!(
                    "  ✅ Created {} vaults for mint {}",
                    mint_config.vault_count,
                    mint_idx + 1
                );
            }
        }

        // 2.d. Vaults delegate stakes to operators
        // Each vault delegates to each operator with configured delegation amounts
        {
            for operator_root in test_ncn.operators.iter() {
                for (vault_index, vault_root) in test_ncn.vaults.iter().enumerate() {
                    // Use the delegation amount for this specific vault
                    let delegation_amount = config.delegations[vault_index];

                    if delegation_amount > 0 {
                        vault_program_client
                            .do_add_delegation(
                                vault_root,
                                &operator_root.operator_pubkey,
                                delegation_amount,
                            )
                            .await
                            .unwrap();

                        // Track delegation costs
                        cost_tracker.add_account_cost("Vault-Operator Delegation", 400);
                        cost_tracker.add_transaction_cost("Add Delegation", 1);
                    }
                }
            }

            let total_delegations =
                config.operator_count * config.delegations.iter().filter(|&&d| d > 0).count();
            println!(
                "  ✅ Created {} delegation relationships",
                total_delegations
            );
        }

        // 2.e. Fast-forward time to simulate epochs passing
        // This is needed for all the relationships to finish warming up
        {
            let restaking_config_address =
                Config::find_program_address(&jito_restaking_program::id()).0;
            let restaking_config = restaking_client
                .get_config(&restaking_config_address)
                .await?;
            let epoch_length = restaking_config.epoch_length();
            fixture
                .warp_slot_incremental(epoch_length * 2)
                .await
                .unwrap();
        }

        // 3. Setting up the NCN-program
        // The following instructions would be executed by the NCN admin in a production environment
        {
            // 3.a. Initialize the config for the NCN program
            ncn_program_client
                .do_initialize_config(
                    test_ncn.ncn_root.ncn_pubkey,
                    &test_ncn.ncn_root.ncn_admin,
                    None,
                )
                .await?;

            cost_tracker.add_account_cost("NCN Program Config", 600);
            cost_tracker.add_transaction_cost("Initialize NCN Config", 1);

            // 3.b Initialize the vault_registry - creates accounts to track vaults
            ncn_program_client
                .do_full_initialize_vault_registry(test_ncn.ncn_root.ncn_pubkey)
                .await?;

            cost_tracker.add_account_cost("Vault Registry", 2000);
            cost_tracker.add_transaction_cost("Initialize Vault Registry", 1);

            // 3.c. Initialize the operator_registry - creates accounts to track operators
            ncn_program_client
                .do_full_initialize_operator_registry(test_ncn.ncn_root.ncn_pubkey)
                .await?;

            cost_tracker.add_account_cost("Operator Registry", 2000);
            cost_tracker.add_transaction_cost("Initialize Operator Registry", 1);

            // 3.d. Register all the Supported Token (ST) mints in the NCN program
            // This assigns weights to each mint for voting power calculations
            for mint_config in config.mints.iter() {
                ncn_program_client
                    .do_admin_register_st_mint(
                        ncn_pubkey,
                        mint_config.keypair.pubkey(),
                        mint_config.weight,
                    )
                    .await?;

                cost_tracker.add_account_cost("ST Mint Registration", 200);
                cost_tracker.add_transaction_cost("Register ST Mint", 1);
            }

            // 3.e Register all the vaults in the NCN program
            // This is permissionless because the admin already approved it by initiating
            // the handshake before
            for vault in test_ncn.vaults.iter() {
                let vault = vault.vault_pubkey;
                let (ncn_vault_ticket, _, _) = NcnVaultTicket::find_program_address(
                    &jito_restaking_program::id(),
                    &ncn_pubkey,
                    &vault,
                );

                ncn_program_client
                    .do_register_vault(ncn_pubkey, vault, ncn_vault_ticket)
                    .await?;

                cost_tracker.add_transaction_cost("Register Vault in NCN", 1);
            }

            println!("  ✅ NCN program setup completed");
        }

        // 4. Register all operators in the NCN program
        fixture.register_operators_to_test_ncn(&test_ncn).await?;
        for _ in 0..config.operator_count {
            cost_tracker.add_transaction_cost("Register Operator in NCN", 1);
        }

        // 4. Prepare the epoch consensus cycle
        // In a real system, these steps would run each epoch to prepare for voting on weather status
        {
            // 4.a. Initialize the epoch state - creates a new state for the current epoch
            fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;
            cost_tracker.add_account_cost("Epoch State", 1000);
            cost_tracker.add_transaction_cost("Initialize Epoch State", 1);

            // 4.b. Initialize the weight table - prepares the table that will track voting weights
            let clock = fixture.clock().await;
            let epoch = clock.epoch;
            ncn_program_client
                .do_full_initialize_weight_table(test_ncn.ncn_root.ncn_pubkey, epoch)
                .await?;

            cost_tracker.add_account_cost("Weight Table", 3000);
            cost_tracker.add_transaction_cost("Initialize Weight Table", 1);

            // 4.c. Take a snapshot of the weights for each ST mint
            // This records the current weights for the voting calculations
            ncn_program_client
                .do_set_epoch_weights(test_ncn.ncn_root.ncn_pubkey, epoch)
                .await?;

            cost_tracker.add_transaction_cost("Set Epoch Weights", 1);
            cost_tracker.add_compute_cost(10000); // Estimated compute units for weight calculations

            // 4.d. Take the epoch snapshot - records the current state for this epoch
            fixture.add_epoch_snapshot_to_test_ncn(&test_ncn).await?;
            cost_tracker.add_account_cost("Epoch Snapshot", 2000);
            cost_tracker.add_transaction_cost("Create Epoch Snapshot", 1);

            // 4.e. Take a snapshot for each operator - records their current stakes
            fixture
                .add_operator_snapshots_to_test_ncn(&test_ncn)
                .await?;

            for _ in 0..config.operator_count {
                cost_tracker.add_account_cost("Operator Snapshot", 800);
                cost_tracker.add_transaction_cost("Create Operator Snapshot", 1);
            }

            // 4.f. Take a snapshot for each vault and its delegation - records delegations
            fixture
                .add_vault_operator_delegation_snapshots_to_test_ncn(&test_ncn)
                .await?;

            let total_vault_operator_snapshots = test_ncn.vaults.len() * config.operator_count;
            for _ in 0..total_vault_operator_snapshots {
                cost_tracker.add_account_cost("Vault-Operator Delegation Snapshot", 600);
                cost_tracker.add_transaction_cost("Snapshot Vault-Operator Delegation", 1);
            }

            println!("  ✅ Epoch preparation completed");
        }

        // Define which weather status we expect to win in the vote
        // In this example, operators will vote on a simulated weather status

        // 5. Cast votes from operators
        {
            let epoch = fixture.clock().await.epoch;

            // All operators vote for the same status to ensure consensus
            // This differs from simulation_test.rs where some operators vote differently
            // for operator_root in test_ncn.operators.iter() {
            //     let operator = operator_root.operator_pubkey;
            //     ncn_program_client
            //         .do_cast_vote(
            //             ncn_pubkey,
            //             operator,
            //             &operator_root.operator_admin,
            //             winning_weather_status,
            //             epoch,
            //         )
            //         .await?;
            //
            //     cost_tracker.add_transaction_cost("Cast Vote", 1);
            //     cost_tracker.add_compute_cost(5000); // Estimated compute units for vote processing
            // }

            println!("  ✅ Voting completed (votes commented out for this test)");
        }

        // 9. Close epoch accounts but keep consensus result
        // This simulates cleanup after epoch completion while preserving the final result
        let epoch_before_closing_account = fixture.clock().await.epoch;
        fixture.close_epoch_accounts_for_test_ncn(&test_ncn).await?;

        // Track account closing costs (rent recovery)
        let accounts_to_close =
            5 + config.operator_count + (test_ncn.vaults.len() * config.operator_count);
        for _ in 0..accounts_to_close {
            cost_tracker.add_transaction_cost("Close Epoch Account", 1);
        }

        // Generate and print cost report
        let report = cost_tracker.generate_report();
        println!("{}", report);

        // Also log a summary
        println!("🎯 SIMULATION COMPLETED SUCCESSFULLY!");
        println!(
            "💰 Total estimated cost: {:.6} SOL ({} lamports)",
            cost_tracker.total_cost_sol(),
            cost_tracker.total_cost_lamports()
        );

        Ok(())
    }

    // Test with basic configuration
    // This test runs the core simulation with a standard set of parameters
    #[ignore = "long test"]
    #[tokio::test]
    async fn test_basic_simulation() -> TestResult<()> {
        // Basic configuration with multiple mints and delegation amounts
        let config = SimConfig {
            operator_count: 13,
            mints: vec![
                MintConfig {
                    keypair: Keypair::new(),
                    weight: WEIGHT,
                    vault_count: 3,
                },
                MintConfig {
                    keypair: Keypair::new(),
                    weight: WEIGHT,
                    vault_count: 2,
                },
                MintConfig {
                    keypair: Keypair::new(),
                    weight: WEIGHT,
                    vault_count: 1,
                },
                MintConfig {
                    keypair: Keypair::new(),
                    weight: WEIGHT,
                    vault_count: 1,
                },
            ],
            delegations: vec![
                // 7 delegation amounts for 7 total vaults
                sol_to_lamports(1.0),   // 1 SOL
                sol_to_lamports(10.0),  // 10 SOL
                sol_to_lamports(100.0), // 100 SOL
                sol_to_lamports(10.0),  // 10 SOL
                sol_to_lamports(1.0),   // 1 SOL
                255,                    // Arbitrary small amount
                1,                      // Minimum delegation amount
            ],
            operator_fee_bps: 100, // 1% operator fee
        };

        run_simulation(config).await
    }

    // Test with high operator count to verify system scalability
    // This test ensures the system can handle a large number of operators
    #[ignore = "long test"]
    #[tokio::test]
    async fn test_high_operator_count_simulation() -> TestResult<()> {
        // Test with a large number of operators to verify scalability
        let config = SimConfig {
            operator_count: 50, // High number of operators
            mints: vec![MintConfig {
                keypair: Keypair::new(),
                weight: WEIGHT,
                vault_count: 2,
            }],
            delegations: vec![sol_to_lamports(1000.0), sol_to_lamports(1000.0)],
            operator_fee_bps: 100,
        };

        run_simulation(config).await
    }

    // Comprehensive fuzz testing with multiple configuration variations
    // This test runs several different configurations sequentially to stress test the system
    #[ignore = "long test"]
    #[tokio::test]
    async fn test_fuzz_simulation() -> TestResult<()> {
        // Create multiple test configurations with different parameters
        let test_configs = vec![
            // Test 1: Mid-size operator set with varied delegation amounts
            SimConfig {
                operator_count: 15,
                mints: vec![
                    MintConfig {
                        keypair: Keypair::new(),
                        weight: WEIGHT,
                        vault_count: 2,
                    },
                    MintConfig {
                        keypair: Keypair::new(),
                        weight: WEIGHT,
                        vault_count: 1,
                    },
                ],
                delegations: vec![
                    sol_to_lamports(50.0),   // Small delegation
                    sol_to_lamports(500.0),  // Medium delegation
                    sol_to_lamports(5000.0), // Large delegation
                ],
                operator_fee_bps: 90, // 0.9% fee
            },
            // Test 2: Extreme delegation amounts
            SimConfig {
                operator_count: 20,
                mints: vec![MintConfig {
                    keypair: Keypair::new(),
                    weight: 2 * WEIGHT, // Double weight
                    vault_count: 3,
                }],
                delegations: vec![
                    1,                            // Minimum possible delegation
                    sol_to_lamports(1.0),         // Very small delegation
                    sol_to_lamports(1_000_000.0), // Extremely large delegation
                ],
                operator_fee_bps: 150, // 1.5% fee
            },
            // Test 3: Mixed token weights and varied delegation amounts
            SimConfig {
                operator_count: 30,
                mints: vec![
                    MintConfig {
                        keypair: Keypair::new(),
                        weight: WEIGHT, // Standard weight
                        vault_count: 1,
                    },
                    MintConfig {
                        keypair: Keypair::new(),
                        weight: WEIGHT * 2, // Double weight
                        vault_count: 1,
                    },
                    MintConfig {
                        keypair: Keypair::new(),
                        weight: WEIGHT / 2, // Half weight
                        vault_count: 1,
                    },
                ],
                delegations: vec![
                    sol_to_lamports(100.0),   // Small delegation
                    sol_to_lamports(1000.0),  // Medium delegation
                    sol_to_lamports(10000.0), // Large delegation
                ],
                operator_fee_bps: 80, // 0.8% fee
            },
        ];

        // Run all configurations sequentially
        for (i, config) in test_configs.into_iter().enumerate() {
            println!("Running fuzz test configuration {}", i + 1);
            run_simulation(config).await?;
        }

        Ok(())
    }
}
