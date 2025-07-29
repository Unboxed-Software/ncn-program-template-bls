#[cfg(test)]
mod tests {
    use ncn_program_core::{
        constants::MAX_OPERATORS,
        error::NCNProgramError,
        g1_point::{G1CompressedPoint, G1Point},
        g2_point::{G2CompressedPoint, G2Point},
        schemes::Sha256Normalized,
        utils::create_signer_bitmap,
    };
    use rand::Rng;
    use solana_sdk::pubkey::Pubkey;
    use std::collections::HashSet;

    use crate::fixtures::{
        ncn_program_client::assert_ncn_program_error, test_builder::TestBuilder, TestResult,
    };

    pub fn get_random_none_signers_indecies(
        total_operators: usize,
        none_signers_count: usize,
    ) -> Vec<usize> {
        assert!(
            none_signers_count <= total_operators,
            "Cannot have more non-signers than total operators"
        );

        let mut rng = rand::rng();
        let mut none_signers_indices = HashSet::new();

        // Generate unique random indices
        while none_signers_indices.len() < none_signers_count {
            let index = rng.random_range(0..total_operators);
            none_signers_indices.insert(index);
        }

        // Convert to vector and sort for consistent output
        let mut result: Vec<usize> = none_signers_indices.into_iter().collect();
        result.sort();
        result
    }

    #[tokio::test]
    async fn test_cast_vote_multiple_signers() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut ncn_program_client = fixture.ncn_program_client();

        let test_ncn = fixture.create_initial_test_ncn(10, None).await?;

        ///// NCNProgram Setup /////
        fixture.warp_slot_incremental(1000).await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        let ncn = test_ncn.ncn_root.ncn_pubkey;

        // Create a test message to sign
        let message = solana_nostd_sha256::hashv(&[b"test message for multiple signers"]);

        let none_signers_indecies = get_random_none_signers_indecies(test_ncn.operators.len(), 2); // Let's say these operators didn't sign

        let mut signitures: Vec<G1Point> = vec![];
        let mut apk2_pubkeys: Vec<G2Point> = vec![];
        for (i, operator) in test_ncn.operators.iter().enumerate() {
            if !none_signers_indecies.contains(&i) {
                apk2_pubkeys.push(operator.bn128_g2_pubkey);
                let signature = operator
                    .bn128_privkey
                    .sign::<Sha256Normalized, &[u8; 32]>(&message)
                    .unwrap();
                signitures.push(signature);
            }
        }

        let apk2 = apk2_pubkeys.into_iter().reduce(|acc, x| acc + x).unwrap();
        let apk2 = G2CompressedPoint::try_from(&apk2).unwrap().0;

        let agg_sig = signitures.into_iter().reduce(|acc, x| acc + x).unwrap();
        let agg_sig = G1CompressedPoint::try_from(agg_sig).unwrap().0;

        // Create signers bitmap - all operators signed (bit 0 = 0 means they signed)
        let signers_bitmap = create_signer_bitmap(&none_signers_indecies, test_ncn.operators.len());

        // print the signers_bitmap as a binary string
        let mut binary_string = String::new();
        for byte in signers_bitmap.clone() {
            binary_string.push_str(&format!("{:08b}", byte));
        }
        println!("signers_bitmap: {}", binary_string);
        println!("apk2: {:?}", apk2);

        ncn_program_client
            .do_cast_vote(ncn, agg_sig, apk2, signers_bitmap, message)
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_cast_vote_not_enough_stake() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut vault_client = fixture.vault_client();

        let test_ncn = fixture.create_initial_test_ncn(10, None).await?;

        ///// NCNProgram Setup /////
        fixture.warp_slot_incremental(1000).await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        {
            // Remove stake from one operator to get it to below minimum stake
            let operator_index = 5;
            let operator_root = &test_ncn.operators[operator_index];

            vault_client
                .do_cooldown_delegation(&test_ncn.vaults[0], &operator_root.operator_pubkey, 99)
                .await?;

            fixture.warp_epoch_incremental(2).await?;

            fixture
                .update_snapshot_test_ncn_new_epoch(&test_ncn)
                .await?;
        }
        let none_signers_indecies: Vec<usize> = vec![1, 9];
        let result = fixture
            .cast_vote_for_test_ncn(&test_ncn, none_signers_indecies)
            .await;

        assert_ncn_program_error(result, NCNProgramError::OperatorHasNoMinimumStake, Some(1));

        Ok(())
    }

    #[tokio::test]
    async fn test_cast_vote_not_enough_stake_cant_vote_next_epoch() -> TestResult<()> {
        // Even if you don't take the snapshot, if the operator has less than the minimum stake, it
        // should not be able to vote
        let mut fixture = TestBuilder::new().await;
        let mut vault_client = fixture.vault_client();

        let test_ncn = fixture.create_initial_test_ncn(10, None).await?;

        ///// NCNProgram Setup /////
        fixture.warp_slot_incremental(1000).await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        {
            // Remove stake from one operator to get it to below minimum stake
            let operator_index = 5;
            let operator_root = &test_ncn.operators[operator_index];

            vault_client
                .do_cooldown_delegation(&test_ncn.vaults[0], &operator_root.operator_pubkey, 99)
                .await?;

            fixture.warp_epoch_incremental(1).await?;

            fixture
                .update_snapshot_test_ncn_new_epoch(&test_ncn)
                .await?;

            fixture.warp_epoch_incremental(1).await?;
        }
        let none_signers_indecies: Vec<usize> = vec![1, 9];

        let result = fixture
            .cast_vote_for_test_ncn(&test_ncn, none_signers_indecies)
            .await;

        assert_ncn_program_error(result, NCNProgramError::OperatorHasNoMinimumStake, Some(1));

        Ok(())
    }

    #[tokio::test]
    async fn test_cast_vote_operator_snapshot_outdated_error() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let test_ncn = fixture.create_initial_test_ncn(10, None).await?;

        ///// NCNProgram Setup /////
        fixture.warp_slot_incremental(1000).await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        fixture.warp_epoch_incremental(2).await?;

        let none_signers_indecies: Vec<usize> = vec![1, 9];
        let result = fixture
            .cast_vote_for_test_ncn(&test_ncn, none_signers_indecies)
            .await;

        assert_ncn_program_error(result, NCNProgramError::OperatorSnapshotOutdated, Some(1));

        Ok(())
    }

    #[tokio::test]
    async fn test_cast_vote_operator_snapshot_outdated_pass_if_not_signer() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut ncn_program_client = fixture.ncn_program_client();
        let mut vault_program_client = fixture.vault_program_client();
        let test_ncn = fixture.create_initial_test_ncn(10, None).await?;

        ///// NCNProgram Setup /////
        fixture.warp_slot_incremental(1000).await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        fixture.warp_epoch_incremental(2).await?;

        fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;
        fixture.add_weights_for_test_ncn(&test_ncn).await?;

        let clock = fixture.clock().await;
        let slot = clock.slot;
        let epoch = clock.epoch;
        let ncn = test_ncn.ncn_root.ncn_pubkey;

        let operators_to_skip_indexes = vec![1, 9];

        let operators_for_update = test_ncn
            .operators
            .iter()
            .map(|operator_root| operator_root.operator_pubkey)
            .collect::<Vec<Pubkey>>();
        let vault = test_ncn.vaults[0].vault_pubkey;

        let vault_is_update_needed = vault_program_client
            .get_vault_is_update_needed(&vault, slot)
            .await?;

        if vault_is_update_needed {
            vault_program_client
                .do_full_vault_update(&vault, &operators_for_update)
                .await?;
        }

        for (i, operator_root) in test_ncn.operators.iter().enumerate() {
            if operators_to_skip_indexes.contains(&i) {
                // Skip the operator that is not signing
                continue;
            }
            let operator = operator_root.operator_pubkey;

            let operator_snapshot = ncn_program_client
                .get_operator_snapshot(operator, ncn)
                .await?;

            // If operator snapshot is finalized, we should not take more snapshots, it is
            if !operator_snapshot.is_active() {
                continue;
            }

            ncn_program_client
                .do_snapshot_vault_operator_delegation(vault, operator, ncn, epoch)
                .await?;
        }

        fixture
            .cast_vote_for_test_ncn(&test_ncn, operators_to_skip_indexes)
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_cast_vote_operator_below_threshold_pass_if_not_signer() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut vault_client = fixture.vault_client();

        let test_ncn = fixture.create_initial_test_ncn(10, None).await?;

        ///// NCNProgram Setup /////
        fixture.warp_slot_incremental(1000).await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        // Remove stake from one operator to get it to below minimum stake
        let operator_index = 5;
        let operator_root = &test_ncn.operators[operator_index];

        vault_client
            .do_cooldown_delegation(&test_ncn.vaults[0], &operator_root.operator_pubkey, 99)
            .await?;

        fixture.warp_epoch_incremental(2).await?;

        fixture
            .update_snapshot_test_ncn_new_epoch(&test_ncn)
            .await?;
        let none_signers_indecies: Vec<usize> = vec![operator_index];
        fixture
            .cast_vote_for_test_ncn(&test_ncn, none_signers_indecies)
            .await?;

        Ok(())
    }

    #[ignore = "takes too long"]
    #[tokio::test]
    async fn test_cast_vote_multiple_signers_max_limits() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut ncn_program_client = fixture.ncn_program_client();

        let test_ncn = fixture.create_initial_test_ncn(MAX_OPERATORS, None).await?;

        ///// NCNProgram Setup /////
        fixture.warp_slot_incremental(1000).await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        let ncn = test_ncn.ncn_root.ncn_pubkey;

        // Create a test message to sign
        let message = solana_nostd_sha256::hashv(&[b"test message for multiple signers"]);

        let none_signers_indecies = get_random_none_signers_indecies(test_ncn.operators.len(), 85); // Let's say these operators didn't sign

        let mut signitures: Vec<G1Point> = vec![];
        let mut apk2_pubkeys: Vec<G2Point> = vec![];
        for (i, operator) in test_ncn.operators.iter().enumerate() {
            if !none_signers_indecies.contains(&i) {
                apk2_pubkeys.push(operator.bn128_g2_pubkey);
                let signature = operator
                    .bn128_privkey
                    .sign::<Sha256Normalized, &[u8; 32]>(&message)
                    .unwrap();
                signitures.push(signature);
            }
        }

        let apk2 = apk2_pubkeys.into_iter().reduce(|acc, x| acc + x).unwrap();
        let apk2 = G2CompressedPoint::try_from(&apk2).unwrap().0;

        let agg_sig = signitures.into_iter().reduce(|acc, x| acc + x).unwrap();
        let agg_sig = G1CompressedPoint::try_from(agg_sig).unwrap().0;

        // Create signers bitmap - all operators signed (bit 0 = 0 means they signed)
        let signers_bitmap = create_signer_bitmap(&none_signers_indecies, test_ncn.operators.len());

        // print the signers_bitmap as a binary string
        let mut binary_string = String::new();
        for byte in signers_bitmap.clone() {
            binary_string.push_str(&format!("{:08b}", byte));
        }
        println!("signers_bitmap: {}", binary_string);
        println!("apk2: {:?}", apk2);

        ncn_program_client
            .do_cast_vote(ncn, agg_sig, apk2, signers_bitmap, message)
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_cast_vote_multiple_signers_passing_wrong_bitmap() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut ncn_program_client = fixture.ncn_program_client();

        let test_ncn = fixture.create_initial_test_ncn(10, None).await?;

        ///// NCNProgram Setup /////
        fixture.warp_slot_incremental(1000).await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        let ncn = test_ncn.ncn_root.ncn_pubkey;

        // Create a test message to sign
        let message = solana_nostd_sha256::hashv(&[b"test message for multiple signers"]);

        let none_signers_indecies = get_random_none_signers_indecies(test_ncn.operators.len(), 2); // Let's say these operators didn't sign

        let mut signitures: Vec<G1Point> = vec![];
        let mut apk2_pubkeys: Vec<G2Point> = vec![];
        for (i, operator) in test_ncn.operators.iter().enumerate() {
            if !none_signers_indecies.contains(&i) {
                apk2_pubkeys.push(operator.bn128_g2_pubkey);
                let signature = operator
                    .bn128_privkey
                    .sign::<Sha256Normalized, &[u8; 32]>(&message)
                    .unwrap();
                signitures.push(signature);
            }
        }

        let apk2 = apk2_pubkeys.into_iter().reduce(|acc, x| acc + x).unwrap();
        let apk2 = G2CompressedPoint::try_from(&apk2).unwrap().0;

        let agg_sig = signitures.into_iter().reduce(|acc, x| acc + x).unwrap();
        let agg_sig = G1CompressedPoint::try_from(agg_sig).unwrap().0;

        // create a wrong bitmap
        let wrong_none_signers_indecies =
            get_random_none_signers_indecies(test_ncn.operators.len(), 3); // Let's say these operators didn't sign
        let wrong_signers_bitmap =
            create_signer_bitmap(&wrong_none_signers_indecies, test_ncn.operators.len());

        // print the signers_bitmap as a binary string
        let mut binary_string = String::new();
        for byte in wrong_signers_bitmap.clone() {
            binary_string.push_str(&format!("{:08b}", byte));
        }

        let result = ncn_program_client
            .do_cast_vote(ncn, agg_sig, apk2, wrong_signers_bitmap, message)
            .await;

        assert_ncn_program_error(
            result,
            NCNProgramError::SignatureVerificationFailed,
            Some(1),
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_cast_vote_invalid_signature_fails() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut ncn_program_client = fixture.ncn_program_client();

        let test_ncn = fixture.create_initial_test_ncn(10, None).await?;

        ///// NCNProgram Setup /////
        fixture.warp_slot_incremental(1000).await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        let ncn = test_ncn.ncn_root.ncn_pubkey;

        // Create a test message
        let message = solana_nostd_sha256::hashv(&[b"test message"]);

        // Use correct operator key but create invalid signature
        let operator_key = test_ncn.operators[0].bn128_privkey;
        let apk2 = G2CompressedPoint::try_from(&operator_key).unwrap().0;

        // Create an invalid signature (just random bytes)
        let agg_sig = [1u8; 32]; // Invalid signature

        let none_signers_indecies = get_random_none_signers_indecies(test_ncn.operators.len(), 0);
        // all have signed in the bitmap
        let signers_bitmap = create_signer_bitmap(&none_signers_indecies, test_ncn.operators.len());

        let result = ncn_program_client
            .do_cast_vote(ncn, agg_sig, apk2, signers_bitmap, message)
            .await;

        assert_ncn_program_error(
            result,
            NCNProgramError::SignatureVerificationFailed,
            Some(1),
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_cast_vote_invalid_bitmap_size_fails() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut ncn_program_client = fixture.ncn_program_client();

        let test_ncn = fixture.create_initial_test_ncn(1, None).await?;

        ///// NCNProgram Setup /////
        fixture.warp_slot_incremental(1000).await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        let ncn = test_ncn.ncn_root.ncn_pubkey;

        let message = solana_nostd_sha256::hashv(&[b"test message"]);
        let operator_key = test_ncn.operators[0].bn128_privkey;
        let signature = operator_key
            .sign::<Sha256Normalized, &[u8; 32]>(&message)
            .unwrap();
        let agg_sig = G1CompressedPoint::try_from(signature).unwrap().0;
        let apk2 = G2CompressedPoint::try_from(&operator_key).unwrap().0;

        // Wrong bitmap size - should be 1 byte for 1 operator, but provide 2 bytes
        let signers_bitmap = vec![0u8; 2];

        let result = ncn_program_client
            .do_cast_vote(ncn, agg_sig, apk2, signers_bitmap, message)
            .await;

        assert_ncn_program_error(result, NCNProgramError::InvalidInputLength, Some(1));

        Ok(())
    }

    #[tokio::test]
    async fn test_cast_vote_invalid_message_fails() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut ncn_program_client = fixture.ncn_program_client();

        let test_ncn = fixture.create_initial_test_ncn(10, None).await?;

        ///// NCNProgram Setup /////
        fixture.warp_slot_incremental(1000).await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        let ncn = test_ncn.ncn_root.ncn_pubkey;

        // Create a test message to sign
        let message = solana_nostd_sha256::hashv(&[b"test message for multiple signers"]);

        let none_signers_indecies = get_random_none_signers_indecies(test_ncn.operators.len(), 3); // Let's say these operators didn't sign

        let mut signitures: Vec<G1Point> = vec![];
        let mut apk2_pubkeys: Vec<G2Point> = vec![];
        for (i, operator) in test_ncn.operators.iter().enumerate() {
            if !none_signers_indecies.contains(&i) {
                apk2_pubkeys.push(operator.bn128_g2_pubkey);
                let signature = operator
                    .bn128_privkey
                    .sign::<Sha256Normalized, &[u8; 32]>(&message)
                    .unwrap();
                signitures.push(signature);
            }
        }

        let apk2 = apk2_pubkeys.into_iter().reduce(|acc, x| acc + x).unwrap();
        let apk2 = G2CompressedPoint::try_from(&apk2).unwrap().0;

        let agg_sig = signitures.into_iter().reduce(|acc, x| acc + x).unwrap();
        let agg_sig = G1CompressedPoint::try_from(agg_sig).unwrap().0;

        let signers_bitmap = create_signer_bitmap(&none_signers_indecies, test_ncn.operators.len());

        let wrong_message = solana_nostd_sha256::hashv(&[b"wrong message"]);

        let result = ncn_program_client
            .do_cast_vote(ncn, agg_sig, apk2, signers_bitmap, wrong_message)
            .await;

        assert_ncn_program_error(
            result,
            NCNProgramError::SignatureVerificationFailed,
            Some(1),
        );

        Ok(())
    }
}
