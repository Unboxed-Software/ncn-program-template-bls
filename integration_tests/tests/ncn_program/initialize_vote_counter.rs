#[cfg(test)]
mod tests {
    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    use ncn_program_core::vote_counter::VoteCounter;
    use solana_program_test::tokio;

    #[tokio::test]
    async fn test_vote_counter_module_exists() -> TestResult<()> {
        // This is a basic test to ensure our vote counter module is properly integrated
        // A full integration test would require adding the InitializeVoteCounter instruction
        // to the ncn_program_client, which would involve generated client updates

        let mut fixture = TestBuilder::new().await;
        let ncn_root = fixture.setup_ncn().await?;

        // Test that we can find the PDA address for the vote counter
        let (vote_counter_pda, bump, _seeds) =
            VoteCounter::find_program_address(&ncn_program::id(), &ncn_root.ncn_pubkey);

        // Verify the PDA generation works correctly
        assert_ne!(
            vote_counter_pda, ncn_root.ncn_pubkey,
            "Vote counter PDA should be different from NCN pubkey"
        );

        // Test VoteCounter creation
        let counter = VoteCounter::new(&ncn_root.ncn_pubkey, bump);
        assert_eq!(
            counter.ncn, ncn_root.ncn_pubkey,
            "Counter should belong to correct NCN"
        );
        assert_eq!(counter.count(), 0, "Counter should start at 0");
        assert_eq!(counter.bump, bump, "Counter should have correct bump");

        // Test counter increment
        let mut test_counter = counter;
        test_counter.increment()?;
        assert_eq!(test_counter.count(), 1, "Counter should increment to 1");

        test_counter.increment()?;
        assert_eq!(test_counter.count(), 2, "Counter should increment to 2");

        println!(
            "Vote counter module test passed! PDA: {}, bump: {}",
            vote_counter_pda, bump
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_initialize_vote_counter_client_method() -> TestResult<()> {
        // Test the NCN program client methods for vote counter
        let mut fixture = TestBuilder::new().await;
        let ncn_root = fixture.setup_ncn().await?;
        let mut ncn_program_client = fixture.ncn_program_client();

        // Initialize the NCN program (this creates the config account)
        ncn_program_client.setup_ncn_program(&ncn_root).await?;

        // Fetch the initialized vote counter
        let vote_counter = ncn_program_client
            .get_vote_counter(ncn_root.ncn_pubkey)
            .await?;

        // Verify the vote counter was initialized correctly
        assert_eq!(
            vote_counter.ncn, ncn_root.ncn_pubkey,
            "Vote counter should belong to correct NCN"
        );
        assert_eq!(vote_counter.count(), 0, "Vote counter should start at 0");

        println!(
            "Vote counter client methods test passed! NCN: {}, count: {}",
            vote_counter.ncn,
            vote_counter.count()
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_vote_counter_discriminator() -> TestResult<()> {
        use jito_bytemuck::Discriminator;
        use ncn_program_core::{discriminators::Discriminators, vote_counter::VoteCounter};

        // Test that the discriminator is set correctly
        assert_eq!(
            VoteCounter::DISCRIMINATOR,
            Discriminators::VoteCounter as u8
        );

        Ok(())
    }
}
