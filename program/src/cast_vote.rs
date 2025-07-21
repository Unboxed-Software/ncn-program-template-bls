use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::ncn::Ncn;
use ncn_program_core::{
    config::Config as NcnConfig,
    constants::{G1_COMPRESSED_POINT_SIZE, G2_COMPRESSED_POINT_SIZE},
    epoch_snapshot::EpochSnapshot,
    epoch_state::EpochState,
    error::NCNProgramError,
    g1_point::{G1CompressedPoint, G1Point},
    g2_point::{G2CompressedPoint, G2Point},
    schemes::Sha256Normalized,
    stake_weight::StakeWeights,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

/// Allows an operator to cast a vote on weather status.
///
/// ### Parameters:
/// - `weather_status`: Status code for the vote (0=Sunny, 1=Cloudy, 2=Rainy)
/// - `epoch`: The target epoch
///
/// ### Accounts:
/// 1. `[writable]` epoch_state: The epoch state account for the target epoch
/// 2. `[]` config: NCN configuration account (named `ncn_config` in code)
/// 4. `[]` ncn: The NCN account
/// 5. `[]` epoch_snapshot: Epoch snapshot containing stake weights and operator snapshots
pub fn process_cast_vote(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
    apk2: [u8; G2_COMPRESSED_POINT_SIZE],
    agg_sig: [u8; G1_COMPRESSED_POINT_SIZE],
    non_signers_bitmap: Vec<u8>,
    message: [u8; 32],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let epoch_state = next_account_info(account_info_iter)?;
    let ncn_config = next_account_info(account_info_iter)?;
    let ncn = next_account_info(account_info_iter)?;
    let epoch_snapshot = next_account_info(account_info_iter)?;

    EpochState::load(program_id, epoch_state, ncn.key, epoch, false)?;
    NcnConfig::load(program_id, ncn_config, ncn.key, false)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    EpochSnapshot::load(program_id, epoch_snapshot, ncn.key, epoch, false)?;

    let epoch_snapshot_data = epoch_snapshot.data.borrow();
    let epoch_snapshot = EpochSnapshot::try_from_slice_unchecked(&epoch_snapshot_data)?;

    let total_stake_weights = {
        if !epoch_snapshot.finalized() {
            msg!("Error: Epoch snapshot not finalized for epoch: {}", epoch);
            return Err(NCNProgramError::EpochSnapshotNotFinalized.into());
        }

        StakeWeights::new(epoch_snapshot.operators_can_vote_count() as u128)
    };

    msg!("Total operators: {}", total_stake_weights.stake_weight());

    // let operator_stake_weights = StakeWeights::new(1);

    let slot = Clock::get()?.slot;
    msg!("Current slot: {}", slot);

    // Check bitmap size
    let required_bitmap_bytes = (epoch_snapshot.operators_registered() + 7) / 8;
    if non_signers_bitmap.len() as u64 != required_bitmap_bytes {
        msg!("Invalid bitmap size");
        return Err(NCNProgramError::InvalidInputLength.into());
    }

    msg!(
        "Bitmap is: {:?} , {:?}",
        non_signers_bitmap.len(),
        epoch_snapshot.operators_registered()
    );

    // Convert apk2 to G2Point
    let apk2_compressed_point = G2CompressedPoint::from(apk2);
    let apk2_point = G2Point::try_from(apk2_compressed_point)
        .map_err(|_| NCNProgramError::G2PointDecompressionError)?;

    // Aggregate the G1 public keys of operators who signed
    let mut aggregated_nonsigners_pubkey: Option<G1Point> = None;
    let mut non_signers_count = 0;

    for (i, operator_snapshot) in epoch_snapshot.operator_snapshots().iter().enumerate() {
        if i >= epoch_snapshot.operators_registered() as usize {
            break;
        }

        let byte_index = i / 8;
        let bit_index = i % 8;
        let signed = (non_signers_bitmap[byte_index] >> bit_index) & 1 == 1;

        if !signed {
            non_signers_count += 1;
            // msg!("Operator {} didn't signed", i);

            // Convert bytes to G1Point
            let g1_compressed = G1CompressedPoint::from(operator_snapshot.g1_pubkey());
            let g1_point = G1Point::try_from(&g1_compressed)
                .map_err(|_| NCNProgramError::G1PointDecompressionError)?;

            if aggregated_nonsigners_pubkey.is_none() {
                aggregated_nonsigners_pubkey = Some(g1_point);
            } else {
                // Add this G1 pubkey to the aggregate using G1Point addition
                let current = aggregated_nonsigners_pubkey.unwrap();
                aggregated_nonsigners_pubkey = Some(current + g1_point);
            }
        } else {
            // msg!("Operator {} did not sign", i);
        }
    }

    let total_agg_g1_pubkey_compressed =
        G1CompressedPoint::from(epoch_snapshot.total_agg_g1_pubkey());
    let total_agg_g1_pubkey = G1Point::try_from(&total_agg_g1_pubkey_compressed)
        .map_err(|_| NCNProgramError::G1PointDecompressionError)?;

    let signature_compressed = G1CompressedPoint(agg_sig);
    let signature = G1Point::try_from(&signature_compressed)
        .map_err(|_| NCNProgramError::G1PointDecompressionError)?;

    if non_signers_count == 0 {
        msg!("All operators signed, verifying aggregate signature with total G1 pubkey");
        apk2_point
            .verify_agg_signature::<Sha256Normalized, &[u8], G1Point>(
                signature,
                &message,
                total_agg_g1_pubkey,
            )
            .map_err(|_| NCNProgramError::SignatureVerificationFailed)?;
    } else {
        msg!("Total non signers: {}", non_signers_count);
        let aggregated_nonsigners_pubkey =
            aggregated_nonsigners_pubkey.ok_or(NCNProgramError::NoNonSignersAggregatedPubkey)?;

        let apk1 = total_agg_g1_pubkey + aggregated_nonsigners_pubkey.negate();

        msg!("Aggreged non-signers G1 pubkeys {:?}", apk1.0);
        msg!("Aggreged G2 {:?}", apk2_point.0);

        // One Pairing attempt
        msg!("Verifying aggregate signature one pairing");
        apk2_point
            .verify_agg_signature::<Sha256Normalized, &[u8], G1Point>(signature, &message, apk1)
            .map_err(|_| NCNProgramError::SignatureVerificationFailed)?;
    }

    Ok(())
}
