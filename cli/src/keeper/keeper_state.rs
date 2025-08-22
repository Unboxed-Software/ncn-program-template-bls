use crate::{
    getters::{get_account, get_all_operators_in_ncn},
    handler::CliHandler,
};
use anyhow::{anyhow, Ok, Result};
use jito_bytemuck::AccountDeserialize;
use ncn_program_core::snapshot::Snapshot;
use solana_sdk::pubkey::Pubkey;

/// Manages the state of the keeper focused on snapshot operations
///
/// The KeeperState tracks the snapshot account from on-chain and determines
/// which operators need to be snapshotted based on their last snapshot epoch.
#[derive(Default, Debug)]
pub struct KeeperState {
    /// The epoch number this keeper state is tracking
    pub epoch: u64,
    /// The on-chain address of the Snapshot account
    pub snapshot_address: Pubkey,
    /// The deserialized Snapshot account data, if it exists
    pub snapshot: Option<Box<Snapshot>>,
    /// Flag indicating if snapshotting should occur
    pub should_snapshot: bool,
    /// List of operators that need to be snapshotted
    pub operators_to_snapshot: Vec<Pubkey>,
}

impl std::fmt::Display for KeeperState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "KeeperState {{")?;
        writeln!(f, "    epoch: {}", self.epoch)?;
        writeln!(f, "    snapshot_address: {}", self.snapshot_address)?;
        writeln!(f, "    snapshot: {:?}", self.snapshot.is_some())?;
        writeln!(f, "    should_snapshot: {}", self.should_snapshot)?;
        writeln!(
            f,
            "    operators_to_snapshot: {:?}",
            self.operators_to_snapshot
        )?;
        write!(f, "}}")
    }
}

impl KeeperState {
    /// Initializes the keeper state for snapshot management
    ///
    /// This method:
    /// 1. Calculates the snapshot address for the NCN
    /// 2. Updates the snapshot state from on-chain data
    /// 3. Sets the epoch number
    ///
    /// # Arguments
    /// * `handler` - The CLI handler containing RPC client and configuration
    /// * `epoch` - The epoch number to track
    pub async fn fetch(&mut self, handler: &CliHandler, epoch: u64) -> Result<()> {
        let ncn = *handler.ncn()?;

        // Calculate the program-derived address for the snapshot account
        let (snapshot_address, _, _) =
            Snapshot::find_program_address(&handler.ncn_program_id, &ncn);
        self.snapshot_address = snapshot_address;

        // Store the epoch number to ensure state consistency
        self.epoch = epoch;

        // Fetch the current state from on-chain
        self.update_state(handler).await?;

        Ok(())
    }

    /// Updates the snapshot state by fetching the latest data from the blockchain
    ///
    /// This method:
    /// 1. Fetches the snapshot account data from on-chain
    /// 2. Deserializes the account data if valid
    /// 3. Determines which operators need snapshotting
    /// 4. Sets the should_snapshot flag based on operator states
    ///
    /// # Arguments
    /// * `handler` - The CLI handler for blockchain interactions
    pub async fn update_state(&mut self, handler: &CliHandler) -> Result<()> {
        // Fetch the raw account data for the snapshot
        let raw_account = get_account(handler, &self.snapshot_address).await?;

        // If no account exists, the snapshot hasn't been created yet
        if raw_account.is_none() {
            self.snapshot = None;
            self.should_snapshot = false;
            self.operators_to_snapshot.clear();
            return Ok(());
        }

        let raw_account = raw_account.unwrap();

        // Validate that the account has sufficient data for a Snapshot
        if raw_account.data.len() < Snapshot::SIZE {
            self.snapshot = None;
            self.should_snapshot = false;
            self.operators_to_snapshot.clear();
            return Ok(());
        }

        // Deserialize the account data into a Snapshot struct
        let account_data = *Snapshot::try_from_slice_unchecked(raw_account.data.as_slice())?;
        self.snapshot = Some(Box::new(account_data));

        // Determine which operators need snapshotting
        self.update_operators_to_snapshot(handler).await?;

        Ok(())
    }

    /// Returns a reference to the snapshot, or an error if it doesn't exist
    ///
    /// # Returns
    /// A reference to the Snapshot if it exists, otherwise an error
    pub fn snapshot(&self) -> Result<&Snapshot> {
        self.snapshot
            .as_ref()
            .map(|boxed| boxed.as_ref())
            .ok_or_else(|| anyhow!("Snapshot does not exist"))
    }

    /// Updates the list of operators that need to be snapshotted
    ///
    /// This method determines which operators need snapshotting by checking if their
    /// last snapshot was taken in the previous epoch. If an operator was not snapshotted
    /// in the current epoch, it should be snapshotted.
    ///
    /// # Arguments
    /// * `handler` - The CLI handler for blockchain queries
    pub async fn update_operators_to_snapshot(&mut self, handler: &CliHandler) -> Result<()> {
        // Get all operators registered with the NCN
        let all_operators = get_all_operators_in_ncn(handler).await?;

        // Clear the current list
        self.operators_to_snapshot.clear();

        if let Some(snapshot) = &self.snapshot {
            // Check each operator to see if they need snapshotting
            for operator in all_operators {
                let needs_snapshot = self
                    .operator_needs_snapshot(&operator, snapshot, handler)
                    .await?;
                if needs_snapshot {
                    self.operators_to_snapshot.push(operator);
                }
            }
        } else {
            // If no snapshot exists, all operators need snapshotting
            self.operators_to_snapshot = all_operators;
        }

        // Update the should_snapshot flag
        self.should_snapshot = !self.operators_to_snapshot.is_empty();

        Ok(())
    }

    /// Checks if a specific operator needs to be snapshotted
    ///
    /// An operator needs snapshotting if:
    /// 1. It's not found in the current snapshot, OR
    /// 2. Its last snapshot was taken in a previous epoch
    ///
    /// # Arguments
    /// * `operator` - The operator pubkey to check
    /// * `snapshot` - The current snapshot to check against
    ///
    /// # Returns
    /// `true` if the operator needs snapshotting, `false` otherwise
    async fn operator_needs_snapshot(
        &self,
        operator: &Pubkey,
        snapshot: &Snapshot,
        handler: &CliHandler,
    ) -> Result<bool> {
        // Find the operator in the snapshot
        for operator_snapshot in snapshot.operator_snapshots() {
            if operator_snapshot.operator() == operator {
                // Check if the operator's last snapshot was in the previous epoch
                let last_snapshot_slot = operator_snapshot.last_snapshot_slot();
                // Get epoch length from restaking config
                let restaking_config = crate::getters::get_restaking_config(handler).await?;
                let last_snapshot_epoch = ncn_program_core::utils::get_epoch(
                    operator_snapshot.last_snapshot_slot(),
                    restaking_config.epoch_length(),
                )?;

                if operator_snapshot.is_active() && last_snapshot_epoch < self.epoch {
                    return Ok(true);
                } else {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Returns whether snapshotting should occur
    ///
    /// # Returns
    /// `true` if there are operators that need snapshotting, `false` otherwise
    pub fn get_should_snapshot(&self) -> bool {
        self.should_snapshot
    }

    /// Returns the list of operators that need to be snapshotted
    ///
    /// # Returns
    /// A reference to the vector of operator pubkeys that need snapshotting
    pub fn get_operators_to_snapshot(&self) -> &Vec<Pubkey> {
        &self.operators_to_snapshot
    }
}
