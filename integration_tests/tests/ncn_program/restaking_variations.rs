#[cfg(test)]
mod tests {

    use jito_restaking_core::MAX_FEE_BPS;
    use ncn_program_core::error::NCNProgramError;
    use solana_sdk::msg;

    use crate::fixtures::{
        ncn_program_client::assert_ncn_program_error, test_builder::TestBuilder, TestResult,
    };

    #[tokio::test]
    async fn test_removing_operator() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut restaking_client = fixture.restaking_program_client();

        const OPERATOR_COUNT: usize = 3;
        const OPERATOR_FEE_BPS: u16 = MAX_FEE_BPS;
        const INDEX_OF_OPERATOR_TO_REMOVE: usize = 1;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, Some(OPERATOR_FEE_BPS))
            .await?;

        {
            fixture.snapshot_test_ncn(&test_ncn).await?;

            fixture.vote_test_ncn(&test_ncn).await?;
        }

        {
            // Remove an operator
            let operator = test_ncn.operators[INDEX_OF_OPERATOR_TO_REMOVE].operator_pubkey;
            restaking_client
                .do_ncn_cooldown_operator(&test_ncn.ncn_root, &operator)
                .await?;

            // Warp to next epoch
            fixture.warp_epoch_incremental(1).await?;
        }

        {
            fixture
                .update_snapshot_test_ncn_new_epoch(&test_ncn)
                .await?;
            fixture.vote_test_ncn(&test_ncn).await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_removing_vault() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut restaking_client = fixture.restaking_program_client();
        let mut ncn_program_client = fixture.ncn_program_client();

        const OPERATOR_COUNT: usize = 1;
        const INDEX_OF_VAULT_TO_REMOVE: usize = 0;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, Some(0))
            .await?;

        {
            fixture.warp_epoch_incremental(2).await?;
        }

        {
            // First Run
            fixture.snapshot_test_ncn(&test_ncn).await?;

            fixture.vote_test_ncn(&test_ncn).await?;
        }

        {
            // Remove a vault
            let vault = test_ncn.vaults[INDEX_OF_VAULT_TO_REMOVE].vault_pubkey;
            restaking_client
                .do_cooldown_ncn_vault_ticket(&test_ncn.ncn_root, &vault)
                .await?;

            // Warp to next epoch
            fixture.warp_epoch_incremental(1).await?;
        }

        let epoch_snapshot = ncn_program_client
            .get_epoch_snapshot(test_ncn.ncn_root.ncn_pubkey)
            .await?;
        msg!("Epoch Snapshot before: {}", epoch_snapshot);

        {
            // Second Run
            fixture
                .update_snapshot_test_ncn_new_epoch(&test_ncn)
                .await?;

            let epoch_snapshot = ncn_program_client
                .get_epoch_snapshot(test_ncn.ncn_root.ncn_pubkey)
                .await?;
            msg!("Epoch Snapshot after: {}", epoch_snapshot);

            let result = fixture.vote_test_ncn(&test_ncn).await;
            assert_ncn_program_error(result, NCNProgramError::OperatorHasNoMinimumStake, Some(1));
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_stale_vault() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut ncn_program_client = fixture.ncn_program_client();

        const OPERATOR_COUNT: usize = 1;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, Some(0))
            .await?;

        {
            // Fast forward to a new epoch
            fixture.warp_epoch_incremental(1).await?;
        }

        fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;
        fixture.add_weights_for_test_ncn(&test_ncn).await?;
        fixture.add_epoch_snapshot_to_test_ncn(&test_ncn).await?;
        fixture
            .add_operator_snapshots_to_test_ncn(&test_ncn)
            .await?;
        {
            let epoch = fixture.clock().await.epoch;
            let ncn = test_ncn.ncn_root.ncn_pubkey;

            let operator = test_ncn.operators[0].operator_pubkey;
            let vault = test_ncn.vaults[0].vault_pubkey;

            let result = ncn_program_client
                .do_snapshot_vault_operator_delegation(vault, operator, ncn, epoch)
                .await;

            assert_ncn_program_error(result, NCNProgramError::VaultNeedsUpdate, None);
        }

        Ok(())
    }
}
