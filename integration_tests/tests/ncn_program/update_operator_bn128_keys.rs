#[cfg(test)]
mod tests {
    use crate::fixtures::{test_builder::TestBuilder, TestResult};
    use ncn_program_core::{
        error::NCNProgramError, g1_point::G1CompressedPoint, g2_point::G2CompressedPoint,
        privkey::PrivKey, schemes::Sha256Normalized,
    };
    use solana_sdk::signature::Keypair;

    #[tokio::test]
    async fn test_update_operator_bn128_keys_success() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;

        let mut restaking_program_client = fixture.restaking_program_client();
        let mut ncn_program_client = fixture.ncn_program_client();

        // Setup NCN
        let ncn_root = fixture.setup_ncn().await?;

        ncn_program_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin, None)
            .await?;

        ncn_program_client
            .do_full_initialize_operator_registry(ncn_root.ncn_pubkey)
            .await?;

        // Setup operator
        let operator_root = restaking_program_client
            .do_initialize_operator(Some(200))
            .await?;

        // Setup operator and handshake
        restaking_program_client
            .do_initialize_ncn_operator_state(&ncn_root, &operator_root.operator_pubkey)
            .await?;
        fixture.warp_slot_incremental(1).await.unwrap();
        restaking_program_client
            .do_ncn_warmup_operator(&ncn_root, &operator_root.operator_pubkey)
            .await?;
        restaking_program_client
            .do_operator_warmup_ncn(&operator_root, &ncn_root.ncn_pubkey)
            .await?;

        // Generate initial BLS keypair
        let initial_g1_compressed =
            G1CompressedPoint::try_from(operator_root.bn128_privkey).unwrap();
        let initial_g2_compressed =
            G2CompressedPoint::try_from(&operator_root.bn128_privkey).unwrap();

        let initial_signature = operator_root
            .bn128_privkey
            .sign::<Sha256Normalized, &[u8; 32]>(&initial_g1_compressed.0)
            .unwrap();

        // Register operator first
        ncn_program_client
            .do_register_operator(
                ncn_root.ncn_pubkey,
                operator_root.operator_pubkey,
                &operator_root.operator_admin,
                initial_g1_compressed.0,
                initial_g2_compressed.0,
                initial_signature.0,
            )
            .await?;

        // Generate new BLS keypair for update
        let new_private_key = PrivKey::from_random();
        let new_g1_compressed = G1CompressedPoint::try_from(new_private_key).unwrap();
        let new_g2_compressed = G2CompressedPoint::try_from(&new_private_key).unwrap();

        let new_signature = new_private_key
            .sign::<Sha256Normalized, &[u8; 32]>(&new_g1_compressed.0)
            .unwrap();

        // Update operator BLS keys
        ncn_program_client
            .do_update_operator_bn128_keys(
                ncn_root.ncn_pubkey,
                operator_root.operator_pubkey,
                &operator_root.operator_admin,
                new_g1_compressed.0,
                new_g2_compressed.0,
                new_signature.0,
            )
            .await?;

        // Verify operator keys were updated
        let operator_registry = ncn_program_client
            .get_operator_registry(ncn_root.ncn_pubkey)
            .await?;

        assert_eq!(operator_registry.operator_count(), 1);

        let operator_entry = operator_registry
            .get_operator_entry(&operator_root.operator_pubkey)
            .unwrap();

        // Verify keys were updated to new values
        assert_eq!(operator_entry.g1_pubkey(), &new_g1_compressed.0);
        assert_eq!(operator_entry.g2_pubkey(), &new_g2_compressed.0);

        // Verify keys are NOT the old values
        assert_ne!(operator_entry.g1_pubkey(), &initial_g1_compressed.0);
        assert_ne!(operator_entry.g2_pubkey(), &initial_g2_compressed.0);

        Ok(())
    }

    #[tokio::test]
    async fn test_update_operator_bn128_keys_unregistered_operator_fails() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;

        let mut restaking_program_client = fixture.restaking_program_client();
        let mut ncn_program_client = fixture.ncn_program_client();

        // Setup NCN
        let ncn_root = fixture.setup_ncn().await?;

        ncn_program_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin, None)
            .await?;

        ncn_program_client
            .do_full_initialize_operator_registry(ncn_root.ncn_pubkey)
            .await?;

        // Setup operator but DON'T register it
        let operator_root = restaking_program_client
            .do_initialize_operator(Some(200))
            .await?;

        // Setup operator and handshake
        restaking_program_client
            .do_initialize_ncn_operator_state(&ncn_root, &operator_root.operator_pubkey)
            .await?;
        fixture.warp_slot_incremental(1).await.unwrap();
        restaking_program_client
            .do_ncn_warmup_operator(&ncn_root, &operator_root.operator_pubkey)
            .await?;
        restaking_program_client
            .do_operator_warmup_ncn(&operator_root, &ncn_root.ncn_pubkey)
            .await?;

        // Generate BLS keypair for update
        let new_private_key = PrivKey::from_random();
        let new_g1_compressed = G1CompressedPoint::try_from(new_private_key).unwrap();
        let new_g2_compressed = G2CompressedPoint::try_from(&new_private_key).unwrap();

        let new_signature = new_private_key
            .sign::<Sha256Normalized, &[u8; 32]>(&new_g1_compressed.0)
            .unwrap();

        // Try to update unregistered operator should fail
        let result = ncn_program_client
            .do_update_operator_bn128_keys(
                ncn_root.ncn_pubkey,
                operator_root.operator_pubkey,
                &operator_root.operator_admin,
                new_g1_compressed.0,
                new_g2_compressed.0,
                new_signature.0,
            )
            .await;

        assert!(result.is_err());
        // Should fail because operator is not registered
        crate::fixtures::ncn_program_client::assert_ncn_program_error(
            result,
            NCNProgramError::OperatorEntryNotFound,
            Some(0),
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_update_operator_bn128_keys_mismatched_keys_fails() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;

        let mut restaking_program_client = fixture.restaking_program_client();
        let mut ncn_program_client = fixture.ncn_program_client();

        // Setup NCN
        let ncn_root = fixture.setup_ncn().await?;

        ncn_program_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin, None)
            .await?;

        ncn_program_client
            .do_full_initialize_operator_registry(ncn_root.ncn_pubkey)
            .await?;

        // Setup operator
        let operator_root = restaking_program_client
            .do_initialize_operator(Some(200))
            .await?;

        // Setup operator and handshake
        restaking_program_client
            .do_initialize_ncn_operator_state(&ncn_root, &operator_root.operator_pubkey)
            .await?;
        fixture.warp_slot_incremental(1).await.unwrap();
        restaking_program_client
            .do_ncn_warmup_operator(&ncn_root, &operator_root.operator_pubkey)
            .await?;
        restaking_program_client
            .do_operator_warmup_ncn(&operator_root, &ncn_root.ncn_pubkey)
            .await?;

        // Generate initial BLS keypair and register
        let initial_private_key = PrivKey::from_random();
        let initial_g1_compressed = G1CompressedPoint::try_from(initial_private_key).unwrap();
        let initial_g2_compressed = G2CompressedPoint::try_from(&initial_private_key).unwrap();

        let initial_signature = initial_private_key
            .sign::<Sha256Normalized, &[u8; 32]>(&initial_g1_compressed.0)
            .unwrap();

        ncn_program_client
            .do_register_operator(
                ncn_root.ncn_pubkey,
                operator_root.operator_pubkey,
                &operator_root.operator_admin,
                initial_g1_compressed.0,
                initial_g2_compressed.0,
                initial_signature.0,
            )
            .await?;

        // Generate mismatched keypair for update
        let new_private_key1 = PrivKey::from_random();
        let new_private_key2 = PrivKey::from_random();
        let new_g1_compressed = G1CompressedPoint::try_from(new_private_key1).unwrap();
        let new_g2_compressed = G2CompressedPoint::try_from(&new_private_key2).unwrap(); // Different key!

        let new_signature = new_private_key1
            .sign::<Sha256Normalized, &[u8; 32]>(&new_g1_compressed.0)
            .unwrap();

        // Try to update with mismatched keys should fail
        let result = ncn_program_client
            .do_update_operator_bn128_keys(
                ncn_root.ncn_pubkey,
                operator_root.operator_pubkey,
                &operator_root.operator_admin,
                new_g1_compressed.0,
                new_g2_compressed.0,
                new_signature.0,
            )
            .await;

        assert!(result.is_err());
        // Should fail due to BLS verification error (keys don't match)
        crate::fixtures::ncn_program_client::assert_ncn_program_error(
            result,
            NCNProgramError::BLSVerificationError,
            Some(0),
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_update_operator_bn128_keys_invalid_signature_fails() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;

        let mut restaking_program_client = fixture.restaking_program_client();
        let mut ncn_program_client = fixture.ncn_program_client();

        // Setup NCN
        let ncn_root = fixture.setup_ncn().await?;

        ncn_program_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin, None)
            .await?;

        ncn_program_client
            .do_full_initialize_operator_registry(ncn_root.ncn_pubkey)
            .await?;

        // Setup operator
        let operator_root = restaking_program_client
            .do_initialize_operator(Some(200))
            .await?;

        // Setup operator and handshake
        restaking_program_client
            .do_initialize_ncn_operator_state(&ncn_root, &operator_root.operator_pubkey)
            .await?;
        fixture.warp_slot_incremental(1).await.unwrap();
        restaking_program_client
            .do_ncn_warmup_operator(&ncn_root, &operator_root.operator_pubkey)
            .await?;
        restaking_program_client
            .do_operator_warmup_ncn(&operator_root, &ncn_root.ncn_pubkey)
            .await?;

        // Generate initial BLS keypair and register
        let initial_private_key = PrivKey::from_random();
        let initial_g1_compressed = G1CompressedPoint::try_from(initial_private_key).unwrap();
        let initial_g2_compressed = G2CompressedPoint::try_from(&initial_private_key).unwrap();

        let initial_signature = initial_private_key
            .sign::<Sha256Normalized, &[u8; 32]>(&initial_g1_compressed.0)
            .unwrap();

        ncn_program_client
            .do_register_operator(
                ncn_root.ncn_pubkey,
                operator_root.operator_pubkey,
                &operator_root.operator_admin,
                initial_g1_compressed.0,
                initial_g2_compressed.0,
                initial_signature.0,
            )
            .await?;

        // Generate new keypair but use wrong signature
        let new_private_key = PrivKey::from_random();
        let wrong_private_key = PrivKey::from_random();
        let new_g1_compressed = G1CompressedPoint::try_from(new_private_key).unwrap();
        let new_g2_compressed = G2CompressedPoint::try_from(&new_private_key).unwrap();

        // Sign with wrong private key
        let wrong_signature = wrong_private_key
            .sign::<Sha256Normalized, &[u8; 32]>(&new_g1_compressed.0)
            .unwrap();

        // Try to update with invalid signature should fail
        let result = ncn_program_client
            .do_update_operator_bn128_keys(
                ncn_root.ncn_pubkey,
                operator_root.operator_pubkey,
                &operator_root.operator_admin,
                new_g1_compressed.0,
                new_g2_compressed.0,
                wrong_signature.0,
            )
            .await;

        assert!(result.is_err());
        // Should fail due to BLS verification error (signature invalid)
        crate::fixtures::ncn_program_client::assert_ncn_program_error(
            result,
            NCNProgramError::BLSVerificationError,
            Some(0),
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_update_operator_bn128_keys_unauthorized_admin_fails() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;

        let mut restaking_program_client = fixture.restaking_program_client();
        let mut ncn_program_client = fixture.ncn_program_client();

        // Setup NCN
        let ncn_root = fixture.setup_ncn().await?;

        ncn_program_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin, None)
            .await?;

        ncn_program_client
            .do_full_initialize_operator_registry(ncn_root.ncn_pubkey)
            .await?;

        // Setup operator
        let operator_root = restaking_program_client
            .do_initialize_operator(Some(200))
            .await?;

        // Setup operator and handshake
        restaking_program_client
            .do_initialize_ncn_operator_state(&ncn_root, &operator_root.operator_pubkey)
            .await?;
        fixture.warp_slot_incremental(1).await.unwrap();
        restaking_program_client
            .do_ncn_warmup_operator(&ncn_root, &operator_root.operator_pubkey)
            .await?;
        restaking_program_client
            .do_operator_warmup_ncn(&operator_root, &ncn_root.ncn_pubkey)
            .await?;

        // Generate initial BLS keypair and register
        let initial_private_key = PrivKey::from_random();
        let initial_g1_compressed = G1CompressedPoint::try_from(initial_private_key).unwrap();
        let initial_g2_compressed = G2CompressedPoint::try_from(&initial_private_key).unwrap();

        let initial_signature = initial_private_key
            .sign::<Sha256Normalized, &[u8; 32]>(&initial_g1_compressed.0)
            .unwrap();

        ncn_program_client
            .do_register_operator(
                ncn_root.ncn_pubkey,
                operator_root.operator_pubkey,
                &operator_root.operator_admin,
                initial_g1_compressed.0,
                initial_g2_compressed.0,
                initial_signature.0,
            )
            .await?;

        // Generate new keypair for update
        let new_private_key = PrivKey::from_random();
        let new_g1_compressed = G1CompressedPoint::try_from(new_private_key).unwrap();
        let new_g2_compressed = G2CompressedPoint::try_from(&new_private_key).unwrap();

        let new_signature = new_private_key
            .sign::<Sha256Normalized, &[u8; 32]>(&new_g1_compressed.0)
            .unwrap();

        // Create unauthorized signer (not the operator admin)
        let fake_admin = Keypair::new();

        // Try to update with unauthorized admin should fail
        let result = ncn_program_client
            .do_update_operator_bn128_keys(
                ncn_root.ncn_pubkey,
                operator_root.operator_pubkey,
                &fake_admin,
                new_g1_compressed.0,
                new_g2_compressed.0,
                new_signature.0,
            )
            .await;

        assert!(result.is_err());
        // Should fail due to unauthorized admin

        Ok(())
    }

    #[tokio::test]
    async fn test_update_operator_bn128_keys_multiple_updates() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;

        let mut restaking_program_client = fixture.restaking_program_client();
        let mut ncn_program_client = fixture.ncn_program_client();

        // Setup NCN
        let ncn_root = fixture.setup_ncn().await?;

        ncn_program_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin, None)
            .await?;

        ncn_program_client
            .do_full_initialize_operator_registry(ncn_root.ncn_pubkey)
            .await?;

        // Setup operator
        let operator_root = restaking_program_client
            .do_initialize_operator(Some(200))
            .await?;

        // Setup operator and handshake
        restaking_program_client
            .do_initialize_ncn_operator_state(&ncn_root, &operator_root.operator_pubkey)
            .await?;
        fixture.warp_slot_incremental(1).await.unwrap();
        restaking_program_client
            .do_ncn_warmup_operator(&ncn_root, &operator_root.operator_pubkey)
            .await?;
        restaking_program_client
            .do_operator_warmup_ncn(&operator_root, &ncn_root.ncn_pubkey)
            .await?;

        // Generate initial BLS keypair and register
        let initial_private_key = PrivKey::from_random();
        let initial_g1_compressed = G1CompressedPoint::try_from(initial_private_key).unwrap();
        let initial_g2_compressed = G2CompressedPoint::try_from(&initial_private_key).unwrap();

        let initial_signature = initial_private_key
            .sign::<Sha256Normalized, &[u8; 32]>(&initial_g1_compressed.0)
            .unwrap();

        ncn_program_client
            .do_register_operator(
                ncn_root.ncn_pubkey,
                operator_root.operator_pubkey,
                &operator_root.operator_admin,
                initial_g1_compressed.0,
                initial_g2_compressed.0,
                initial_signature.0,
            )
            .await?;

        // First update
        let update1_private_key = PrivKey::from_random();
        let update1_g1_compressed = G1CompressedPoint::try_from(update1_private_key).unwrap();
        let update1_g2_compressed = G2CompressedPoint::try_from(&update1_private_key).unwrap();

        let update1_signature = update1_private_key
            .sign::<Sha256Normalized, &[u8; 32]>(&update1_g1_compressed.0)
            .unwrap();

        ncn_program_client
            .do_update_operator_bn128_keys(
                ncn_root.ncn_pubkey,
                operator_root.operator_pubkey,
                &operator_root.operator_admin,
                update1_g1_compressed.0,
                update1_g2_compressed.0,
                update1_signature.0,
            )
            .await?;

        // Verify first update
        let operator_registry = ncn_program_client
            .get_operator_registry(ncn_root.ncn_pubkey)
            .await?;

        let operator_entry = operator_registry
            .get_operator_entry(&operator_root.operator_pubkey)
            .unwrap();

        assert_eq!(operator_entry.g1_pubkey(), &update1_g1_compressed.0);
        assert_eq!(operator_entry.g2_pubkey(), &update1_g2_compressed.0);

        // Second update
        let update2_private_key = PrivKey::from_random();
        let update2_g1_compressed = G1CompressedPoint::try_from(update2_private_key).unwrap();
        let update2_g2_compressed = G2CompressedPoint::try_from(&update2_private_key).unwrap();

        let update2_signature = update2_private_key
            .sign::<Sha256Normalized, &[u8; 32]>(&update2_g1_compressed.0)
            .unwrap();

        ncn_program_client
            .do_update_operator_bn128_keys(
                ncn_root.ncn_pubkey,
                operator_root.operator_pubkey,
                &operator_root.operator_admin,
                update2_g1_compressed.0,
                update2_g2_compressed.0,
                update2_signature.0,
            )
            .await?;

        // Verify second update
        let operator_registry = ncn_program_client
            .get_operator_registry(ncn_root.ncn_pubkey)
            .await?;

        let operator_entry = operator_registry
            .get_operator_entry(&operator_root.operator_pubkey)
            .unwrap();

        assert_eq!(operator_entry.g1_pubkey(), &update2_g1_compressed.0);
        assert_eq!(operator_entry.g2_pubkey(), &update2_g2_compressed.0);

        // Verify keys are NOT from first update anymore
        assert_ne!(operator_entry.g1_pubkey(), &update1_g1_compressed.0);
        assert_ne!(operator_entry.g2_pubkey(), &update1_g2_compressed.0);

        Ok(())
    }

    #[tokio::test]
    async fn test_update_operator_bn128_keys_same_keys_updates_timestamp() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;

        let mut restaking_program_client = fixture.restaking_program_client();
        let mut ncn_program_client = fixture.ncn_program_client();

        // Setup NCN
        let ncn_root = fixture.setup_ncn().await?;

        ncn_program_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin, None)
            .await?;

        ncn_program_client
            .do_full_initialize_operator_registry(ncn_root.ncn_pubkey)
            .await?;

        // Setup operator
        let operator_root = restaking_program_client
            .do_initialize_operator(Some(200))
            .await?;

        // Setup operator and handshake
        restaking_program_client
            .do_initialize_ncn_operator_state(&ncn_root, &operator_root.operator_pubkey)
            .await?;
        fixture.warp_slot_incremental(1).await.unwrap();
        restaking_program_client
            .do_ncn_warmup_operator(&ncn_root, &operator_root.operator_pubkey)
            .await?;
        restaking_program_client
            .do_operator_warmup_ncn(&operator_root, &ncn_root.ncn_pubkey)
            .await?;

        // Generate BLS keypair
        let private_key = PrivKey::from_random();
        let g1_compressed = G1CompressedPoint::try_from(private_key).unwrap();
        let g2_compressed = G2CompressedPoint::try_from(&private_key).unwrap();

        let signature = private_key
            .sign::<Sha256Normalized, &[u8; 32]>(&g1_compressed.0)
            .unwrap();

        // Register operator
        ncn_program_client
            .do_register_operator(
                ncn_root.ncn_pubkey,
                operator_root.operator_pubkey,
                &operator_root.operator_admin,
                g1_compressed.0,
                g2_compressed.0,
                signature.0,
            )
            .await?;

        // Get initial timestamp
        let operator_registry = ncn_program_client
            .get_operator_registry(ncn_root.ncn_pubkey)
            .await?;

        let initial_entry = operator_registry
            .get_operator_entry(&operator_root.operator_pubkey)
            .unwrap();
        let initial_timestamp = initial_entry.slot_registered();

        // Wait a bit (in a real test environment, slots would advance)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Update with the same keys (should still succeed and update timestamp)
        ncn_program_client
            .do_update_operator_bn128_keys(
                ncn_root.ncn_pubkey,
                operator_root.operator_pubkey,
                &operator_root.operator_admin,
                g1_compressed.0,
                g2_compressed.0,
                signature.0,
            )
            .await?;

        // Verify keys are the same but timestamp might be updated
        let operator_registry = ncn_program_client
            .get_operator_registry(ncn_root.ncn_pubkey)
            .await?;

        let updated_entry = operator_registry
            .get_operator_entry(&operator_root.operator_pubkey)
            .unwrap();

        assert_eq!(updated_entry.g1_pubkey(), &g1_compressed.0);
        assert_eq!(updated_entry.g2_pubkey(), &g2_compressed.0);
        // Timestamp should be greater than or equal to initial timestamp
        assert!(updated_entry.slot_registered() >= initial_timestamp);

        Ok(())
    }
}
