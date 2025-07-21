#[cfg(test)]
mod tests {

    use ncn_program_core::{
        g1_point::{G1CompressedPoint, G1Point},
        g2_point::G2CompressedPoint,
        schemes::Sha256Normalized,
    };

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_snapshot_vault_operator_delegation() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut vault_client = fixture.vault_program_client();
        let mut ncn_program_client = fixture.ncn_program_client();

        let test_ncn = fixture.create_initial_test_ncn(1, 1, None).await?;
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

        let test_ncn = fixture.create_initial_test_ncn(OPERATORS, 1, None).await?;
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

        let epoch_snapshot = ncn_program_client.get_epoch_snapshot(ncn, epoch).await?;

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

        let epoch_snapshot = ncn_program_client.get_epoch_snapshot(ncn, epoch).await?;

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
}
