//! Block scanner for Orchard notes
//!
//! Scans the blockchain to find notes that belong to a viewing key
//! and maintains the commitment tree for spending.

#![allow(dead_code)]

use super::{
    constants::MIN_CONFIRMATIONS,
    keys::OrchardViewingKey,
    OrchardResult, ShieldedPool,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Orchard and cryptography imports for note decryption
use subtle::CtOption;

/// Progress information for blockchain scanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    /// Chain being scanned
    pub chain: String,

    /// Type of scan (orchard, sapling, etc.)
    pub scan_type: String,

    /// Last fully scanned block height
    pub last_scanned_height: u64,

    /// Current chain tip height
    pub chain_tip_height: u64,

    /// Percentage complete (0-100)
    pub progress_percent: f64,

    /// Estimated time remaining in seconds
    pub estimated_seconds_remaining: Option<u64>,

    /// Whether scanning is currently active
    pub is_scanning: bool,

    /// Number of notes found
    pub notes_found: u64,
}

impl ScanProgress {
    /// Create a new scan progress tracker
    pub fn new(chain: &str, scan_type: &str, birthday_height: u64, chain_tip: u64) -> Self {
        Self {
            chain: chain.to_string(),
            scan_type: scan_type.to_string(),
            last_scanned_height: birthday_height,
            chain_tip_height: chain_tip,
            progress_percent: 0.0,
            estimated_seconds_remaining: None,
            is_scanning: false,
            notes_found: 0,
        }
    }

    /// Update progress after scanning a range
    pub fn update(&mut self, scanned_to: u64, notes_found: u64, elapsed_secs: f64) {
        self.last_scanned_height = scanned_to;
        self.notes_found += notes_found;

        let total_blocks = self.chain_tip_height - self.last_scanned_height;
        let scanned_blocks = scanned_to - self.last_scanned_height;

        if total_blocks > 0 {
            self.progress_percent = (scanned_blocks as f64 / total_blocks as f64) * 100.0;

            if scanned_blocks > 0 && elapsed_secs > 0.0 {
                let blocks_per_sec = scanned_blocks as f64 / elapsed_secs;
                let remaining_blocks = self.chain_tip_height - scanned_to;
                self.estimated_seconds_remaining = Some((remaining_blocks as f64 / blocks_per_sec) as u64);
            }
        } else {
            self.progress_percent = 100.0;
            self.estimated_seconds_remaining = Some(0);
        }
    }

    /// Mark scan as complete
    pub fn complete(&mut self) {
        self.progress_percent = 100.0;
        self.estimated_seconds_remaining = Some(0);
        self.is_scanning = false;
    }
}

/// An Orchard note (received shielded funds)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchardNote {
    /// Unique note ID (internal)
    pub id: Option<i64>,

    /// Account ID this note belongs to
    pub account_id: u32,

    /// Transaction hash where this note was received
    pub tx_hash: String,

    /// Block height where confirmed
    pub block_height: u64,

    /// Note commitment
    #[serde(with = "hex_array")]
    pub note_commitment: [u8; 32],

    /// Nullifier (used to mark as spent)
    #[serde(with = "hex_array")]
    pub nullifier: [u8; 32],

    /// Value in zatoshis
    pub value_zatoshis: u64,

    /// Position in the commitment tree
    pub position: u64,

    /// Whether this note has been spent
    pub is_spent: bool,

    /// Decrypted memo (if any)
    pub memo: Option<String>,

    /// Merkle path for spending (populated when needed)
    #[serde(skip)]
    pub merkle_path: Option<Vec<[u8; 32]>>,
}

/// Hex serialization for fixed-size arrays
mod hex_array {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
        if bytes.len() != 32 {
            return Err(serde::de::Error::custom("Invalid length"));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(arr)
    }
}

/// Information about a spent note detected during scanning
#[derive(Debug, Clone)]
pub struct SpentNoteInfo {
    /// The nullifier of the spent note
    pub nullifier: [u8; 32],
    /// The transaction hash where the note was spent
    pub spent_in_tx: String,
    /// The block height where the spend was detected
    pub block_height: u64,
}

/// Scanner for detecting Orchard notes in blocks
pub struct OrchardScanner {
    /// Viewing keys to scan for
    viewing_keys: Vec<OrchardViewingKey>,

    /// Commitment tree for the Orchard pool
    commitment_tree: CommitmentTree,

    /// Scanned notes by account
    notes: HashMap<u32, Vec<OrchardNote>>,

    /// Known nullifiers (spent notes)
    spent_nullifiers: Vec<[u8; 32]>,

    /// Newly detected spent notes (cleared after retrieval)
    newly_spent_notes: Vec<SpentNoteInfo>,

    /// Scan progress
    progress: ScanProgress,
}

/// Simplified commitment tree for tracking note positions
struct CommitmentTree {
    /// All commitments in order
    commitments: Vec<[u8; 32]>,

    /// Tree depth
    depth: usize,

    /// Cached tree roots at various positions
    cached_roots: HashMap<u64, [u8; 32]>,
}

impl CommitmentTree {
    fn new() -> Self {
        Self {
            commitments: Vec::new(),
            depth: 32, // Orchard tree depth
            cached_roots: HashMap::new(),
        }
    }

    /// Append a commitment and return its position
    fn append(&mut self, commitment: [u8; 32]) -> u64 {
        let position = self.commitments.len() as u64;
        self.commitments.push(commitment);
        position
    }

    /// Get the current root of the tree
    fn root(&self) -> [u8; 32] {
        if self.commitments.is_empty() {
            return [0u8; 32]; // Empty tree root
        }

        // Simplified root calculation using Blake2b
        let mut hasher = blake2b_simd::Params::new()
            .hash_length(32)
            .personal(b"ZcashOrchardRoot")
            .to_state();

        for commitment in &self.commitments {
            hasher.update(commitment);
        }

        let result = hasher.finalize();
        let mut root = [0u8; 32];
        root.copy_from_slice(result.as_bytes());
        root
    }

    /// Get the merkle path for a commitment at a given position
    fn merkle_path(&self, position: u64) -> Option<Vec<[u8; 32]>> {
        if position as usize >= self.commitments.len() {
            return None;
        }

        // Simplified merkle path - in production, use proper incremental merkle tree
        let mut path = Vec::with_capacity(self.depth);

        for i in 0..self.depth {
            let mut sibling = [0u8; 32];

            let sibling_pos = if (position >> i) & 1 == 0 {
                position + (1 << i)
            } else {
                position - (1 << i)
            };

            if (sibling_pos as usize) < self.commitments.len() {
                sibling = self.commitments[sibling_pos as usize];
            }

            path.push(sibling);
        }

        Some(path)
    }
}

impl OrchardScanner {
    /// Create a new scanner with the given viewing keys
    pub fn new(viewing_keys: Vec<OrchardViewingKey>) -> Self {
        // Get the minimum birthday height from all viewing keys
        // This ensures we start scanning from the earliest possible block where notes could exist
        let birthday_height = viewing_keys
            .iter()
            .map(|vk| vk.birthday_height)
            .min()
            .unwrap_or(2_000_000); // Default to Orchard activation height if no keys

        tracing::debug!(
            "[OrchardScanner] Created with {} viewing keys, birthday_height={}",
            viewing_keys.len(),
            birthday_height
        );

        Self {
            viewing_keys,
            commitment_tree: CommitmentTree::new(),
            notes: HashMap::new(),
            spent_nullifiers: Vec::new(),
            newly_spent_notes: Vec::new(),
            progress: ScanProgress::new("zcash", "orchard", birthday_height, 0),
        }
    }

    /// Get current scan progress
    pub fn progress(&self) -> &ScanProgress {
        &self.progress
    }

    /// Get all unspent notes for an account
    pub fn get_unspent_notes(&self, account_id: u32) -> Vec<OrchardNote> {
        self.notes
            .get(&account_id)
            .map(|notes| {
                notes
                    .iter()
                    .filter(|n| !n.is_spent)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get spendable notes (confirmed and not spent)
    pub fn get_spendable_notes(&self, account_id: u32, current_height: u64) -> Vec<OrchardNote> {
        self.notes
            .get(&account_id)
            .map(|notes| {
                notes
                    .iter()
                    .filter(|n| {
                        !n.is_spent
                            && current_height >= n.block_height + MIN_CONFIRMATIONS as u64
                    })
                    .cloned()
                    .map(|mut note| {
                        // Populate merkle path
                        note.merkle_path = self.commitment_tree.merkle_path(note.position);
                        note
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get total shielded balance for an account
    pub fn get_balance(&self, account_id: u32) -> u64 {
        self.notes
            .get(&account_id)
            .map(|notes| notes.iter().filter(|n| !n.is_spent).map(|n| n.value_zatoshis).sum())
            .unwrap_or(0)
    }

    /// Get spendable balance (confirmed notes only)
    pub fn get_spendable_balance(&self, account_id: u32, current_height: u64) -> u64 {
        self.notes
            .get(&account_id)
            .map(|notes| {
                notes
                    .iter()
                    .filter(|n| {
                        !n.is_spent
                            && current_height >= n.block_height + MIN_CONFIRMATIONS as u64
                    })
                    .map(|n| n.value_zatoshis)
                    .sum()
            })
            .unwrap_or(0)
    }

    /// Scan a range of blocks
    pub async fn scan_blocks(
        &mut self,
        blocks: Vec<CompactBlock>,
        chain_tip: u64,
    ) -> OrchardResult<Vec<OrchardNote>> {
        let start_time = std::time::Instant::now();
        let mut found_notes = Vec::new();
        let block_count = blocks.len();
        let first_height = blocks.first().map(|b| b.height).unwrap_or(0);
        let last_height = blocks.last().map(|b| b.height).unwrap_or(0);

        self.progress.chain_tip_height = chain_tip;
        self.progress.is_scanning = true;

        let viewing_key_count = self.viewing_keys.len();
        let mut total_actions = 0usize;
        let mut total_txs = 0usize;

        tracing::debug!(
            "[Orchard Scan] Starting scan of {} blocks ({}-{}), {} viewing keys registered",
            block_count,
            first_height,
            last_height,
            viewing_key_count
        );

        for block in blocks {
            let block_tx_count = block.transactions.len();
            let block_action_count: usize = block.transactions.iter().map(|tx| tx.orchard_actions.len()).sum();

            if block_action_count > 0 {
                tracing::trace!(
                    "[Orchard Scan] Block {}: {} txs, {} orchard actions",
                    block.height,
                    block_tx_count,
                    block_action_count
                );
            }

            total_txs += block_tx_count;

            // Process each transaction in the block
            for tx in &block.transactions {
                // Process Orchard actions
                for action in &tx.orchard_actions {
                    total_actions += 1;

                    // Add commitment to tree
                    let position = self.commitment_tree.append(action.cmx);

                    // Try to decrypt with each viewing key
                    for vk in &self.viewing_keys {
                        if let Some(note) = self.try_decrypt_note(
                            vk,
                            action,
                            &tx.hash,
                            block.height,
                            position,
                        ) {
                            tracing::info!(
                                "[Orchard Scan] 🎉 Found note! block={}, tx={}, value={} zatoshis ({:.8} ZEC), account={}",
                                block.height,
                                &tx.hash[..16],
                                note.value_zatoshis,
                                note.value_zatoshis as f64 / 100_000_000.0,
                                vk.account_index
                            );
                            found_notes.push(note.clone());

                            // Store the note
                            self.notes
                                .entry(vk.account_index)
                                .or_insert_with(Vec::new)
                                .push(note);
                        }
                    }

                    // Check for spent notes
                    self.check_spent_nullifier(&action.nullifier, &tx.hash, block.height);
                }
            }

            self.progress.last_scanned_height = block.height;
        }

        let elapsed = start_time.elapsed().as_secs_f64();
        self.progress.update(
            self.progress.last_scanned_height,
            found_notes.len() as u64,
            elapsed,
        );

        if self.progress.last_scanned_height >= chain_tip {
            self.progress.complete();
        }

        tracing::info!(
            "[Orchard Scan] Completed: {} blocks ({}-{}), {} txs, {} actions scanned, {} notes found, {:.2}s elapsed",
            block_count,
            first_height,
            last_height,
            total_txs,
            total_actions,
            found_notes.len(),
            elapsed
        );

        Ok(found_notes)
    }

    /// Try to decrypt a note with a viewing key using official Orchard decryption API
    fn try_decrypt_note(
        &self,
        viewing_key: &OrchardViewingKey,
        action: &CompactOrchardAction,
        tx_hash: &str,
        block_height: u64,
        position: u64,
    ) -> Option<OrchardNote> {
        use orchard::keys::PreparedIncomingViewingKey;
        use orchard::note::{ExtractedNoteCommitment, Nullifier};
        use orchard::note_encryption::{CompactAction, OrchardDomain};
        use zcash_note_encryption::{batch, EphemeralKeyBytes, COMPACT_NOTE_SIZE};

        // Ensure ciphertext is long enough for compact decryption (52 bytes)
        if action.ciphertext.len() < COMPACT_NOTE_SIZE {
            tracing::trace!("[Decrypt] Ciphertext too short: {} < {}", action.ciphertext.len(), COMPACT_NOTE_SIZE);
            return None;
        }

        // Parse nullifier
        let nullifier = {
            let nf_option: CtOption<Nullifier> = Nullifier::from_bytes(&action.nullifier);
            if bool::from(nf_option.is_some()) {
                nf_option.unwrap()
            } else {
                tracing::trace!("[Decrypt] Invalid nullifier bytes");
                return None;
            }
        };

        // Parse note commitment (cmx)
        let cmx = {
            let cmx_option: CtOption<ExtractedNoteCommitment> = ExtractedNoteCommitment::from_bytes(&action.cmx);
            if bool::from(cmx_option.is_some()) {
                cmx_option.unwrap()
            } else {
                tracing::trace!("[Decrypt] Invalid cmx bytes");
                return None;
            }
        };

        // Create compact ciphertext array
        let enc_ciphertext: [u8; COMPACT_NOTE_SIZE] = action.ciphertext[..COMPACT_NOTE_SIZE]
            .try_into()
            .ok()?;

        // Get the incoming viewing key from the full viewing key
        let fvk = viewing_key.fvk();

        // Try External scope first, then Internal scope
        for scope in [orchard::keys::Scope::External, orchard::keys::Scope::Internal] {
            let ivk = fvk.to_ivk(scope);
            let prepared_ivk = PreparedIncomingViewingKey::new(&ivk);

            // Recreate CompactAction and domain for each attempt
            let compact_action_for_scope = CompactAction::from_parts(
                nullifier,
                cmx,
                EphemeralKeyBytes(action.ephemeral_key),
                enc_ciphertext,
            );
            let domain_for_scope = OrchardDomain::for_compact_action(&compact_action_for_scope);

            // Use the official batch decryption API
            // batch::try_compact_note_decryption returns Vec<Option<((Note, Address), ivk_index)>>
            let results = batch::try_compact_note_decryption(
                &[prepared_ivk],
                &[(domain_for_scope, compact_action_for_scope)],
            );

            if let Some(Some(((note, _recipient), _ivk_idx))) = results.into_iter().next() {
                // Successfully decrypted the note!
                let value_zatoshis = note.value().inner();

                // Compute the nullifier using the full viewing key
                let note_nullifier = note.nullifier(fvk);

                tracing::debug!(
                    "[Decrypt] ✅ Successfully decrypted note: value={} zatoshis, scope={:?}",
                    value_zatoshis,
                    scope
                );

                return Some(OrchardNote {
                    id: None,
                    account_id: viewing_key.account_index,
                    tx_hash: tx_hash.to_string(),
                    block_height,
                    note_commitment: action.cmx,
                    nullifier: note_nullifier.to_bytes(),
                    value_zatoshis,
                    position,
                    is_spent: false,
                    memo: None, // Compact blocks don't include memo
                    merkle_path: None,
                });
            }
        }

        // Decryption failed for all scopes - this note doesn't belong to this viewing key
        None
    }

    /// Check if a nullifier corresponds to one of our notes
    fn check_spent_nullifier(&mut self, nullifier: &[u8; 32], tx_hash: &str, block_height: u64) {
        // Check all notes
        for notes in self.notes.values_mut() {
            for note in notes.iter_mut() {
                if &note.nullifier == nullifier && !note.is_spent {
                    note.is_spent = true;
                    self.spent_nullifiers.push(*nullifier);

                    // Record this newly spent note for database sync
                    self.newly_spent_notes.push(SpentNoteInfo {
                        nullifier: *nullifier,
                        spent_in_tx: tx_hash.to_string(),
                        block_height,
                    });

                    tracing::info!(
                        "Note spent: {} zatoshis, nullifier: {}, tx: {}",
                        note.value_zatoshis,
                        hex::encode(nullifier),
                        tx_hash
                    );
                }
            }
        }
    }

    /// Get and clear newly detected spent notes
    /// Call this after scan_blocks to get the list of notes that were marked as spent
    pub fn take_newly_spent_notes(&mut self) -> Vec<SpentNoteInfo> {
        std::mem::take(&mut self.newly_spent_notes)
    }

    /// Get the current commitment tree anchor
    pub fn get_anchor(&self) -> [u8; 32] {
        self.commitment_tree.root()
    }

    /// Mark a note as spent by nullifier
    pub fn mark_spent(&mut self, nullifier: &[u8; 32], tx_hash: &str, block_height: u64) {
        self.check_spent_nullifier(nullifier, tx_hash, block_height);
    }
}

/// Compact block data for scanning
#[derive(Debug, Clone)]
pub struct CompactBlock {
    /// Block height
    pub height: u64,
    /// Block hash
    pub hash: [u8; 32],
    /// Transactions in this block
    pub transactions: Vec<CompactTransaction>,
}

/// Compact transaction data
#[derive(Debug, Clone)]
pub struct CompactTransaction {
    /// Transaction hash
    pub hash: String,
    /// Orchard actions
    pub orchard_actions: Vec<CompactOrchardAction>,
}

/// Compact Orchard action data
#[derive(Debug, Clone)]
pub struct CompactOrchardAction {
    /// Note commitment
    pub cmx: [u8; 32],
    /// Nullifier
    pub nullifier: [u8; 32],
    /// Ephemeral key
    pub ephemeral_key: [u8; 32],
    /// Encrypted note ciphertext
    pub ciphertext: Vec<u8>,
}


/// Balance breakdown by pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShieldedBalance {
    /// Total balance in zatoshis
    pub total_zatoshis: u64,

    /// Spendable balance (confirmed) in zatoshis
    pub spendable_zatoshis: u64,

    /// Pending balance (unconfirmed) in zatoshis
    pub pending_zatoshis: u64,

    /// Number of unspent notes
    pub note_count: u32,

    /// Pool type
    pub pool: ShieldedPool,
}

impl ShieldedBalance {
    /// Create a new balance summary
    pub fn new(pool: ShieldedPool, total: u64, spendable: u64, note_count: u32) -> Self {
        Self {
            total_zatoshis: total,
            spendable_zatoshis: spendable,
            pending_zatoshis: total - spendable,
            note_count,
            pool,
        }
    }

    /// Get balance in ZEC (decimal)
    pub fn total_zec(&self) -> f64 {
        self.total_zatoshis as f64 / 100_000_000.0
    }

    /// Get spendable balance in ZEC
    pub fn spendable_zec(&self) -> f64 {
        self.spendable_zatoshis as f64 / 100_000_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::zcash::orchard::keys::OrchardKeyManager;

    #[test]
    fn test_scan_progress() {
        let mut progress = ScanProgress::new("zcash", "orchard", 2000000, 2500000);

        assert_eq!(progress.progress_percent, 0.0);
        assert!(!progress.is_scanning);

        progress.is_scanning = true;
        progress.update(2250000, 5, 10.0);

        assert!(progress.progress_percent > 0.0);
        assert_eq!(progress.notes_found, 5);
    }

    #[test]
    fn test_commitment_tree() {
        let mut tree = CommitmentTree::new();

        let cm1 = [1u8; 32];
        let cm2 = [2u8; 32];

        let pos1 = tree.append(cm1);
        let pos2 = tree.append(cm2);

        assert_eq!(pos1, 0);
        assert_eq!(pos2, 1);

        let root = tree.root();
        assert_ne!(root, [0u8; 32]);
    }

    #[test]
    fn test_scanner_balance() {
        let seed = vec![0u8; 64];
        let (_, vk) = OrchardKeyManager::derive_from_seed(&seed, 0, 2000000).unwrap();

        let scanner = OrchardScanner::new(vec![vk]);

        assert_eq!(scanner.get_balance(0), 0);
        assert_eq!(scanner.get_spendable_balance(0, 2500000), 0);
    }
}
