#[cfg(test)]
mod tests {

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_admin_update_weight_table() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut vault_client = fixture.vault_program_client();
        let mut ncn_program_client = fixture.ncn_program_client();

        let test_ncn = fixture.create_initial_test_ncn(1, None).await?;

        fixture.warp_slot_incremental(1000).await?;

        let clock = fixture.clock().await;
        let epoch = clock.epoch;

        ncn_program_client
            .do_intialize_epoch_state(test_ncn.ncn_root.ncn_pubkey, epoch)
            .await?;

        ncn_program_client
            .do_full_initialize_weight_table(test_ncn.ncn_root.ncn_pubkey, epoch)
            .await?;

        let vault_root = test_ncn.vaults[0].clone();
        let vault = vault_client.get_vault(&vault_root.vault_pubkey).await?;

        let mint = vault.supported_mint;
        let weight = 100;

        ncn_program_client
            .do_admin_set_weight(test_ncn.ncn_root.ncn_pubkey, epoch, mint, weight)
            .await?;

        Ok(())
    }
}
