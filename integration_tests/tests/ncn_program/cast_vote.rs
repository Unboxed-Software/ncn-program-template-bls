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

        let test_ncn = fixture.create_initial_test_ncn(100, 1, None).await?;

        ///// NCNProgram Setup /////
        fixture.warp_slot_incremental(1000).await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        let clock = fixture.clock().await;
        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let epoch = clock.epoch;

        // Create a test message to sign
        let message = solana_nostd_sha256::hashv(&[b"test message for multiple signers"]);

        let none_signers_indecies = get_random_none_signers_indecies(test_ncn.operators.len(), 10); // Let's say these operators didn't sign

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
            .do_cast_vote(ncn, epoch, agg_sig, apk2, signers_bitmap, message)
            .await?;

        Ok(())
    }

    #[ignore = "takes too long"]
    #[tokio::test]
    async fn test_cast_vote_multiple_signers_max_limits() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut ncn_program_client = fixture.ncn_program_client();

        let test_ncn = fixture
            .create_initial_test_ncn(MAX_OPERATORS, 10, None)
            .await?;

        ///// NCNProgram Setup /////
        fixture.warp_slot_incremental(1000).await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        let clock = fixture.clock().await;
        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let epoch = clock.epoch;

        // Create a test message to sign
        let message = solana_nostd_sha256::hashv(&[b"test message for multiple signers"]);

        let none_signers_indecies = get_random_none_signers_indecies(test_ncn.operators.len(), 100); // Let's say these operators didn't sign

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
            .do_cast_vote(ncn, epoch, agg_sig, apk2, signers_bitmap, message)
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_cast_vote_multiple_signers_passing_wrong_bitmap() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut ncn_program_client = fixture.ncn_program_client();

        let test_ncn = fixture.create_initial_test_ncn(10, 1, None).await?;

        ///// NCNProgram Setup /////
        fixture.warp_slot_incremental(1000).await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        let clock = fixture.clock().await;
        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let epoch = clock.epoch;

        // Create a test message to sign
        let message = solana_nostd_sha256::hashv(&[b"test message for multiple signers"]);

        let none_signers_indecies = get_random_none_signers_indecies(test_ncn.operators.len(), 5); // Let's say these operators didn't sign

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
            get_random_none_signers_indecies(test_ncn.operators.len(), 2); // Let's say these operators didn't sign
        let wrong_signers_bitmap =
            create_signer_bitmap(&wrong_none_signers_indecies, test_ncn.operators.len());

        // print the signers_bitmap as a binary string
        let mut binary_string = String::new();
        for byte in wrong_signers_bitmap.clone() {
            binary_string.push_str(&format!("{:08b}", byte));
        }

        let result = ncn_program_client
            .do_cast_vote(ncn, epoch, agg_sig, apk2, wrong_signers_bitmap, message)
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

        let test_ncn = fixture.create_initial_test_ncn(1, 1, None).await?;

        ///// NCNProgram Setup /////
        fixture.warp_slot_incremental(1000).await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        let clock = fixture.clock().await;
        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let epoch = clock.epoch;

        // Create a test message
        let message = solana_nostd_sha256::hashv(&[b"test message"]);

        // Use correct operator key but create invalid signature
        let operator_key = test_ncn.operators[0].bn128_privkey;
        let apk2 = G2CompressedPoint::try_from(&operator_key).unwrap().0;

        // Create an invalid signature (just random bytes)
        let agg_sig = [1u8; 32]; // Invalid signature

        let signers_bitmap = vec![0u8; 1]; // All signed

        let result = ncn_program_client
            .do_cast_vote(ncn, epoch, agg_sig, apk2, signers_bitmap, message)
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

        let test_ncn = fixture.create_initial_test_ncn(1, 1, None).await?;

        ///// NCNProgram Setup /////
        fixture.warp_slot_incremental(1000).await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        let clock = fixture.clock().await;
        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let epoch = clock.epoch;

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
            .do_cast_vote(ncn, epoch, agg_sig, apk2, signers_bitmap, message)
            .await;

        assert_ncn_program_error(result, NCNProgramError::InvalidInputLength, Some(1));

        Ok(())
    }

    #[tokio::test]
    async fn test_cast_vote_invalid_message_fails() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut ncn_program_client = fixture.ncn_program_client();

        let test_ncn = fixture.create_initial_test_ncn(10, 1, None).await?;

        ///// NCNProgram Setup /////
        fixture.warp_slot_incremental(1000).await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        let clock = fixture.clock().await;
        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let epoch = clock.epoch;

        // Create a test message to sign
        let message = solana_nostd_sha256::hashv(&[b"test message for multiple signers"]);

        let none_signers_indecies = get_random_none_signers_indecies(test_ncn.operators.len(), 5); // Let's say these operators didn't sign

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
            .do_cast_vote(ncn, epoch, agg_sig, apk2, signers_bitmap, wrong_message)
            .await;

        assert_ncn_program_error(
            result,
            NCNProgramError::SignatureVerificationFailed,
            Some(1),
        );

        Ok(())
    }
}
