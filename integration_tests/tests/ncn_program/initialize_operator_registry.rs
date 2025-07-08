#[cfg(test)]
mod tests {
    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_initialize_operator_registry_ok() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        // fixture.initialize_restaking_and_vault_programs().await?;

        let mut ncn_program_client = fixture.ncn_program_client();
        let ncn_root = fixture.setup_ncn().await?;

        ncn_program_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;

        // Test that we can initialize the operator registry
        ncn_program_client
            .do_full_initialize_operator_registry(ncn_root.ncn_pubkey)
            .await?;

        // Verify the operator registry was created correctly
        let operator_registry = ncn_program_client
            .get_operator_registry(ncn_root.ncn_pubkey)
            .await?;

        assert_eq!(operator_registry.ncn, ncn_root.ncn_pubkey);
        assert_eq!(operator_registry.operator_count(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_initialize_operator_registry_double_init_fails() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;

        let mut ncn_program_client = fixture.ncn_program_client();
        let ncn_root = fixture.setup_ncn().await?;

        ncn_program_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;

        // Initialize once should succeed
        ncn_program_client
            .do_full_initialize_operator_registry(ncn_root.ncn_pubkey)
            .await?;

        // Initialize again should fail
        let result = ncn_program_client
            .do_full_initialize_operator_registry(ncn_root.ncn_pubkey)
            .await;

        assert!(result.is_err());

        Ok(())
    }
}
