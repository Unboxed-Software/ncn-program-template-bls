#[cfg(test)]
mod tests {
    use crate::fixtures::{test_builder::TestBuilder, TestResult};
    use ncn_program_core::{
        g1_point::G1CompressedPoint, g2_point::G2CompressedPoint, schemes::Sha256Normalized,
    };

    #[tokio::test]
    async fn test_update_operator_ip_socket_success() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;

        let mut restaking_program_client = fixture.restaking_program_client();
        let mut ncn_program_client = fixture.ncn_program_client();

        // Setup NCN
        let ncn_root = fixture.setup_ncn().await?;

        ncn_program_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin, None)
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
        ncn_program_client
            .do_full_initialize_snapshot(ncn_root.ncn_pubkey)
            .await?;

        // Generate BLS keypair
        let g1_compressed = G1CompressedPoint::try_from(operator_root.bn128_privkey).unwrap();
        let g2_compressed = G2CompressedPoint::try_from(&operator_root.bn128_privkey).unwrap();

        let signature = operator_root
            .bn128_privkey
            .sign::<Sha256Normalized, &[u8; 32]>(&g1_compressed.0)
            .unwrap();

        // Register the operator first
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

        // Verify initial IP address and socket are zeros
        let ncn_operator_account = ncn_program_client
            .get_ncn_operator_account(ncn_root.ncn_pubkey, operator_root.operator_pubkey)
            .await?;
        assert_eq!(ncn_operator_account.ip_address(), &[0u8; 16]);
        assert_eq!(ncn_operator_account.socket(), &[0u8; 16]);

        // Update IP address and socket
        let new_ip_address = [192, 168, 1, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let new_socket = [80, 80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        ncn_program_client
            .do_update_operator_ip_socket(
                ncn_root.ncn_pubkey,
                operator_root.operator_pubkey,
                &operator_root.operator_admin,
                new_ip_address,
                new_socket,
            )
            .await?;

        // Verify the update
        let updated_ncn_operator_account = ncn_program_client
            .get_ncn_operator_account(ncn_root.ncn_pubkey, operator_root.operator_pubkey)
            .await?;
        assert_eq!(updated_ncn_operator_account.ip_address(), &new_ip_address);
        assert_eq!(updated_ncn_operator_account.socket(), &new_socket);

        Ok(())
    }

    #[tokio::test]
    async fn test_update_operator_ip_socket_unauthorized() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;

        let mut restaking_program_client = fixture.restaking_program_client();
        let mut ncn_program_client = fixture.ncn_program_client();

        // Setup NCN
        let ncn_root = fixture.setup_ncn().await?;

        ncn_program_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin, None)
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
        ncn_program_client
            .do_full_initialize_snapshot(ncn_root.ncn_pubkey)
            .await?;

        // Generate BLS keypair
        let g1_compressed = G1CompressedPoint::try_from(operator_root.bn128_privkey).unwrap();
        let g2_compressed = G2CompressedPoint::try_from(&operator_root.bn128_privkey).unwrap();

        let signature = operator_root
            .bn128_privkey
            .sign::<Sha256Normalized, &[u8; 32]>(&g1_compressed.0)
            .unwrap();

        // Register the operator first
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

        // Setup another operator (unauthorized)
        let unauthorized_operator_root = restaking_program_client
            .do_initialize_operator(Some(200))
            .await?;

        // Try to update IP address and socket with unauthorized operator admin
        let new_ip_address = [192, 168, 1, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let new_socket = [80, 80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        let result = ncn_program_client
            .do_update_operator_ip_socket(
                ncn_root.ncn_pubkey,
                operator_root.operator_pubkey,
                &unauthorized_operator_root.operator_admin, // Wrong admin
                new_ip_address,
                new_socket,
            )
            .await;

        // Should fail with InvalidAccountData error
        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_update_operator_ip_socket_operator_not_registered() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;

        let mut restaking_program_client = fixture.restaking_program_client();
        let mut ncn_program_client = fixture.ncn_program_client();

        // Setup NCN
        let ncn_root = fixture.setup_ncn().await?;

        ncn_program_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin, None)
            .await?;

        // Setup operator but don't register it
        let operator_root = restaking_program_client
            .do_initialize_operator(Some(200))
            .await?;

        // Try to update IP address and socket for unregistered operator
        let new_ip_address = [192, 168, 1, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let new_socket = [80, 80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        let result = ncn_program_client
            .do_update_operator_ip_socket(
                ncn_root.ncn_pubkey,
                operator_root.operator_pubkey,
                &operator_root.operator_admin,
                new_ip_address,
                new_socket,
            )
            .await;

        // Should fail because the NCN operator account doesn't exist
        assert!(result.is_err());

        Ok(())
    }
}
