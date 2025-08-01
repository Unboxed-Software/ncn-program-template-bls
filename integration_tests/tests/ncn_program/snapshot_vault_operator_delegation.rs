#[cfg(test)]
mod tests {

    use ncn_program_core::{
        g1_point::{G1CompressedPoint, G1Point},
        g2_point::G2CompressedPoint,
        schemes::Sha256Normalized,
    };
    use solana_sdk::msg;

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_snapshot_vault_operator_delegation() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut vault_client = fixture.vault_program_client();
        let mut ncn_program_client = fixture.ncn_program_client();

        let test_ncn = fixture.create_initial_test_ncn(1, None).await?;
        fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;

        fixture.warp_slot_incremental(1000).await?;

        let epoch = fixture.clock().await.epoch;

        ncn_program_client
            .do_full_initialize_weight_table(test_ncn.ncn_root.ncn_pubkey, epoch)
            .await?;

        let ncn = test_ncn.ncn_root.ncn_pubkey;

        let vault_root = test_ncn.vaults[0].clone();
        let vault_address = vault_root.vault_pubkey;
        let vault = vault_client.get_vault(&vault_address).await?;

        let mint = vault.supported_mint;
        let weight = 100;

        ncn_program_client
            .do_admin_set_weight(ncn, epoch, mint, weight)
            .await?;

        ncn_program_client
            .do_full_initialize_epoch_snapshot(ncn, epoch)
            .await?;

        let operator = test_ncn.operators[0].operator_pubkey;

        ncn_program_client
            .do_initialize_operator_snapshot(operator, ncn, epoch)
            .await?;

        ncn_program_client
            .do_snapshot_vault_operator_delegation(vault_address, operator, ncn, epoch)
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_snapshot_aggregates_the_right_g1_pubkey() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut vault_client = fixture.vault_program_client();
        let mut ncn_program_client = fixture.ncn_program_client();

        const OPERATORS: usize = 10;

        let test_ncn = fixture.create_initial_test_ncn(OPERATORS, None).await?;
        fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;

        fixture.warp_slot_incremental(1000).await?;

        let epoch = fixture.clock().await.epoch;

        ncn_program_client
            .do_full_initialize_weight_table(test_ncn.ncn_root.ncn_pubkey, epoch)
            .await?;

        let ncn = test_ncn.ncn_root.ncn_pubkey;

        let vault_root = test_ncn.vaults[0].clone();
        let vault_address = vault_root.vault_pubkey;
        let vault = vault_client.get_vault(&vault_address).await?;

        let mint = vault.supported_mint;
        let weight = 100;

        ncn_program_client
            .do_admin_set_weight(ncn, epoch, mint, weight)
            .await?;

        ncn_program_client
            .do_full_initialize_epoch_snapshot(ncn, epoch)
            .await?;

        for operator_root in test_ncn.operators.iter() {
            ncn_program_client
                .do_initialize_operator_snapshot(operator_root.operator_pubkey, ncn, epoch)
                .await?;

            ncn_program_client
                .do_snapshot_vault_operator_delegation(
                    vault_address,
                    operator_root.operator_pubkey,
                    ncn,
                    epoch,
                )
                .await?;
        }

        let operator_registry = ncn_program_client.get_operator_registry(ncn).await?;

        assert_eq!(operator_registry.operator_count(), OPERATORS as u64);

        // Verify that the epoch snapshot aggregates the G1 pubkeys correctly
        let mut g1_pubkeys = Vec::new();
        for operator_root in test_ncn.operators.iter() {
            let g1_pubkey = G1Point::try_from(operator_root.bn128_privkey).unwrap();
            g1_pubkeys.push(g1_pubkey);
        }
        let agg_g1_pubkey = g1_pubkeys.into_iter().reduce(|acc, x| acc + x).unwrap();
        let agg_g1_pubkey_compressed = G1CompressedPoint::try_from(agg_g1_pubkey).unwrap();

        let epoch_snapshot = ncn_program_client.get_epoch_snapshot(ncn).await?;

        assert_eq!(
            epoch_snapshot.total_agg_g1_pubkey(),
            agg_g1_pubkey_compressed.0
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_snapshot_aggregates_the_right_g1_pubkey_if_one_is_not_active() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        fixture.initialize_restaking_and_vault_programs().await?;
        let mut vault_client = fixture.vault_program_client();

        const OPERATORS: usize = 10;

        let mut test_ncn = fixture.create_test_ncn().await?;
        let mut ncn_program_client = fixture.ncn_program_client();
        ncn_program_client
            .setup_ncn_program(&test_ncn.ncn_root)
            .await?;

        fixture
            .add_operators_to_test_ncn(&mut test_ncn, OPERATORS, Some(100))
            .await?;
        fixture
            .add_vaults_to_test_ncn(&mut test_ncn, 1, None)
            .await?;
        fixture.add_delegation_in_test_ncn(&test_ncn, 100).await?;
        fixture.add_vault_registry_to_test_ncn(&test_ncn).await?;
        fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;

        fixture.warp_slot_incremental(1000).await?;

        let epoch = fixture.clock().await.epoch;

        ncn_program_client
            .do_full_initialize_weight_table(test_ncn.ncn_root.ncn_pubkey, epoch)
            .await?;

        let ncn = test_ncn.ncn_root.ncn_pubkey;

        let vault_root = test_ncn.vaults[0].clone();
        let vault_address = vault_root.vault_pubkey;
        let vault = vault_client.get_vault(&vault_address).await?;

        let mint = vault.supported_mint;
        let weight = 100;

        ncn_program_client
            .do_admin_set_weight(ncn, epoch, mint, weight)
            .await?;

        let mut g1_pubkeys = Vec::new();

        for operator_root in test_ncn.operators.iter().skip(1) {
            let g1_pubkey = G1Point::try_from(operator_root.bn128_privkey).unwrap();
            let g1_compressed = G1CompressedPoint::try_from(g1_pubkey).unwrap();
            g1_pubkeys.push(g1_pubkey);
            let g2_compressed = G2CompressedPoint::try_from(&operator_root.bn128_privkey).unwrap();

            let signature = operator_root
                .bn128_privkey
                .sign::<Sha256Normalized, &[u8; 32]>(&g1_compressed.0)
                .unwrap();

            ncn_program_client
                .do_register_operator(
                    ncn,
                    operator_root.operator_pubkey,
                    &operator_root.operator_admin,
                    g1_compressed.0,
                    g2_compressed.0,
                    signature.0,
                )
                .await?;
        }

        ncn_program_client
            .do_full_initialize_epoch_snapshot(ncn, epoch)
            .await?;

        for operator_root in test_ncn.operators.iter() {
            ncn_program_client
                .do_initialize_operator_snapshot(operator_root.operator_pubkey, ncn, epoch)
                .await?;

            ncn_program_client
                .do_snapshot_vault_operator_delegation(
                    vault_address,
                    operator_root.operator_pubkey,
                    ncn,
                    epoch,
                )
                .await?;
        }

        let operator_registry = ncn_program_client.get_operator_registry(ncn).await?;

        assert_eq!(operator_registry.operator_count(), OPERATORS as u64 - 1);

        // Verify that the epoch snapshot aggregates the G1 pubkeys correctly
        let agg_g1_pubkey = g1_pubkeys.into_iter().reduce(|acc, x| acc + x).unwrap();
        let agg_g1_pubkey_compressed = G1CompressedPoint::try_from(agg_g1_pubkey).unwrap();

        let mut all_g1_pubkeys = Vec::new();
        for operator_root in test_ncn.operators.iter() {
            let g1_pubkey = G1Point::try_from(operator_root.bn128_privkey).unwrap();
            all_g1_pubkeys.push(g1_pubkey);
        }
        let all_agg_g1_pubkey = all_g1_pubkeys.into_iter().reduce(|acc, x| acc + x).unwrap();
        let all_agg_g1_pubkey_compressed = G1CompressedPoint::try_from(all_agg_g1_pubkey).unwrap();

        let epoch_snapshot = ncn_program_client.get_epoch_snapshot(ncn).await?;

        assert_eq!(
            epoch_snapshot.total_agg_g1_pubkey(),
            agg_g1_pubkey_compressed.0
        );

        assert_ne!(
            epoch_snapshot.total_agg_g1_pubkey(),
            all_agg_g1_pubkey_compressed.0
        );

        assert_ne!(&agg_g1_pubkey_compressed.0, &all_agg_g1_pubkey_compressed.0);

        Ok(())
    }

    #[tokio::test]
    async fn test_operator_snapshot_gets_updated_when_snapshotted() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut vault_client = fixture.vault_program_client();
        let mut ncn_program_client = fixture.ncn_program_client();

        let test_ncn = fixture.create_initial_test_ncn(1, None).await?;
        fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;

        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let operator = test_ncn.operators[0].operator_pubkey;
        let vault_root = test_ncn.vaults[0].clone();
        let vault_address = vault_root.vault_pubkey;
        let vault = vault_client.get_vault(&vault_address).await?;
        let mint = vault.supported_mint;

        {
            // Warp to new epoch
            fixture.warp_slot_incremental(1000).await?;
            let current_epoch = fixture.clock().await.epoch;
            println!("=== Testing Epoch: {} ===", current_epoch);

            // Initialize weight table for this epoch
            ncn_program_client
                .do_full_initialize_weight_table(ncn, current_epoch)
                .await?;

            let weight = 100; // Increase weight each epoch
            ncn_program_client
                .do_admin_set_weight(ncn, current_epoch, mint, weight)
                .await?;

            // Initialize epoch snapshot for this epoch
            ncn_program_client
                .do_full_initialize_epoch_snapshot(ncn, current_epoch)
                .await?;

            // Initialize operator snapshot for this epoch
            ncn_program_client
                .do_initialize_operator_snapshot(operator, ncn, current_epoch)
                .await?;

            // Get operator snapshot before delegation snapshot
            let operator_snapshot_before = ncn_program_client
                .get_operator_snapshot(operator, ncn)
                .await?;

            println!(
                "Before snapshot - Last snapshot slot: {}",
                operator_snapshot_before.last_snapshot_slot()
            );
            println!(
                "Before snapshot - Slot created: {}",
                operator_snapshot_before.slot_created()
            );
            println!(
                "Before snapshot - Stake weight: {}",
                operator_snapshot_before.stake_weight().stake_weight()
            );
            println!(
                "Before snapshot - Next epoch stake weight: {}",
                operator_snapshot_before
                    .next_epoch_stake_weight()
                    .stake_weight()
            );
            println!(
                "Before snapshot - Has minimum stake weight: {}",
                operator_snapshot_before.has_minimum_stake_weight()
            );

            // Record the current slot before snapshot
            let current_slot = fixture.clock().await.slot;

            // Take the operator snapshot
            ncn_program_client
                .do_snapshot_vault_operator_delegation(vault_address, operator, ncn, current_epoch)
                .await?;

            // Get operator snapshot after delegation snapshot
            let epoch_snapshot_after = ncn_program_client.get_epoch_snapshot(ncn).await?;
            let operator_snapshot_after = ncn_program_client
                .get_operator_snapshot(operator, ncn)
                .await?;

            println!(
                "After snapshot - Last snapshot slot: {}",
                operator_snapshot_after.last_snapshot_slot()
            );
            println!(
                "After snapshot - Stake weight: {}",
                operator_snapshot_after.stake_weight().stake_weight()
            );
            println!(
                "After snapshot - Next epoch stake weight: {}",
                operator_snapshot_after
                    .next_epoch_stake_weight()
                    .stake_weight()
            );
            println!(
                "After snapshot - Has minimum stake weight: {}",
                operator_snapshot_after.has_minimum_stake_weight()
            );

            // Verify that last_snapshot_slot was updated
            assert!(
                operator_snapshot_after.last_snapshot_slot()
                    > operator_snapshot_before.last_snapshot_slot(),
                "Last snapshot slot should be updated in epoch {}",
                current_epoch
            );

            // Verify that last_snapshot_slot is around the current slot
            assert!(
                operator_snapshot_after.last_snapshot_slot() >= current_slot,
                "Last snapshot slot should be at least the current slot in epoch {}",
                current_epoch
            );

            // Verify that stake weights were calculated and set
            assert!(
                operator_snapshot_after.stake_weight().stake_weight() > 0,
                "Stake weight should be greater than 0 after snapshot"
            );
            assert!(
                operator_snapshot_after
                    .next_epoch_stake_weight()
                    .stake_weight()
                    > 0,
                "Next epoch stake weight should be greater than 0 after snapshot"
            );

            // Verify operator registration count was updated properly
            assert_eq!(epoch_snapshot_after.operators_registered(), 1);

            // Verify that the operator snapshot can vote (has minimum stake weight and is active)
            assert!(
            operator_snapshot_after.has_minimum_stake_weight()
                && operator_snapshot_after.is_active(),
            "Operator snapshot should be eligible to vote (has minimum stake weight and is active)"
            );

            println!("✅ Operator snapshot update verification completed!");
            println!(
                "   • Last snapshot slot: {} → {}",
                operator_snapshot_before.last_snapshot_slot(),
                operator_snapshot_after.last_snapshot_slot()
            );
            println!(
                "   • Stake weight: {} → {}",
                operator_snapshot_before.stake_weight().stake_weight(),
                operator_snapshot_after.stake_weight().stake_weight()
            );
            println!(
                "   • Has minimum stake: {} → {}",
                operator_snapshot_before.has_minimum_stake_weight(),
                operator_snapshot_after.has_minimum_stake_weight()
            );
        }

        {
            // Warp to new epoch
            fixture.warp_epoch_incremental(1).await?;
            let current_epoch = fixture.clock().await.epoch;
            println!("=== Testing Epoch: {} ===", current_epoch);

            fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;
            // Initialize weight table for this epoch
            ncn_program_client
                .do_full_initialize_weight_table(ncn, current_epoch)
                .await?;

            let weight = 200;
            ncn_program_client
                .do_admin_set_weight(ncn, current_epoch, mint, weight)
                .await?;

            let operator_snapshot_before = ncn_program_client
                .get_operator_snapshot(operator, ncn)
                .await?;

            println!(
                "Before snapshot - Last snapshot slot: {}",
                operator_snapshot_before.last_snapshot_slot()
            );
            println!(
                "Before snapshot - Slot created: {}",
                operator_snapshot_before.slot_created()
            );
            println!(
                "Before snapshot - Stake weight: {}",
                operator_snapshot_before.stake_weight().stake_weight()
            );
            println!(
                "Before snapshot - Next epoch stake weight: {}",
                operator_snapshot_before
                    .next_epoch_stake_weight()
                    .stake_weight()
            );
            println!(
                "Before snapshot - Has minimum stake weight: {}",
                operator_snapshot_before.has_minimum_stake_weight()
            );

            // Record the current slot before snapshot
            let current_slot = fixture.clock().await.slot;

            fixture
                .add_vault_operator_delegation_snapshots_to_test_ncn(&test_ncn)
                .await?;

            // Get operator snapshot after delegation snapshot
            let epoch_snapshot_after = ncn_program_client.get_epoch_snapshot(ncn).await?;
            let operator_snapshot_after = ncn_program_client
                .get_operator_snapshot(operator, ncn)
                .await?;

            println!(
                "After snapshot - Last snapshot slot: {}",
                operator_snapshot_after.last_snapshot_slot()
            );
            println!(
                "After snapshot - Stake weight: {}",
                operator_snapshot_after.stake_weight().stake_weight()
            );
            println!(
                "After snapshot - Next epoch stake weight: {}",
                operator_snapshot_after
                    .next_epoch_stake_weight()
                    .stake_weight()
            );
            println!(
                "After snapshot - Has minimum stake weight: {}",
                operator_snapshot_after.has_minimum_stake_weight()
            );

            // Verify that last_snapshot_slot was updated
            assert!(
                operator_snapshot_after.last_snapshot_slot()
                    > operator_snapshot_before.last_snapshot_slot(),
                "Last snapshot slot should be updated in epoch {}",
                current_epoch
            );

            // Verify that last_snapshot_slot is around the current slot
            assert!(
                operator_snapshot_after.last_snapshot_slot() >= current_slot,
                "Last snapshot slot should be at least the current slot in epoch {}",
                current_epoch
            );

            // Verify that stake weights were calculated and set
            assert!(
                operator_snapshot_after.stake_weight().stake_weight()
                    > operator_snapshot_before.stake_weight().stake_weight(),
                "Stake weight should be greater than 0 after snapshot"
            );
            assert!(
                operator_snapshot_after
                    .next_epoch_stake_weight()
                    .stake_weight()
                    > 0,
                "Next epoch stake weight should be greater than 0 after snapshot"
            );

            // Verify operator registration count was updated properly
            assert_eq!(epoch_snapshot_after.operators_registered(), 2);

            // Verify that the operator snapshot can vote (has minimum stake weight and is active)
            assert!(
            operator_snapshot_after.has_minimum_stake_weight()
                && operator_snapshot_after.is_active(),
            "Operator snapshot should be eligible to vote (has minimum stake weight and is active)"
            );

            println!("✅ Operator snapshot update verification completed!");
            println!(
                "   • Last snapshot slot: {} → {}",
                operator_snapshot_before.last_snapshot_slot(),
                operator_snapshot_after.last_snapshot_slot()
            );
            println!(
                "   • Stake weight: {} → {}",
                operator_snapshot_before.stake_weight().stake_weight(),
                operator_snapshot_after.stake_weight().stake_weight()
            );
            println!(
                "   • Has minimum stake: {} → {}",
                operator_snapshot_before.has_minimum_stake_weight(),
                operator_snapshot_after.has_minimum_stake_weight()
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_operator_snapshot_delegation_next_epoch_calculations() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut vault_client = fixture.vault_program_client();

        fixture.initialize_restaking_and_vault_programs().await?;

        // NCN prep
        let mut test_ncn = fixture.create_test_ncn().await?;
        let mut ncn_program_client = fixture.ncn_program_client();
        ncn_program_client
            .do_initialize_config(
                test_ncn.ncn_root.ncn_pubkey,
                &test_ncn.ncn_root.ncn_admin,
                Some(10000),
            )
            .await?;

        ncn_program_client
            .do_full_initialize_vault_registry(test_ncn.ncn_root.ncn_pubkey)
            .await?;

        ncn_program_client
            .do_full_initialize_operator_registry(test_ncn.ncn_root.ncn_pubkey)
            .await?;
        fixture
            .add_operators_to_test_ncn(&mut test_ncn, 1, None)
            .await?;
        fixture
            .add_vaults_to_test_ncn(&mut test_ncn, 1, None)
            .await?;
        fixture.add_delegation_in_test_ncn(&test_ncn, 10).await?;
        fixture.add_vault_registry_to_test_ncn(&test_ncn).await?;
        fixture.register_operators_to_test_ncn(&test_ncn).await?;
        fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;
        fixture.add_weights_for_test_ncn(&test_ncn).await?;
        fixture.add_epoch_snapshot_to_test_ncn(&test_ncn).await?;
        fixture
            .add_operator_snapshots_to_test_ncn(&test_ncn)
            .await?;

        fixture
            .add_vault_operator_delegation_snapshots_to_test_ncn(&test_ncn)
            .await?;

        {
            // the operator does not have enough stake
            let epoch_snapshot = ncn_program_client
                .get_epoch_snapshot(test_ncn.ncn_root.ncn_pubkey)
                .await?;
            let operator_snapshot = epoch_snapshot.get_operator_snapshot(0).unwrap();

            assert_eq!(operator_snapshot.stake_weight().stake_weight(), 1000);
            assert_eq!(
                operator_snapshot.next_epoch_stake_weight().stake_weight(),
                1000
            );

            assert!(!operator_snapshot.has_minimum_stake_weight());
            assert!(!operator_snapshot.has_minimum_stake_weight_next_epoch());
            msg!("Epoch Snapshot before more delegation: {}", epoch_snapshot);
        }

        {
            // adding more delegation should takes the operator to over the minimum stake weight
            fixture.add_delegation_in_test_ncn(&test_ncn, 90).await?;
            fixture
                .add_vault_operator_delegation_snapshots_to_test_ncn(&test_ncn)
                .await?;
            let epoch_snapshot = ncn_program_client
                .get_epoch_snapshot(test_ncn.ncn_root.ncn_pubkey)
                .await?;
            let operator_snapshot = epoch_snapshot.get_operator_snapshot(0).unwrap();

            assert_eq!(operator_snapshot.stake_weight().stake_weight(), 10000);
            assert_eq!(
                operator_snapshot.next_epoch_stake_weight().stake_weight(),
                10000
            );
            //
            assert!(operator_snapshot.has_minimum_stake_weight());
            assert!(operator_snapshot.has_minimum_stake_weight_next_epoch());
            msg!("Epoch Snapshot after delegation: {}", epoch_snapshot);
        }

        {
            // cooling down some of the delegation should only take effect in the next epoch
            vault_client
                .do_cooldown_delegation(
                    &test_ncn.vaults[0],
                    &test_ncn.operators[0].operator_pubkey,
                    10,
                )
                .await?;

            fixture
                .add_vault_operator_delegation_snapshots_to_test_ncn(&test_ncn)
                .await?;
            let epoch_snapshot = ncn_program_client
                .get_epoch_snapshot(test_ncn.ncn_root.ncn_pubkey)
                .await?;
            let operator_snapshot = epoch_snapshot.get_operator_snapshot(0).unwrap();

            assert_eq!(operator_snapshot.stake_weight().stake_weight(), 10000);
            assert_eq!(
                operator_snapshot.next_epoch_stake_weight().stake_weight(),
                10000
            );
            assert!(operator_snapshot.has_minimum_stake_weight());
            assert!(operator_snapshot.has_minimum_stake_weight_next_epoch());

            msg!(
                "Epoch Snapshot after cooldown delegation: {}",
                epoch_snapshot
            );
        }

        fixture.warp_epoch_incremental(1).await?;
        fixture
            .update_snapshot_test_ncn_new_epoch(&test_ncn)
            .await?;

        {
            // the operator should still have the same stake weight, but next epoch stake weight should be updated
            let epoch_snapshot = ncn_program_client
                .get_epoch_snapshot(test_ncn.ncn_root.ncn_pubkey)
                .await?;
            let operator_snapshot = epoch_snapshot.get_operator_snapshot(0).unwrap();
            assert_eq!(operator_snapshot.stake_weight().stake_weight(), 10000);
            assert_eq!(
                operator_snapshot.next_epoch_stake_weight().stake_weight(),
                9000
            );
            assert!(operator_snapshot.has_minimum_stake_weight());
            assert!(!operator_snapshot.has_minimum_stake_weight_next_epoch());
            msg!(
                "Epoch Snapshot after warpping one epoch: {}",
                epoch_snapshot
            );
        }

        fixture.warp_epoch_incremental(1).await?;
        fixture
            .update_snapshot_test_ncn_new_epoch(&test_ncn)
            .await?;

        {
            // after two epochs, the funds should be fully withdrawn from the operator
            let epoch_snapshot = ncn_program_client
                .get_epoch_snapshot(test_ncn.ncn_root.ncn_pubkey)
                .await?;
            let operator_snapshot = epoch_snapshot.get_operator_snapshot(0).unwrap();
            assert_eq!(operator_snapshot.stake_weight().stake_weight(), 9000);
            assert_eq!(
                operator_snapshot.next_epoch_stake_weight().stake_weight(),
                9000
            );
            assert!(!operator_snapshot.has_minimum_stake_weight());
            assert!(!operator_snapshot.has_minimum_stake_weight_next_epoch());
            msg!(
                "Epoch Snapshot after warpping two epochs: {}",
                epoch_snapshot
            );
        }

        Ok(())
    }
}
