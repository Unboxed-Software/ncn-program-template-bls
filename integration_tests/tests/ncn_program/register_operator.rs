#[cfg(test)]
mod tests {
    use crate::fixtures::{test_builder::TestBuilder, TestResult};
    use ncn_program_core::{
        g1_point::G1CompressedPoint, g2_point::G2CompressedPoint, privkey::PrivKey,
        schemes::Sha256Normalized,
    };

    #[tokio::test]
    async fn test_register_operator_success() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;

        let mut restaking_program_client = fixture.restaking_program_client();
        let mut ncn_program_client = fixture.ncn_program_client();

        // Setup NCN
        let ncn_root = fixture.setup_ncn().await?;

        ncn_program_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;

        ncn_program_client
            .do_full_initialize_operator_registry(ncn_root.ncn_pubkey)
            .await?;

        // Setup operator
        let operator_root = restaking_program_client
            .do_initialize_operator(Some(200))
            .await?;

        // Generate BLS keypair
        let g1_compressed = G1CompressedPoint::try_from(operator_root.bn128_privkey).unwrap();
        let g2_compressed = G2CompressedPoint::try_from(&operator_root.bn128_privkey).unwrap();

        let signature = operator_root
            .bn128_privkey
            .sign::<Sha256Normalized, &[u8; 32]>(&g1_compressed.0)
            .unwrap();

        // Test operator registration
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

        // Verify operator was registered
        let operator_registry = ncn_program_client
            .get_operator_registry(ncn_root.ncn_pubkey)
            .await?;

        assert_eq!(operator_registry.operator_count(), 1);
        assert!(operator_registry.has_operator(&operator_root.operator_pubkey));

        let operator_entry = operator_registry
            .get_operator_entry(&operator_root.operator_pubkey)
            .unwrap();

        assert_eq!(
            operator_entry.operator_pubkey(),
            &operator_root.operator_pubkey
        );
        assert_eq!(operator_entry.g1_pubkey(), &g1_compressed.0);
        assert_eq!(operator_entry.g2_pubkey(), &g2_compressed.0);

        Ok(())
    }

    #[tokio::test]
    async fn test_register_operator_duplicate() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;

        let mut restaking_program_client = fixture.restaking_program_client();
        let mut ncn_program_client = fixture.ncn_program_client();

        // Setup NCN
        let ncn_root = fixture.setup_ncn().await?;

        ncn_program_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;

        ncn_program_client
            .do_full_initialize_operator_registry(ncn_root.ncn_pubkey)
            .await?;

        // Setup operator
        let operator_root = restaking_program_client
            .do_initialize_operator(Some(200))
            .await?;

        // Generate BLS keypair
        let g1_compressed = G1CompressedPoint::try_from(operator_root.bn128_privkey).unwrap();
        let g2_compressed = G2CompressedPoint::try_from(&operator_root.bn128_privkey).unwrap();

        let signature = operator_root
            .bn128_privkey
            .sign::<Sha256Normalized, &[u8; 32]>(&g1_compressed.0)
            .unwrap();

        // Register operator first time
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

        // Register same operator again should succeed (no-op)
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

        // Should still only have one operator
        let operator_registry = ncn_program_client
            .get_operator_registry(ncn_root.ncn_pubkey)
            .await?;

        assert_eq!(operator_registry.operator_count(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_register_operator_mismatched_bls_keys() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;

        let mut restaking_program_client = fixture.restaking_program_client();
        let mut ncn_program_client = fixture.ncn_program_client();

        // Setup NCN
        let ncn_root = fixture.setup_ncn().await?;

        ncn_program_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;

        ncn_program_client
            .do_full_initialize_operator_registry(ncn_root.ncn_pubkey)
            .await?;

        // Setup operator
        let operator_root = restaking_program_client
            .do_initialize_operator(Some(200))
            .await?;

        // Generate mismatched BLS keypair
        let fake_privkey = PrivKey::from_random();
        let g1_compressed = G1CompressedPoint::try_from(operator_root.bn128_privkey).unwrap();
        let g2_compressed = G2CompressedPoint::try_from(&fake_privkey).unwrap(); // Different key!

        let signature = operator_root
            .bn128_privkey
            .sign::<Sha256Normalized, &[u8; 32]>(&g1_compressed.0)
            .unwrap();

        // Test operator registration with mismatched keys should fail
        let result = ncn_program_client
            .do_register_operator(
                ncn_root.ncn_pubkey,
                operator_root.operator_pubkey,
                &operator_root.operator_admin,
                g1_compressed.0,
                g2_compressed.0,
                signature.0,
            )
            .await;

        // This should fail due to BLS verification error
        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_register_operator_without_registry_fails() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;

        let mut restaking_program_client = fixture.restaking_program_client();
        let mut ncn_program_client = fixture.ncn_program_client();

        // Setup NCN but DON'T initialize operator registry
        let ncn_root = fixture.setup_ncn().await?;

        ncn_program_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;

        // Setup operator
        let operator_root = restaking_program_client
            .do_initialize_operator(Some(200))
            .await?;

        // Generate BLS keypair
        let g1_compressed = G1CompressedPoint::try_from(operator_root.bn128_privkey).unwrap();
        let g2_compressed = G2CompressedPoint::try_from(&operator_root.bn128_privkey).unwrap();

        let signature = operator_root
            .bn128_privkey
            .sign::<Sha256Normalized, &[u8; 32]>(&g1_compressed.0)
            .unwrap();

        // Try to register operator without operator registry should fail
        let result = ncn_program_client
            .do_register_operator(
                ncn_root.ncn_pubkey,
                operator_root.operator_pubkey,
                &operator_root.operator_admin,
                g1_compressed.0,
                g2_compressed.0,
                signature.0,
            )
            .await;

        assert!(result.is_err());

        Ok(())
    }
}
