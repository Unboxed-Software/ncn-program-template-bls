use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::loader::load_system_program;
use jito_restaking_core::ncn::Ncn;
use ncn_program_core::{account_payer::AccountPayer, config::Config, vote_counter::VoteCounter};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

/// Initializes the vote counter PDA for tracking successful votes
/// This counter is incremented each time a vote instruction passes
///
/// ### Accounts:
/// 1. `[]` config: The config account to verify the NCN is properly initialized
/// 2. `[writable]` vote_counter: The vote counter PDA to initialize `[seeds = [b"vote_counter", ncn.key().as_ref()], bump]`
/// 3. `[]` ncn: The NCN account this counter belongs to
/// 4. `[writable, signer]` account_payer: Account paying for the initialization and rent
/// 5. `[]` system_program: Solana System Program
pub fn process_initialize_vote_counter(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let config = next_account_info(account_info_iter)?;
    let vote_counter = next_account_info(account_info_iter)?;
    let ncn = next_account_info(account_info_iter)?;
    let account_payer = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Load and validate accounts
    load_system_program(system_program)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    Config::load(program_id, config, ncn.key, false)?;
    AccountPayer::load(program_id, account_payer, ncn.key, true)?;

    // Generate and validate the vote counter PDA
    let (vote_counter_pda, vote_counter_bump, mut vote_counter_seeds) =
        VoteCounter::find_program_address(program_id, ncn.key);
    vote_counter_seeds.push(vec![vote_counter_bump]);

    msg!(
        "Generated vote counter PDA: {}, bump: {}",
        vote_counter_pda,
        vote_counter_bump
    );

    if vote_counter_pda != *vote_counter.key {
        msg!(
            "Error: Invalid vote counter PDA. Expected: {}, got: {}",
            vote_counter_pda,
            vote_counter.key
        );
        return Err(ProgramError::InvalidSeeds);
    }

    // Create the vote counter account
    AccountPayer::pay_and_create_account(
        program_id,
        ncn.key,
        account_payer,
        vote_counter,
        system_program,
        program_id,
        VoteCounter::SIZE,
        &vote_counter_seeds,
    )?;

    // Initialize the vote counter data
    let mut vote_counter_data = vote_counter.try_borrow_mut_data()?;
    vote_counter_data[0] = VoteCounter::DISCRIMINATOR;
    let vote_counter_account = VoteCounter::try_from_slice_unchecked_mut(&mut vote_counter_data)?;

    *vote_counter_account = VoteCounter::new(ncn.key, vote_counter_bump);

    msg!(
        "Successfully initialized vote counter for NCN: {} with initial count: {}",
        ncn.key,
        vote_counter_account.count()
    );

    Ok(())
}
