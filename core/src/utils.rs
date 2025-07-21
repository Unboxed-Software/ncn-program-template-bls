use solana_program::program_error::ProgramError;

use crate::constants::MODULUS;
use crate::{
    constants::MAX_REALLOC_BYTES, epoch_snapshot::OperatorSnapshot, error::NCNProgramError,
};
use dashu::integer::UBig;

/// Calculate new size for reallocation, capped at target size
/// Returns the minimum of (current_size + MAX_REALLOC_BYTES) and target_size
pub fn get_new_size(current_size: usize, target_size: usize) -> Result<usize, ProgramError> {
    Ok(current_size
        .checked_add(MAX_REALLOC_BYTES as usize)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .min(target_size))
}

#[inline(always)]
#[track_caller]
pub fn assert_ncn_program_error<T>(
    test_error: Result<T, NCNProgramError>,
    ncn_program_error: NCNProgramError,
) {
    assert!(test_error.is_err());
    assert_eq!(test_error.err().unwrap(), ncn_program_error);
}

/// Determines if an operator is eligible to vote in the current epoch
///
/// An operator can vote if:
/// 1. They haven't already voted in this epoch
/// 2. They have a non-zero stake weight
///
/// # Arguments
/// * `ballot_box` - The current epoch's ballot box tracking votes
/// * `operator_snapshot` - Snapshot of operator's state for this epoch
/// * `operator` - Public key of the operator to check
///
/// # Returns
/// * `bool` - True if operator can vote, false otherwise
pub fn can_operator_vote(operator_snapshot: OperatorSnapshot) -> bool {
    // Check if operator has already voted in this epoch

    operator_snapshot.is_active() && operator_snapshot.has_minimum_stake_weight()
}

/// Computes a scalar alpha by hashing together all prover-controlled inputs and reducing modulo the curve order.
/// Inputs should be provided as byte slices or arrays (e.g., message, signature, agg_pubkey, apk2).
/// Returns a 32-byte scalar (big-endian, mod curve order).
pub fn compute_alpha(
    message: &[u8; 64],
    signature: &[u8; 64],
    apk1: &[u8; 64],
    apk2: &[u8; 128],
) -> [u8; 32] {
    // Concatenate all inputs
    let mut input = Vec::with_capacity(message.len() + signature.len() + apk1.len() + apk2.len());
    input.extend_from_slice(message);
    input.extend_from_slice(signature);
    input.extend_from_slice(apk1);
    input.extend_from_slice(apk2);

    // Hash the concatenated input
    let hash = solana_nostd_sha256::hashv(&[&input]);

    // Convert hash to UBig and reduce modulo MODULUS
    let hash_ubig = UBig::from_be_bytes(&hash) % MODULUS.clone();
    let mut alpha_bytes = [0u8; 32];
    let hash_bytes = hash_ubig.to_be_bytes();
    // Copy to 32 bytes, pad with zeros if needed
    let pad = 32usize.saturating_sub(hash_bytes.len());
    if pad > 0 {
        alpha_bytes[..pad].fill(0);
        alpha_bytes[pad..].copy_from_slice(&hash_bytes);
    } else {
        alpha_bytes.copy_from_slice(&hash_bytes[hash_bytes.len() - 32..]);
    }
    alpha_bytes
}

/// Creates a bitmap representing which operators have signed, given their indices and the total number of operators.
/// Each bit in the bitmap corresponds to an operator: bit set to 1 means the operator at that index has signed.
///
/// # Arguments
/// * `signer_indices` - A slice of indices (usize) indicating which operators have signed.
/// * `total_operators` - The total number of operators (determines the bitmap length).
///
/// # Returns
/// A vector of bytes (`Vec<u8>`) where each bit represents the signing status of an operator.
pub fn create_signer_bitmap(non_signer_indices: &[usize], total_operators: usize) -> Vec<u8> {
    // Calculate the number of bytes needed to represent all operators (1 bit per operator).
    // Add 7 before dividing by 8 to ensure rounding up for any remainder bits.
    let bitmap_size = (total_operators + 7) / 8;
    // Initialize the bitmap with all bits set to 1 (all operators have signed).
    let mut bitmap = vec![255u8; bitmap_size];

    // Iterate over each index in non_signer_indices, setting the corresponding bit in the bitmap.
    for &index in non_signer_indices {
        // Determine which byte in the bitmap this operator's bit falls into.
        let byte_index = index / 8;
        // Determine the bit position within the byte (0 = least significant bit).
        let bit_index = index % 8;
        // Only set the bit if the byte_index is within the bitmap bounds.
        if byte_index < bitmap.len() {
            // Set the bit at bit_index in the byte at byte_index to 0.
            bitmap[byte_index] &= !(1 << bit_index);
        }
    }

    // Return the constructed bitmap.
    bitmap
}
