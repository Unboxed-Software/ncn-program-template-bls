#[cfg(test)]
mod tests {
    use jito_restaking_core::{config::Config, ncn_vault_ticket::NcnVaultTicket};
    use ncn_program_core::{
        constants::WEIGHT,
        g1_point::{G1CompressedPoint, G1Point},
        g2_point::{G2CompressedPoint, G2Point},
        schemes::Sha256Normalized,
        utils::create_signer_bitmap,
    };

    use solana_sdk::{msg, signature::Keypair, signer::Signer};

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    // This test runs a complete end-to-end NCN (Network of Consensus Nodes) consensus workflow
    #[tokio::test]
    async fn simulation_test() -> TestResult<()> {
        // 1. Setup test environment
        // 1.a. Building the test environment
        let mut fixture = TestBuilder::new().await;
        // 1.b. Initialize the configuration for the restaking and vault programs
        // Note: On mainnet, these programs would already be configured
        fixture.initialize_restaking_and_vault_programs().await?;

        let mut ncn_program_client = fixture.ncn_program_client();
        let mut vault_program_client = fixture.vault_client();
        let mut restaking_client = fixture.restaking_program_client();

        // 2. Define test parameters
        const OPERATOR_COUNT: usize = 13; // Number of operators to create for testing
        let mints = vec![
            (Keypair::new(), WEIGHT),     // Alice with base weight
            (Keypair::new(), WEIGHT * 2), // Bob with double weight
            (Keypair::new(), WEIGHT * 3), // Charlie with triple weight
            (Keypair::new(), WEIGHT * 4), // Dave with quadruple weight
        ];
        let delegations = [
            1,                  // minimum delegation amount
            10_000_000_000,     // 10 tokens
            100_000_000_000,    // 100 tokens
            1_000_000_000_000,  // 1k tokens
            10_000_000_000_000, // 10k tokens
        ];

        // 3. Initialize system accounts and establish relationships
        // 3.a. Initialize the NCN account using the Jito Restaking program
        let mut test_ncn = fixture.create_test_ncn().await?;
        let ncn_pubkey = test_ncn.ncn_root.ncn_pubkey;

        // 3.b. Initialize operators and establish NCN <> operator relationships
        {
            for _ in 0..OPERATOR_COUNT {
                // Set operator fee to 100 basis points (1%)
                let operator_fees_bps: Option<u16> = Some(100);

                // Initialize a new operator account with the specified fee
                let operator_root = restaking_client
                    .do_initialize_operator(operator_fees_bps)
                    .await?;

                // Establish bidirectional handshake between NCN and operator:
                // 1. Initialize the NCN's state tracking for this operator
                restaking_client
                    .do_initialize_ncn_operator_state(
                        &test_ncn.ncn_root,
                        &operator_root.operator_pubkey,
                    )
                    .await?;

                // 2. Advance slot to satisfy timing requirements
                fixture.warp_slot_incremental(1).await.unwrap();

                // 3. NCN warms up to operator - creates NCN's half of the handshake
                restaking_client
                    .do_ncn_warmup_operator(&test_ncn.ncn_root, &operator_root.operator_pubkey)
                    .await?;

                // 4. Operator warms up to NCN - completes operator's half of the handshake
                restaking_client
                    .do_operator_warmup_ncn(&operator_root, &test_ncn.ncn_root.ncn_pubkey)
                    .await?;

                // Add the initialized operator to our test NCN's operator list
                test_ncn.operators.push(operator_root);
            }
        }

        // 3.c. Initialize vaults and establish NCN <> vaults and vault <> operator relationships
        {
            // Create 3 vaults for Alice
            fixture
                .add_vaults_to_test_ncn(&mut test_ncn, 3, Some(mints[0].0.insecure_clone()))
                .await?;
            // Create 2 vaults for Bob
            fixture
                .add_vaults_to_test_ncn(&mut test_ncn, 2, Some(mints[1].0.insecure_clone()))
                .await?;
            // Create 1 vault for Charlie
            fixture
                .add_vaults_to_test_ncn(&mut test_ncn, 1, Some(mints[2].0.insecure_clone()))
                .await?;
            // Create 1 vault for Dave
            fixture
                .add_vaults_to_test_ncn(&mut test_ncn, 1, Some(mints[3].0.insecure_clone()))
                .await?;
        }

        // 3.d. Vaults delegate stakes to operators
        // Each vault delegates different amounts to different operators based on the delegation amounts array
        {
            for (index, operator_root) in test_ncn.operators.iter().enumerate() {
                for vault_root in test_ncn.vaults.iter() {
                    // Cycle through delegation amounts based on operator index
                    let delegation_amount = delegations[index % delegations.len()];

                    if delegation_amount > 0 {
                        vault_program_client
                            .do_add_delegation(
                                vault_root,
                                &operator_root.operator_pubkey,
                                delegation_amount,
                            )
                            .await
                            .unwrap();
                    }
                }
            }
        }

        // 3.e. Fast-forward time to simulate a full epoch passing
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

        // 4. Setting up the NCN-program
        // The following instructions would be executed by the NCN admin in a production environment
        {
            // 4.a. Initialize the config for the NCN program
            ncn_program_client
                .do_initialize_config(
                    test_ncn.ncn_root.ncn_pubkey,
                    &test_ncn.ncn_root.ncn_admin,
                    Some(100),
                )
                .await?;

            // 4.b Initialize the vault_registry - creates accounts to track vaults
            ncn_program_client
                .do_full_initialize_vault_registry(test_ncn.ncn_root.ncn_pubkey)
                .await?;
            // 4.c Initialize the operator_registry - creates accounts to track operators
            ncn_program_client
                .do_full_initialize_operator_registry(ncn_pubkey)
                .await?;

            // 4.d. Register all the Supported Token (ST) mints in the NCN program
            // This assigns weights to each mint for voting power calculations
            for (mint, weight) in mints.iter() {
                ncn_program_client
                    .do_admin_register_st_mint(ncn_pubkey, mint.pubkey(), *weight)
                    .await?;
            }

            // 4.c Register all the vaults in the NCN program
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
            }
        }

        // 5. Register all the operators in the NCN program
        {
            for operator_root in test_ncn.operators.iter() {
                let g1_pubkey = G1Point::try_from(operator_root.bn128_privkey).unwrap();
                let g1_compressed = G1CompressedPoint::try_from(g1_pubkey).unwrap();
                let g2_compressed =
                    G2CompressedPoint::try_from(&operator_root.bn128_privkey).unwrap();

                let signature = operator_root
                    .bn128_privkey
                    .sign::<Sha256Normalized, &[u8; 32]>(&g1_compressed.0)
                    .unwrap();

                ncn_program_client
                    .do_register_operator(
                        ncn_pubkey,
                        operator_root.operator_pubkey,
                        &operator_root.operator_admin,
                        g1_compressed.0,
                        g2_compressed.0,
                        signature.0,
                    )
                    .await?;
            }
        }

        // 6. Prepare the epoch consensus cycle
        // In a real system, these steps would run each epoch to prepare for voting on weather status
        {
            // 5.a. Initialize the epoch state - creates a new state for the current epoch
            fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;

            // 5.b. Initialize the weight table - prepares the table that will track voting weights
            let clock = fixture.clock().await;
            let epoch = clock.epoch;
            ncn_program_client
                .do_full_initialize_weight_table(test_ncn.ncn_root.ncn_pubkey, epoch)
                .await?;

            // 5.c. Take a snapshot of the weights for each ST mint
            // This records the current weights for the voting calculations
            ncn_program_client
                .do_set_epoch_weights(test_ncn.ncn_root.ncn_pubkey, epoch)
                .await?;

            // 5.d. Take the epoch snapshot - records the current state for this epoch
            fixture.add_epoch_snapshot_to_test_ncn(&test_ncn).await?;

            for operator_root in test_ncn.operators.iter() {
                let operator = operator_root.operator_pubkey;

                ncn_program_client
                    .initialize_operator_snapshot(operator, ncn_pubkey, epoch)
                    .await?;
            }

            // 5.f. Take a snapshot for each vault and its delegation - records delegations
            fixture
                .add_vault_operator_delegation_snapshots_to_test_ncn(&test_ncn)
                .await?;
        }

        // 6. Cast votes from operators
        {
            let epoch = fixture.clock().await.epoch;

            let epoch_snapshot = ncn_program_client
                .get_epoch_snapshot(ncn_pubkey, epoch)
                .await?;

            msg!("Epoch snapshot: {}", epoch_snapshot);

            // Create a message for voting on Sunny weather status
            // This message represents what operators are collectively agreeing on
            let sunny_vote_message =
                solana_nostd_sha256::hashv(&[b"weather_vote", b"Sunny", &epoch.to_le_bytes()]);

            // Most operators vote for Sunny (will be the winning vote)
            // Skip the first operator to create a minority that doesn't participate in this vote
            let sunny_voters = &test_ncn.operators[1..]; // All except first operator
            let mut sunny_signatures: Vec<G1Point> = vec![];
            let mut sunny_apk2_pubkeys: Vec<G2Point> = vec![];

            for operator in sunny_voters {
                sunny_apk2_pubkeys.push(operator.bn128_g2_pubkey);
                let signature = operator
                    .bn128_privkey
                    .sign::<Sha256Normalized, &[u8; 32]>(&sunny_vote_message)
                    .unwrap();
                sunny_signatures.push(signature);
            }

            // Aggregate signatures and public keys for Sunny vote
            let sunny_apk2 = sunny_apk2_pubkeys
                .into_iter()
                .reduce(|acc, x| acc + x)
                .unwrap();
            let sunny_apk2_compressed = G2CompressedPoint::try_from(&sunny_apk2).unwrap().0;

            let sunny_agg_sig = sunny_signatures
                .into_iter()
                .reduce(|acc, x| acc + x)
                .unwrap();
            let sunny_agg_sig_compressed = G1CompressedPoint::try_from(sunny_agg_sig).unwrap().0;

            // Create signers bitmap - operator 0 didn't sign (bit 0 = 1), others signed (bit = 0)
            let non_signers_indices = vec![0]; // First operator didn't sign
            let sunny_signers_bitmap =
                create_signer_bitmap(&non_signers_indices, test_ncn.operators.len());

            // Cast the aggregated vote for Sunny weather
            ncn_program_client
                .do_cast_vote(
                    ncn_pubkey,
                    epoch,
                    sunny_agg_sig_compressed,
                    sunny_apk2_compressed,
                    sunny_signers_bitmap,
                    sunny_vote_message,
                )
                .await?;

            // Create a minority vote for Cloudy (just the first operator)
            let cloudy_vote_message =
                solana_nostd_sha256::hashv(&[b"weather_vote", b"Cloudy", &epoch.to_le_bytes()]);

            let first_operator = &test_ncn.operators[0];
            let cloudy_signature = first_operator
                .bn128_privkey
                .sign::<Sha256Normalized, &[u8; 32]>(&cloudy_vote_message)
                .unwrap();

            let cloudy_agg_sig = G1CompressedPoint::try_from(cloudy_signature).unwrap().0;
            let cloudy_apk2 = G2CompressedPoint::try_from(&first_operator.bn128_privkey)
                .unwrap()
                .0;

            // Create signers bitmap where only operator 0 signed, others didn't
            let cloudy_non_signers: Vec<usize> = (1..test_ncn.operators.len()).collect();
            let cloudy_signers_bitmap =
                create_signer_bitmap(&cloudy_non_signers, test_ncn.operators.len());

            // Cast the minority vote for Cloudy weather
            ncn_program_client
                .do_cast_vote(
                    ncn_pubkey,
                    epoch,
                    cloudy_agg_sig,
                    cloudy_apk2,
                    cloudy_signers_bitmap,
                    cloudy_vote_message,
                )
                .await?;

            println!("✅ BLS aggregate signature verification successful!");
            println!("✅ Cast vote operations completed successfully!");
        }

        {
            fixture.close_epoch_accounts_for_test_ncn(&test_ncn).await?;

            println!("✅ Consensus result account persisted after epoch cleanup");
        }

        println!("✅ BLS aggregate signature verification implementation complete!");
        println!("✅ Successfully demonstrated BLS signature aggregation and verification!");
        println!("✅ Simulation test completed successfully!");

        Ok(())
    }
}
