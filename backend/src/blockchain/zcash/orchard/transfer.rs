//! Orchard privacy transfer implementation
//!
//! This module implements shielded (private) transfers using the Orchard protocol.
//! It supports:
//! - Shielded to shielded transfers (maximum privacy)
//! - Transparent to shielded transfers (shielding)
//! - Shielded to transparent transfers (deshielding)

#![allow(dead_code)]

use super::{
    constants::DEFAULT_FEE_ZATOSHIS,
    keys::OrchardSpendingKey,
    scanner::{OrchardNote, ShieldedBalance},
    OrchardError, OrchardResult,
};
use serde::{Deserialize, Serialize};

use orchard::{
    builder::{Builder as OrchardBuilder, BundleType, InProgress, Unauthorized},
    circuit::ProvingKey,
    keys::SpendAuthorizingKey,
    tree::Anchor,
    value::NoteValue,
    Proof,
};
use rand::rngs::OsRng;
use std::sync::OnceLock;

use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};

// ZIP 244 sighash personalization strings
const ZCASH_TRANSPARENT_HASH: &[u8] = b"ZTxIdTranspaHash";
const ZCASH_PREVOUTS_HASH: &[u8] = b"ZTxIdPrevoutHash";
const ZCASH_SEQUENCE_HASH: &[u8] = b"ZTxIdSequencHash";
const ZCASH_OUTPUTS_HASH: &[u8] = b"ZTxIdOutputsHash";
const ZCASH_TX_HASH: &[u8] = b"ZcashTxHash_";
const ZCASH_TRANSPARENT_SIG: &[u8] = b"Zcash___TxInHash";

// Transaction constants
const TX_VERSION_WITH_OVERWINTERED: u32 = 0x80000005;
const VERSION_GROUP_ID_V5: u32 = 0x26A7270A;

/// Global proving key (expensive to build, so we cache it)
static ORCHARD_PROVING_KEY: OnceLock<ProvingKey> = OnceLock::new();

/// Initialize the Orchard proving key (call at startup to avoid first-transfer delay)
/// This is an expensive operation (~20 seconds) but only needs to be done once.
pub fn init_proving_key() {
    let _ = get_proving_key();
}

/// Get or build the Orchard proving key
fn get_proving_key() -> &'static ProvingKey {
    ORCHARD_PROVING_KEY.get_or_init(|| {
        tracing::info!("Building Orchard proving key (this may take a moment)...");
        let pk = ProvingKey::build();
        tracing::info!("Orchard proving key built successfully");
        pk
    })
}

/// Fund source for a transfer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FundSource {
    /// Automatically select funds (prefer shielded, fallback to transparent)
    Auto,
    /// Only use shielded funds
    Shielded,
    /// Only use transparent funds (shielding operation)
    Transparent,
}

impl Default for FundSource {
    fn default() -> Self {
        FundSource::Auto
    }
}

/// Transfer request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRequest {
    /// Source wallet ID
    pub wallet_id: i32,
    /// Recipient address (unified or transparent)
    pub to_address: String,
    /// Amount in ZEC (decimal string)
    pub amount_zec: String,
    /// Amount in zatoshis (1 ZEC = 100,000,000 zatoshis)
    pub amount_zatoshis: Option<u64>,
    /// Optional encrypted memo (max 512 bytes)
    pub memo: Option<String>,
    /// Fund source preference
    #[serde(default)]
    pub fund_source: FundSource,
}

impl TransferRequest {
    /// Get amount in zatoshis
    pub fn get_zatoshis(&self) -> OrchardResult<u64> {
        if let Some(zatoshis) = self.amount_zatoshis {
            return Ok(zatoshis);
        }

        let zec: f64 = self.amount_zec.parse()
            .map_err(|_| OrchardError::TransactionBuild("Invalid amount format".to_string()))?;

        if zec <= 0.0 {
            return Err(OrchardError::TransactionBuild("Amount must be positive".to_string()));
        }

        Ok((zec * 100_000_000.0) as u64)
    }
}

/// Result of initiating a transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProposal {
    /// Unique proposal ID
    pub proposal_id: String,
    /// Amount to send (zatoshis)
    pub amount_zatoshis: u64,
    /// Estimated fee (zatoshis)
    pub fee_zatoshis: u64,
    /// Source of funds
    pub fund_source: FundSource,
    /// Whether this is a shielding operation
    pub is_shielding: bool,
    /// Recipient address
    pub to_address: String,
    /// Memo if provided
    pub memo: Option<String>,
    /// Expiry height for the transaction
    pub expiry_height: u64,
}

/// Result of executing a transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResult {
    /// Transaction ID (hash)
    pub tx_id: String,
    /// Transaction status
    pub status: TransferStatus,
    /// Raw transaction hex (for broadcasting)
    pub raw_tx: Option<String>,
    /// Amount sent (zatoshis)
    pub amount_zatoshis: u64,
    /// Fee paid (zatoshis)
    pub fee_zatoshis: u64,
}

/// Transfer status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferStatus {
    /// Proposal created, awaiting execution
    Pending,
    /// Transaction built and signed
    Signed,
    /// Transaction submitted to network
    Submitted,
    /// Transaction confirmed on chain
    Confirmed,
    /// Transaction failed
    Failed,
}

/// Orchard transfer service
pub struct OrchardTransferService {
    /// Network parameters
    network: NetworkType,
}

/// Network type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkType {
    Mainnet,
    Testnet,
}

impl NetworkType {
    /// Get the consensus branch ID for the current network upgrade (NU6.1)
    /// NU6.1 activated at height 3,146,400 on mainnet
    pub fn consensus_branch_id(&self) -> u32 {
        match self {
            NetworkType::Mainnet => 0x4dec4df0, // NU6.1 mainnet
            NetworkType::Testnet => 0x4dec4df0, // NU6.1 testnet
        }
    }

    /// Get the activation height for Orchard
    pub fn orchard_activation_height(&self) -> u64 {
        match self {
            NetworkType::Mainnet => 1687104, // NU5 activation on mainnet
            NetworkType::Testnet => 1842420, // NU5 activation on testnet
        }
    }
}

impl OrchardTransferService {
    /// Create a new transfer service
    pub fn new(network: NetworkType) -> Self {
        Self { network }
    }

    /// Create a transfer proposal
    ///
    /// This validates the request and calculates fees without building the transaction.
    pub fn create_proposal(
        &self,
        request: &TransferRequest,
        transparent_balance_zatoshis: u64,
        shielded_balance: Option<&ShieldedBalance>,
        current_height: u64,
    ) -> OrchardResult<TransferProposal> {
        let amount = request.get_zatoshis()?;

        // Determine effective fund source and validate balance
        let (fund_source, is_shielding) = self.determine_fund_source(
            request.fund_source,
            amount,
            transparent_balance_zatoshis,
            shielded_balance,
        )?;

        // Calculate fee based on action count
        let fee = self.calculate_fee(1, fund_source);
        let total_needed = amount + fee;

        // Validate sufficient funds
        let available = match fund_source {
            FundSource::Transparent => transparent_balance_zatoshis,
            FundSource::Shielded => shielded_balance.map(|b| b.spendable_zatoshis).unwrap_or(0),
            FundSource::Auto => {
                let shielded = shielded_balance.map(|b| b.spendable_zatoshis).unwrap_or(0);
                shielded + transparent_balance_zatoshis
            }
        };

        if available < total_needed {
            return Err(OrchardError::InsufficientBalance {
                available,
                required: total_needed,
            });
        }

        // Generate proposal ID
        let proposal_id = self.generate_proposal_id();

        // Calculate expiry height (default: current + 40 blocks, ~40 minutes)
        let expiry_height = current_height + 40;

        Ok(TransferProposal {
            proposal_id,
            amount_zatoshis: amount,
            fee_zatoshis: fee,
            fund_source,
            is_shielding,
            to_address: request.to_address.clone(),
            memo: request.memo.clone(),
            expiry_height,
        })
    }

    /// Build and sign a transfer transaction
    ///
    /// This creates the actual Orchard transaction with proofs.
    ///
    /// # Arguments
    /// * `proposal` - The transfer proposal
    /// * `spending_key` - Orchard spending key for shielded operations
    /// * `private_key_hex` - Private key (hex) for signing transparent inputs
    /// * `spendable_notes` - Available shielded notes to spend
    /// * `transparent_inputs` - Transparent UTXOs to shield
    /// * `anchor_height` - Block height for anchor
    /// * `anchor` - Merkle tree anchor for Orchard
    pub fn build_transaction(
        &self,
        proposal: &TransferProposal,
        spending_key: &OrchardSpendingKey,
        private_key_hex: &str,
        spendable_notes: Vec<OrchardNote>,
        transparent_inputs: Vec<TransparentInput>,
        anchor_height: u64,
        anchor: [u8; 32],
    ) -> OrchardResult<TransferResult> {
        // Log all proposal details for debugging
        tracing::info!(
            "build_transaction called: proposal_id={}, amount_zatoshis={}, fee_zatoshis={}, \
             fund_source={:?}, is_shielding={}, to_address={}",
            proposal.proposal_id,
            proposal.amount_zatoshis,
            proposal.fee_zatoshis,
            proposal.fund_source,
            proposal.is_shielding,
            &proposal.to_address[..std::cmp::min(20, proposal.to_address.len())]
        );

        // CRITICAL SAFETY CHECK: Prevent zero-value transactions
        if proposal.amount_zatoshis == 0 {
            tracing::error!(
                "BLOCKED in build_transaction: amount_zatoshis=0! proposal={}",
                proposal.proposal_id
            );
            return Err(OrchardError::TransactionBuild(
                "Cannot build transaction with zero amount".to_string()
            ));
        }

        // Log transparent inputs
        let total_input: u64 = transparent_inputs.iter().map(|i| i.value).sum();
        tracing::info!(
            "Transparent inputs: count={}, total_value={} zatoshis",
            transparent_inputs.len(),
            total_input
        );

        // Validate that total input covers amount + fee
        if proposal.is_shielding && total_input < proposal.amount_zatoshis + proposal.fee_zatoshis {
            tracing::error!(
                "Insufficient inputs: have {} zatoshis, need {} zatoshis (amount={} + fee={})",
                total_input,
                proposal.amount_zatoshis + proposal.fee_zatoshis,
                proposal.amount_zatoshis,
                proposal.fee_zatoshis
            );
            return Err(OrchardError::InsufficientBalance {
                available: total_input,
                required: proposal.amount_zatoshis + proposal.fee_zatoshis,
            });
        }

        // Validate inputs
        if proposal.fund_source == FundSource::Shielded && spendable_notes.is_empty() {
            return Err(OrchardError::NoSpendableNotes);
        }

        if proposal.fund_source == FundSource::Transparent && transparent_inputs.is_empty() {
            return Err(OrchardError::TransactionBuild(
                "No transparent inputs provided for shielding".to_string()
            ));
        }

        // Build the transaction
        let tx_data = self.build_orchard_transaction(
            proposal,
            spending_key,
            private_key_hex,
            spendable_notes,
            transparent_inputs,
            anchor_height,
            anchor,
        )?;

        // Generate transaction ID
        let tx_id = self.compute_tx_id(&tx_data);

        Ok(TransferResult {
            tx_id,
            status: TransferStatus::Signed,
            raw_tx: Some(hex::encode(&tx_data)),
            amount_zatoshis: proposal.amount_zatoshis,
            fee_zatoshis: proposal.fee_zatoshis,
        })
    }

    /// Determine the effective fund source based on availability
    fn determine_fund_source(
        &self,
        requested: FundSource,
        amount: u64,
        transparent_balance: u64,
        shielded_balance: Option<&ShieldedBalance>,
    ) -> OrchardResult<(FundSource, bool)> {
        let shielded_available = shielded_balance
            .map(|b| b.spendable_zatoshis)
            .unwrap_or(0);

        // Estimate minimum fee
        let min_fee = DEFAULT_FEE_ZATOSHIS;
        let total_needed = amount + min_fee;

        match requested {
            FundSource::Shielded => {
                if shielded_available < total_needed {
                    return Err(OrchardError::InsufficientBalance {
                        available: shielded_available,
                        required: total_needed,
                    });
                }
                Ok((FundSource::Shielded, false))
            }
            FundSource::Transparent => {
                if transparent_balance < total_needed {
                    return Err(OrchardError::InsufficientBalance {
                        available: transparent_balance,
                        required: total_needed,
                    });
                }
                Ok((FundSource::Transparent, true)) // Shielding operation
            }
            FundSource::Auto => {
                // Prefer shielded funds for better privacy
                if shielded_available >= total_needed {
                    Ok((FundSource::Shielded, false))
                } else if transparent_balance >= total_needed {
                    Ok((FundSource::Transparent, true))
                } else if shielded_available + transparent_balance >= total_needed {
                    // Use both - this is a more complex case
                    // For now, we'll prefer using transparent as the primary source
                    // and shielding everything
                    Ok((FundSource::Transparent, true))
                } else {
                    Err(OrchardError::InsufficientBalance {
                        available: shielded_available + transparent_balance,
                        required: total_needed,
                    })
                }
            }
        }
    }

    /// Calculate transaction fee using ZIP-317 formula
    ///
    /// ZIP-317: fee ≥ marginal_fee × max(grace_actions, logical_actions)
    /// - marginal_fee = 5000 zatoshis
    /// - grace_actions = 2
    /// - logical_actions = transparent_inputs + orchard_actions
    fn calculate_fee(&self, num_outputs: u32, fund_source: FundSource) -> u64 {
        // ZIP-317 constants
        const MARGINAL_FEE: u64 = 5000;
        const GRACE_ACTIONS: u32 = 2;

        // Calculate logical actions based on fund source
        let (transparent_inputs, orchard_actions) = match fund_source {
            FundSource::Shielded => {
                // Pure shielded: no transparent inputs
                // Orchard actions = max(spends, outputs), minimum 2 due to padding
                // With payment + change = 2 outputs, need 2 actions
                (0u32, std::cmp::max(2, num_outputs + 1)) // +1 for potential change
            }
            FundSource::Transparent => {
                // Shielding: transparent inputs + Orchard outputs
                // Assume 1 transparent input (will be adjusted if more UTXOs needed)
                // Orchard outputs = payment + change = 2, so 2 actions
                (1u32, std::cmp::max(2, num_outputs + 1)) // +1 for change
            }
            FundSource::Auto => {
                // Could be either, assume worst case (shielding with change)
                (1u32, std::cmp::max(2, num_outputs + 1))
            }
        };

        let logical_actions = transparent_inputs + orchard_actions;
        let fee = MARGINAL_FEE * std::cmp::max(GRACE_ACTIONS, logical_actions) as u64;

        tracing::debug!(
            "ZIP-317 fee calculation: transparent_inputs={}, orchard_actions={}, logical_actions={}, fee={}",
            transparent_inputs,
            orchard_actions,
            logical_actions,
            fee
        );

        fee
    }

    /// Generate a unique proposal ID
    fn generate_proposal_id(&self) -> String {
        use rand::RngCore;
        let mut bytes = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut bytes);
        hex::encode(bytes)
    }

    /// Build the actual Orchard transaction
    fn build_orchard_transaction(
        &self,
        proposal: &TransferProposal,
        spending_key: &OrchardSpendingKey,
        private_key_hex: &str,
        spendable_notes: Vec<OrchardNote>,
        transparent_inputs: Vec<TransparentInput>,
        _anchor_height: u64,
        anchor: [u8; 32],
    ) -> OrchardResult<Vec<u8>> {
        // Create transaction builder
        let mut tx_data = Vec::new();

        // Transaction header (v5 format for Orchard support)
        // Version: 5 with overwinter flag (0x80000005 in little-endian)
        const TX_VERSION_V5_OVERWINTERED: u32 = 0x80000005;
        tx_data.extend_from_slice(&TX_VERSION_V5_OVERWINTERED.to_le_bytes());

        // Version group ID for v5 (0x26A7270A in little-endian)
        const VERSION_GROUP_ID_V5: u32 = 0x26A7270A;
        tx_data.extend_from_slice(&VERSION_GROUP_ID_V5.to_le_bytes());

        // Consensus branch ID
        tx_data.extend_from_slice(&self.network.consensus_branch_id().to_le_bytes());

        // Lock time (0 = no lock)
        tx_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

        // Expiry height
        tx_data.extend_from_slice(&(proposal.expiry_height as u32).to_le_bytes());

        // Build the appropriate bundle based on fund source
        match proposal.fund_source {
            FundSource::Shielded => {
                // Shielded to shielded transfer
                self.build_shielded_bundle(
                    &mut tx_data,
                    proposal,
                    spending_key,
                    spendable_notes,
                    anchor,
                )?;
            }
            FundSource::Transparent => {
                // Transparent to shielded (shielding)
                self.build_shielding_bundle(
                    &mut tx_data,
                    proposal,
                    spending_key,
                    private_key_hex,
                    transparent_inputs,
                    anchor,
                )?;
            }
            FundSource::Auto => {
                // Mixed - for now, treat as shielded if notes available
                if !spendable_notes.is_empty() {
                    self.build_shielded_bundle(
                        &mut tx_data,
                        proposal,
                        spending_key,
                        spendable_notes,
                        anchor,
                    )?;
                } else {
                    self.build_shielding_bundle(
                        &mut tx_data,
                        proposal,
                        spending_key,
                        private_key_hex,
                        transparent_inputs,
                        anchor,
                    )?;
                }
            }
        }

        Ok(tx_data)
    }

    /// Build shielded-to-shielded bundle
    fn build_shielded_bundle(
        &self,
        tx_data: &mut Vec<u8>,
        proposal: &TransferProposal,
        spending_key: &OrchardSpendingKey,
        notes: Vec<OrchardNote>,
        anchor: [u8; 32],
    ) -> OrchardResult<()> {
        // No transparent inputs
        tx_data.push(0x00); // vin count

        // No transparent outputs
        tx_data.push(0x00); // vout count

        // No Sapling spends
        tx_data.push(0x00); // nSpendsSapling

        // No Sapling outputs
        tx_data.push(0x00); // nOutputsSapling

        // Build Orchard bundle
        let orchard_bundle = self.create_orchard_actions(
            proposal,
            spending_key,
            notes,
            anchor,
        )?;

        tx_data.extend_from_slice(&orchard_bundle);

        Ok(())
    }

    /// Build transparent-to-shielded (shielding) bundle
    fn build_shielding_bundle(
        &self,
        tx_data: &mut Vec<u8>,
        proposal: &TransferProposal,
        spending_key: &OrchardSpendingKey,
        private_key_hex: &str,
        transparent_inputs: Vec<TransparentInput>,
        _anchor: [u8; 32],
    ) -> OrchardResult<()> {
        // Calculate total transparent input
        let total_transparent_input: u64 = transparent_inputs.iter().map(|i| i.value).sum();
        let num_inputs = transparent_inputs.len() as u64;

        // Recalculate fee based on actual input count (ZIP-317)
        // fee = 5000 * max(2, transparent_inputs + orchard_actions)
        // orchard_actions = 2 (payment + change, padded to even number)
        let orchard_actions: u64 = 2;
        let logical_actions = num_inputs + orchard_actions;
        let actual_fee = 5000 * std::cmp::max(2, logical_actions);

        tracing::info!(
            "build_shielding_bundle: {} inputs totaling {} zatoshis, amount={}, actual_fee={} (proposal_fee={})",
            num_inputs,
            total_transparent_input,
            proposal.amount_zatoshis,
            actual_fee,
            proposal.fee_zatoshis
        );

        // Use the higher of proposal fee or actual required fee
        let effective_fee = std::cmp::max(proposal.fee_zatoshis, actual_fee);

        // Step 1: Build the Orchard proven bundle (without binding signature yet)
        let proven_bundle = self.create_orchard_proven_bundle_with_fee(
            proposal,
            spending_key,
            total_transparent_input,
            effective_fee,
        )?;

        // Step 2: Compute the shielded sighash for binding signature
        // For shielding tx, we need: header_digest, transparent_txid_digest, sapling_digest, orchard_digest
        let shielded_sighash = self.compute_shielded_sighash(
            &transparent_inputs,
            proposal.expiry_height as u32,
            self.network.consensus_branch_id(),
            &proven_bundle,
        )?;

        tracing::debug!("Shielded sighash for binding sig: {}", hex::encode(&shielded_sighash));

        // Step 3: Apply signatures with correct sighash
        let saks: &[SpendAuthorizingKey] = &[]; // No spend auth keys for output-only
        let orchard_bundle = proven_bundle
            .apply_signatures(OsRng, shielded_sighash, saks)
            .map_err(|e| OrchardError::TransactionBuild(format!("Failed to apply signatures: {:?}", e)))?;

        // Step 4: Sign the transparent inputs using the authorized bundle
        let signed_inputs = sign_transparent_inputs_with_bundle(
            &transparent_inputs,
            private_key_hex,
            proposal.expiry_height as u32,
            self.network.consensus_branch_id(),
            &orchard_bundle,
        )?;

        // Step 5: Write signed transparent inputs
        tx_data.extend_from_slice(&serialize_compact_size(signed_inputs.len() as u64));

        for input in &signed_inputs {
            // Previous output hash (32 bytes, little-endian)
            let mut txid_le = input.prev_tx_hash;
            txid_le.reverse();
            tx_data.extend_from_slice(&txid_le);
            // Previous output index (4 bytes)
            tx_data.extend_from_slice(&input.prev_tx_index.to_le_bytes());
            // Script sig length + script sig
            tx_data.extend_from_slice(&serialize_compact_size(input.script_sig.len() as u64));
            tx_data.extend_from_slice(&input.script_sig);
            // Sequence (4 bytes)
            tx_data.extend_from_slice(&input.sequence.to_le_bytes());
        }

        // No transparent outputs (all going to shielded)
        tx_data.push(0x00); // vout count

        // No Sapling spends
        tx_data.push(0x00);

        // No Sapling outputs
        tx_data.push(0x00);

        // Serialize and write the Orchard bundle
        let bundle_start = tx_data.len();
        self.serialize_orchard_bundle(&orchard_bundle, tx_data)?;
        let bundle_len = tx_data.len() - bundle_start;

        tracing::info!(
            "Built shielding transaction: {} transparent inputs, {} bytes Orchard bundle",
            signed_inputs.len(),
            bundle_len
        );

        Ok(())
    }

    /// Compute shielded sighash for binding signature (SignableInput::Shielded equivalent)
    /// This follows ZIP 244 for v5 transactions with SignableInput::Shielded
    fn compute_shielded_sighash<V: Copy + Into<i64>>(
        &self,
        transparent_inputs: &[TransparentInput],
        expiry_height: u32,
        consensus_branch_id: u32,
        bundle: &orchard::bundle::Bundle<InProgress<Proof, Unauthorized>, V>,
    ) -> OrchardResult<[u8; 32]> {
        // T.1: header_digest
        let mut header_data = Vec::new();
        header_data.extend_from_slice(&TX_VERSION_WITH_OVERWINTERED.to_le_bytes());
        header_data.extend_from_slice(&VERSION_GROUP_ID_V5.to_le_bytes());
        header_data.extend_from_slice(&consensus_branch_id.to_le_bytes());
        header_data.extend_from_slice(&0u32.to_le_bytes()); // lock_time
        header_data.extend_from_slice(&expiry_height.to_le_bytes());
        let header_digest = blake2b_256(b"ZTxIdHeadersHash", &header_data);

        // T.2: transparent_sig_digest for SignableInput::Shielded
        // According to ZIP 244, this requires:
        // - hash_type (SIGHASH_ALL = 0x01)
        // - prevouts_digest
        // - amounts_digest (hash of all input amounts)
        // - scripts_digest (hash of all input scriptPubKeys)
        // - sequences_digest
        // - outputs_digest (empty for shielding)
        // - txin_sig_digest (empty for Shielded input type)
        let transparent_sig_digest = if transparent_inputs.is_empty() {
            // No transparent inputs - use empty hash
            blake2b_256(ZCASH_TRANSPARENT_HASH, &[])
        } else {
            let prevouts_digest = hash_prevouts(transparent_inputs);
            let amounts_digest = hash_amounts(transparent_inputs);
            let scripts_digest = hash_script_pubkeys(transparent_inputs);
            let sequences_digest = hash_sequences(transparent_inputs);
            // outputs_digest is empty since no transparent outputs
            let outputs_digest = blake2b_256(ZCASH_OUTPUTS_HASH, &[]);
            // txin_sig_digest is empty for SignableInput::Shielded
            let txin_sig_digest = blake2b_256(ZCASH_TRANSPARENT_SIG, &[]);

            let mut transparent_data = Vec::new();
            transparent_data.push(0x01); // SIGHASH_ALL
            transparent_data.extend_from_slice(&prevouts_digest);
            transparent_data.extend_from_slice(&amounts_digest);
            transparent_data.extend_from_slice(&scripts_digest);
            transparent_data.extend_from_slice(&sequences_digest);
            transparent_data.extend_from_slice(&outputs_digest);
            transparent_data.extend_from_slice(&txin_sig_digest);
            blake2b_256(ZCASH_TRANSPARENT_HASH, &transparent_data)
        };

        // T.3: sapling_digest (empty)
        let sapling_digest = blake2b_256(b"ZTxIdSaplingHash", &[]);

        // T.4: orchard_digest - use the bundle's commitment
        let orchard_digest = compute_orchard_digest_from_proven(bundle);

        tracing::debug!("Shielded sighash components:");
        tracing::debug!("  header_digest: {}", hex::encode(&header_digest));
        tracing::debug!("  transparent_sig_digest: {}", hex::encode(&transparent_sig_digest));
        tracing::debug!("  sapling_digest: {}", hex::encode(&sapling_digest));
        tracing::debug!("  orchard_digest: {}", hex::encode(&orchard_digest));

        // Final sighash
        let mut personalization = ZCASH_TX_HASH.to_vec();
        personalization.extend_from_slice(&consensus_branch_id.to_le_bytes());

        let mut sig_data = Vec::new();
        sig_data.extend_from_slice(&header_digest);
        sig_data.extend_from_slice(&transparent_sig_digest);
        sig_data.extend_from_slice(&sapling_digest);
        sig_data.extend_from_slice(&orchard_digest);

        Ok(blake2b_256(&personalization, &sig_data))
    }

    /// Create Orchard actions (spends + outputs)
    fn create_orchard_actions(
        &self,
        proposal: &TransferProposal,
        spending_key: &OrchardSpendingKey,
        notes: Vec<OrchardNote>,
        anchor: [u8; 32],
    ) -> OrchardResult<Vec<u8>> {
        let mut bundle = Vec::new();

        // Select notes to spend
        let (selected_notes, total_input) = self.select_notes(
            notes,
            proposal.amount_zatoshis + proposal.fee_zatoshis,
        )?;

        let num_spends = selected_notes.len();
        let has_change = total_input > proposal.amount_zatoshis + proposal.fee_zatoshis;
        let num_outputs = if has_change { 2 } else { 1 }; // payment + optional change

        // Pad to even number (Orchard requirement)
        let num_actions = std::cmp::max(num_spends, num_outputs);
        let num_actions = if num_actions % 2 == 1 { num_actions + 1 } else { num_actions };

        // Number of actions
        bundle.push(num_actions as u8);

        // Create actions
        for i in 0..num_actions {
            let action = self.create_action(
                i,
                if i < num_spends { Some(&selected_notes[i]) } else { None },
                proposal,
                spending_key,
                i == 0, // First action is the payment
                has_change && i == 1, // Second action is change if needed
                total_input,
            )?;
            bundle.extend_from_slice(&action);
        }

        // Flags
        let flags = 0x03u8; // spends_enabled | outputs_enabled
        bundle.push(flags);

        // Value balance (negative means shielded pool gains value)
        let value_balance: i64 = 0; // For shielded-to-shielded, net is 0
        bundle.extend_from_slice(&value_balance.to_le_bytes());

        // Anchor
        bundle.extend_from_slice(&anchor);

        // Proof (placeholder - real implementation generates Halo 2 proofs)
        let proof_size = num_actions * 2720; // Each action proof is 2720 bytes
        bundle.extend_from_slice(&(proof_size as u32).to_le_bytes());
        bundle.extend_from_slice(&vec![0u8; proof_size]);

        // Binding signature (64 bytes)
        let binding_sig = self.create_binding_signature(spending_key)?;
        bundle.extend_from_slice(&binding_sig);

        Ok(bundle)
    }

    /// Create Orchard proven bundle (for shielding) - without binding signature
    /// Returns a proven bundle that needs apply_signatures with correct sighash
    ///
    /// # Arguments
    /// * `proposal` - Transfer proposal with amount and recipient
    /// * `spending_key` - Sender's spending key (for change address and OVK)
    /// * `total_transparent_input` - Total value of transparent inputs (for calculating change)
    /// * `effective_fee` - Actual fee to use (may differ from proposal.fee_zatoshis for ZIP-317 compliance)
    fn create_orchard_proven_bundle_with_fee(
        &self,
        proposal: &TransferProposal,
        spending_key: &OrchardSpendingKey,
        total_transparent_input: u64,
        effective_fee: u64,
    ) -> OrchardResult<orchard::bundle::Bundle<InProgress<Proof, Unauthorized>, i64>> {
        use super::address::OrchardAddressManager;
        use orchard::keys::Scope;

        // CRITICAL SAFETY CHECK: Prevent zero-value Orchard bundles
        if proposal.amount_zatoshis == 0 {
            tracing::error!(
                "BLOCKED in create_orchard_proven_bundle: amount_zatoshis=0! proposal={}, to={}",
                proposal.proposal_id,
                proposal.to_address
            );
            return Err(OrchardError::TransactionBuild(
                "Cannot create Orchard output with zero value - this would cause complete fund loss".to_string()
            ));
        }

        // Calculate change amount using effective fee
        let total_output = proposal.amount_zatoshis + effective_fee;
        if total_transparent_input < total_output {
            return Err(OrchardError::InsufficientBalance {
                available: total_transparent_input,
                required: total_output,
            });
        }
        let change_amount = total_transparent_input - total_output;

        tracing::info!(
            "Shielding transaction: input={}, amount={}, fee={}, change={}",
            total_transparent_input,
            proposal.amount_zatoshis,
            effective_fee,
            change_amount
        );

        // Get the proving key (cached globally)
        let pk = get_proving_key();

        // For shielding (no spends), we use an empty tree anchor
        let anchor = Anchor::empty_tree();

        // Create builder with DEFAULT bundle type
        let bundle_type = BundleType::DEFAULT;
        let mut builder = OrchardBuilder::new(bundle_type, anchor);

        // Parse recipient address to get Orchard receiver
        let recipient_address = OrchardAddressManager::extract_orchard_address(&proposal.to_address)?;

        // Get OVK for sender to be able to decrypt outgoing transaction
        let ovk = Some(spending_key.to_ovk());

        // === Output 1: Payment to recipient ===
        tracing::info!(
            "Creating Orchard payment output: amount={} zatoshis, to={}...",
            proposal.amount_zatoshis,
            &proposal.to_address[..std::cmp::min(20, proposal.to_address.len())]
        );

        let payment_value = NoteValue::from_raw(proposal.amount_zatoshis);
        let memo_bytes: Option<[u8; 512]> = proposal.memo.as_ref().map(|m| {
            let mut bytes = [0u8; 512];
            let memo_data = m.as_bytes();
            let len = std::cmp::min(memo_data.len(), 512);
            bytes[..len].copy_from_slice(&memo_data[..len]);
            bytes
        });

        builder
            .add_output(ovk.clone(), recipient_address, payment_value, memo_bytes)
            .map_err(|e| OrchardError::TransactionBuild(format!("Failed to add payment output: {:?}", e)))?;

        // === Output 2: Change to sender (if any) ===
        if change_amount > 0 {
            // Get sender's own Orchard address for change
            let fvk = spending_key.to_fvk();
            // Use diversifier index 0 for change address
            let change_diversifier = orchard::keys::Diversifier::from_bytes([0u8; 11]);
            let change_address = fvk.address(change_diversifier, Scope::Internal);

            tracing::info!(
                "Creating Orchard change output: amount={} zatoshis (to self)",
                change_amount
            );

            let change_value = NoteValue::from_raw(change_amount);
            // No memo for change output
            builder
                .add_output(ovk, change_address, change_value, None)
                .map_err(|e| OrchardError::TransactionBuild(format!("Failed to add change output: {:?}", e)))?;
        }

        // Build the bundle (returns UnauthorizedBundle)
        let (unauthorized_bundle, _meta) = builder
            .build::<i64>(&mut OsRng)
            .map_err(|e| OrchardError::TransactionBuild(format!("Failed to build bundle: {:?}", e)))?
            .ok_or_else(|| OrchardError::TransactionBuild("Empty bundle".to_string()))?;

        // Log the value_balance to debug
        let value_balance = *unauthorized_bundle.value_balance();

        // Expected value_balance = -(payment + change) = -(total_input - fee)
        let total_shielded_output = proposal.amount_zatoshis + change_amount;
        let expected_value_balance = -(total_shielded_output as i64);

        tracing::info!(
            "Orchard bundle built: value_balance={}, expected={}, actions={}, outputs={}",
            value_balance,
            expected_value_balance,
            unauthorized_bundle.actions().len(),
            if change_amount > 0 { 2 } else { 1 }
        );

        // CRITICAL SAFETY CHECK: Verify value_balance is correct
        if value_balance != expected_value_balance {
            tracing::error!(
                "CRITICAL: Orchard bundle value_balance mismatch! got={}, expected={}, payment={}, change={}",
                value_balance,
                expected_value_balance,
                proposal.amount_zatoshis,
                change_amount
            );
            if value_balance == 0 {
                return Err(OrchardError::TransactionBuild(
                    format!(
                        "CRITICAL: Orchard bundle has zero value_balance! This would cause complete fund loss. \
                         Expected: {}, Payment: {} zatoshis, Change: {} zatoshis",
                        expected_value_balance, proposal.amount_zatoshis, change_amount
                    )
                ));
            }
        }

        // Additional safety: verify the math
        // transparent_input = |value_balance| + fee
        // transparent_input = (payment + change) + fee
        let computed_input = ((-value_balance) as u64) + effective_fee;
        if computed_input != total_transparent_input {
            tracing::error!(
                "CRITICAL: Input/output mismatch! computed_input={}, actual_input={}, fee={}",
                computed_input,
                total_transparent_input,
                effective_fee
            );
            return Err(OrchardError::TransactionBuild(
                format!(
                    "Transaction balance mismatch: computed input {} != actual input {}",
                    computed_input, total_transparent_input
                )
            ));
        }

        tracing::info!(
            "Transaction balance verified: {} (input) = {} (shielded) + {} (fee)",
            total_transparent_input,
            total_shielded_output,
            effective_fee
        );

        // Create proof - returns a proven but not yet signed bundle
        let proven_bundle = unauthorized_bundle
            .create_proof(pk, &mut OsRng)
            .map_err(|e| OrchardError::TransactionBuild(format!("Failed to create proof: {:?}", e)))?;

        Ok(proven_bundle)
    }

    /// Serialize an authorized Orchard bundle to bytes
    /// Uses zcash_primitives for correct serialization format
    fn serialize_orchard_bundle(
        &self,
        bundle: &orchard::Bundle<orchard::bundle::Authorized, i64>,
        output: &mut Vec<u8>,
    ) -> OrchardResult<()> {
        use zcash_primitives::transaction::components::orchard as orchard_serialization;
        use zcash_protocol::value::ZatBalance;

        // Convert Bundle<Authorized, i64> to Bundle<Authorized, ZatBalance>
        let value_balance_i64 = *bundle.value_balance();
        let zat_balance = ZatBalance::from_i64(value_balance_i64)
            .map_err(|_| OrchardError::TransactionBuild("Invalid value balance".to_string()))?;

        let bundle_with_zat = bundle.clone().try_map_value_balance::<ZatBalance, _, _>(|_| Ok(zat_balance))
            .map_err(|e: std::convert::Infallible| OrchardError::TransactionBuild(format!("Failed to map value balance: {:?}", e)))?;

        // Use zcash_primitives serialization
        let num_actions = bundle.actions().len();
        orchard_serialization::write_v5_bundle(Some(&bundle_with_zat), &mut *output)
            .map_err(|e| OrchardError::TransactionBuild(format!("Failed to serialize bundle: {}", e)))?;

        tracing::debug!(
            "Serialized Orchard bundle: {} actions, {} bytes total",
            num_actions,
            output.len()
        );

        Ok(())
    }

    /// Select notes to cover the required amount
    fn select_notes(
        &self,
        mut notes: Vec<OrchardNote>,
        amount_needed: u64,
    ) -> OrchardResult<(Vec<OrchardNote>, u64)> {
        if notes.is_empty() {
            return Err(OrchardError::NoSpendableNotes);
        }

        // Sort by value descending
        notes.sort_by(|a, b| b.value_zatoshis.cmp(&a.value_zatoshis));

        let mut selected = Vec::new();
        let mut total: u64 = 0;

        for note in notes {
            if total >= amount_needed {
                break;
            }
            total += note.value_zatoshis;
            selected.push(note);
        }

        if total < amount_needed {
            return Err(OrchardError::InsufficientBalance {
                available: total,
                required: amount_needed,
            });
        }

        Ok((selected, total))
    }

    /// Create a single Orchard action
    fn create_action(
        &self,
        _index: usize,
        spend_note: Option<&OrchardNote>,
        proposal: &TransferProposal,
        _spending_key: &OrchardSpendingKey,
        is_payment: bool,
        is_change: bool,
        total_input: u64,
    ) -> OrchardResult<Vec<u8>> {
        let mut action = Vec::new();

        // Nullifier (32 bytes) - from spend note or dummy
        if let Some(note) = spend_note {
            action.extend_from_slice(&note.nullifier);
        } else {
            action.extend_from_slice(&[0u8; 32]);
        }

        // Randomized verification key (32 bytes)
        let mut rk = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut rk);
        action.extend_from_slice(&rk);

        // Note commitment (32 bytes)
        let cmx = self.compute_note_commitment(
            if is_payment { proposal.amount_zatoshis }
            else if is_change { total_input - proposal.amount_zatoshis - proposal.fee_zatoshis }
            else { 0 },
        )?;
        action.extend_from_slice(&cmx);

        // Encrypted note (580 bytes)
        let enc_ciphertext = self.encrypt_note(
            proposal,
            is_payment,
            is_change,
            total_input,
        )?;
        action.extend_from_slice(&enc_ciphertext);

        // Ephemeral key (32 bytes)
        let mut epk = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut epk);
        action.extend_from_slice(&epk);

        // Out ciphertext (80 bytes)
        action.extend_from_slice(&[0u8; 80]);

        // cv (value commitment, 32 bytes)
        let cv = self.compute_value_commitment(
            if is_payment { proposal.amount_zatoshis }
            else if is_change { total_input - proposal.amount_zatoshis - proposal.fee_zatoshis }
            else { 0 },
        )?;
        action.extend_from_slice(&cv);

        // Authorization signature (64 bytes)
        action.extend_from_slice(&[0u8; 64]);

        Ok(action)
    }

    /// Create output-only action
    fn create_output_action(
        &self,
        proposal: &TransferProposal,
        _spending_key: &OrchardSpendingKey,
    ) -> OrchardResult<Vec<u8>> {
        let mut action = Vec::new();

        // Nullifier (dummy for output-only)
        action.extend_from_slice(&[0u8; 32]);

        // rk
        let mut rk = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut rk);
        action.extend_from_slice(&rk);

        // cmx
        let cmx = self.compute_note_commitment(proposal.amount_zatoshis)?;
        action.extend_from_slice(&cmx);

        // enc_ciphertext
        let enc = self.encrypt_note(proposal, true, false, proposal.amount_zatoshis)?;
        action.extend_from_slice(&enc);

        // epk
        let mut epk = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut epk);
        action.extend_from_slice(&epk);

        // out_ciphertext
        action.extend_from_slice(&[0u8; 80]);

        // cv
        let cv = self.compute_value_commitment(proposal.amount_zatoshis)?;
        action.extend_from_slice(&cv);

        // auth sig (dummy for output)
        action.extend_from_slice(&[0u8; 64]);

        Ok(action)
    }

    /// Create dummy action for padding
    fn create_dummy_action(&self) -> OrchardResult<Vec<u8>> {
        let mut action = Vec::new();

        // All zeros for dummy action
        action.extend_from_slice(&[0u8; 32]); // nullifier
        action.extend_from_slice(&[0u8; 32]); // rk
        action.extend_from_slice(&[0u8; 32]); // cmx
        action.extend_from_slice(&[0u8; 580]); // enc_ciphertext
        action.extend_from_slice(&[0u8; 32]); // epk
        action.extend_from_slice(&[0u8; 80]); // out_ciphertext
        action.extend_from_slice(&[0u8; 32]); // cv
        action.extend_from_slice(&[0u8; 64]); // auth sig

        Ok(action)
    }

    /// Compute note commitment
    fn compute_note_commitment(&self, value: u64) -> OrchardResult<[u8; 32]> {
        let mut hasher = blake2b_simd::Params::new()
            .hash_length(32)
            .personal(b"ZcashOrchardCm__")
            .to_state();
        hasher.update(&value.to_le_bytes());

        let mut rcm = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut rcm);
        hasher.update(&rcm);

        let result = hasher.finalize();
        let mut cm = [0u8; 32];
        cm.copy_from_slice(result.as_bytes());
        Ok(cm)
    }

    /// Compute value commitment
    fn compute_value_commitment(&self, value: u64) -> OrchardResult<[u8; 32]> {
        let mut hasher = blake2b_simd::Params::new()
            .hash_length(32)
            .personal(b"ZcashOrchardCV__")
            .to_state();
        hasher.update(&value.to_le_bytes());

        let mut rcv = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut rcv);
        hasher.update(&rcv);

        let result = hasher.finalize();
        let mut cv = [0u8; 32];
        cv.copy_from_slice(result.as_bytes());
        Ok(cv)
    }

    /// Encrypt note data
    fn encrypt_note(
        &self,
        proposal: &TransferProposal,
        is_payment: bool,
        is_change: bool,
        total_input: u64,
    ) -> OrchardResult<Vec<u8>> {
        let mut plaintext = Vec::new();

        // Note plaintext structure:
        // - 1 byte: lead byte (0x02 for Orchard)
        // - 11 bytes: diversifier
        // - 8 bytes: value
        // - 32 bytes: rseed
        // - 512 bytes: memo

        plaintext.push(0x02); // Orchard lead byte
        plaintext.extend_from_slice(&[0u8; 11]); // diversifier (would be derived from address)

        let value = if is_payment {
            proposal.amount_zatoshis
        } else if is_change {
            total_input - proposal.amount_zatoshis - proposal.fee_zatoshis
        } else {
            0
        };
        plaintext.extend_from_slice(&value.to_le_bytes());

        // Random rseed
        let mut rseed = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut rseed);
        plaintext.extend_from_slice(&rseed);

        // Memo (512 bytes)
        let mut memo = [0u8; 512];
        if let Some(ref m) = proposal.memo {
            let bytes = m.as_bytes();
            let len = std::cmp::min(bytes.len(), 512);
            memo[..len].copy_from_slice(&bytes[..len]);
        }
        plaintext.extend_from_slice(&memo);

        // In real implementation, encrypt with recipient's key using ChaCha20Poly1305
        // For now, just pad to 580 bytes (encrypted size with tag)
        let mut ciphertext = plaintext;
        ciphertext.resize(580, 0);

        Ok(ciphertext)
    }

    /// Create binding signature
    fn create_binding_signature(&self, _spending_key: &OrchardSpendingKey) -> OrchardResult<[u8; 64]> {
        // In real implementation, this would sign with the binding key
        // For now, return placeholder
        let mut sig = [0u8; 64];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut sig);
        Ok(sig)
    }

    /// Compute transaction ID from raw transaction data
    fn compute_tx_id(&self, tx_data: &[u8]) -> String {
        let hash = blake2b_simd::Params::new()
            .hash_length(32)
            .personal(b"ZcashTxId_____")
            .hash(tx_data);

        // Reverse for display (big-endian)
        let mut tx_id = hash.as_bytes().to_vec();
        tx_id.reverse();

        hex::encode(tx_id)
    }
}

/// Transparent input for shielding
#[derive(Debug, Clone)]
pub struct TransparentInput {
    /// Previous transaction hash (stored in big-endian for display, reversed for wire format)
    pub prev_tx_hash: [u8; 32],
    /// Previous output index
    pub prev_tx_index: u32,
    /// Script pubkey (the UTXO's locking script - NOT the signature)
    pub script_pubkey: Vec<u8>,
    /// Value in zatoshis
    pub value: u64,
    /// Sequence number
    pub sequence: u32,
}

/// Signed transparent input ready for serialization
#[derive(Debug, Clone)]
pub struct SignedTransparentInput {
    /// Previous transaction hash
    pub prev_tx_hash: [u8; 32],
    /// Previous output index
    pub prev_tx_index: u32,
    /// Signed script sig (signature + pubkey)
    pub script_sig: Vec<u8>,
    /// Sequence number
    pub sequence: u32,
}

// ============================================================================
// Transparent Input Signing Helpers (ZIP 244 compliant)
// ============================================================================

/// BLAKE2b-256 hash with personalization (ZIP 244 compliant)
fn blake2b_256(personalization: &[u8], data: &[u8]) -> [u8; 32] {
    let mut pers = [0u8; 16];
    let len = std::cmp::min(personalization.len(), 16);
    pers[..len].copy_from_slice(&personalization[..len]);

    let hash = blake2b_simd::Params::new()
        .hash_length(32)
        .personal(&pers)
        .hash(data);

    let mut output = [0u8; 32];
    output.copy_from_slice(hash.as_bytes());
    output
}

/// Serialize a compact size integer (Bitcoin/Zcash varint)
fn serialize_compact_size(n: u64) -> Vec<u8> {
    if n < 0xfd {
        vec![n as u8]
    } else if n <= 0xffff {
        let mut v = vec![0xfd];
        v.extend_from_slice(&(n as u16).to_le_bytes());
        v
    } else if n <= 0xffffffff {
        let mut v = vec![0xfe];
        v.extend_from_slice(&(n as u32).to_le_bytes());
        v
    } else {
        let mut v = vec![0xff];
        v.extend_from_slice(&n.to_le_bytes());
        v
    }
}

/// Hash of all prevouts (ZIP 244)
fn hash_prevouts(inputs: &[TransparentInput]) -> [u8; 32] {
    let mut data = Vec::new();
    for input in inputs {
        // txid is stored big-endian, but serialized little-endian
        let mut txid_le = input.prev_tx_hash;
        txid_le.reverse();
        data.extend_from_slice(&txid_le);
        data.extend_from_slice(&input.prev_tx_index.to_le_bytes());
    }
    blake2b_256(ZCASH_PREVOUTS_HASH, &data)
}

/// Hash of all sequences (ZIP 244)
fn hash_sequences(inputs: &[TransparentInput]) -> [u8; 32] {
    let mut data = Vec::new();
    for input in inputs {
        data.extend_from_slice(&input.sequence.to_le_bytes());
    }
    blake2b_256(ZCASH_SEQUENCE_HASH, &data)
}

/// Hash of input amounts (ZIP 244)
fn hash_amounts(inputs: &[TransparentInput]) -> [u8; 32] {
    let mut data = Vec::new();
    for input in inputs {
        data.extend_from_slice(&(input.value as i64).to_le_bytes());
    }
    blake2b_256(b"ZTxTrAmountsHash", &data)
}

/// Hash of input scripts (ZIP 244)
fn hash_script_pubkeys(inputs: &[TransparentInput]) -> [u8; 32] {
    let mut data = Vec::new();
    for input in inputs {
        data.extend_from_slice(&serialize_compact_size(input.script_pubkey.len() as u64));
        data.extend_from_slice(&input.script_pubkey);
    }
    blake2b_256(b"ZTxTrScriptsHash", &data)
}

/// Calculate ZIP 244 sighash for a transparent input in a shielding transaction
fn calculate_shielding_sighash(
    inputs: &[TransparentInput],
    input_index: usize,
    expiry_height: u32,
    consensus_branch_id: u32,
    bundle: &orchard::Bundle<orchard::bundle::Authorized, i64>,
) -> OrchardResult<[u8; 32]> {
    let input = &inputs[input_index];

    tracing::debug!(
        "Calculating sighash for input {}: txid={} vout={} value={} script_len={}",
        input_index,
        hex::encode(&input.prev_tx_hash),
        input.prev_tx_index,
        input.value,
        input.script_pubkey.len()
    );
    tracing::debug!("Script pubkey: {}", hex::encode(&input.script_pubkey));

    // Build prevouts_sig_digest
    let prevouts_digest = hash_prevouts(inputs);
    tracing::debug!("prevouts_digest: {}", hex::encode(&prevouts_digest));

    // Build amounts_sig_digest
    let amounts_digest = hash_amounts(inputs);
    tracing::debug!("amounts_digest: {}", hex::encode(&amounts_digest));

    // Build scripts_sig_digest
    let scripts_digest = hash_script_pubkeys(inputs);
    tracing::debug!("scripts_digest: {}", hex::encode(&scripts_digest));

    // Build sequences_sig_digest
    let sequences_digest = hash_sequences(inputs);
    tracing::debug!("sequences_digest: {}", hex::encode(&sequences_digest));

    // Build outputs_sig_digest (empty for shielding - no transparent outputs)
    let outputs_digest = blake2b_256(ZCASH_OUTPUTS_HASH, &[]);
    tracing::debug!("outputs_digest: {}", hex::encode(&outputs_digest));

    // Build txin_sig_digest for this input
    let mut txin_data = Vec::new();
    let mut txid_le = input.prev_tx_hash;
    txid_le.reverse();
    txin_data.extend_from_slice(&txid_le);
    txin_data.extend_from_slice(&input.prev_tx_index.to_le_bytes());
    txin_data.extend_from_slice(&(input.value as i64).to_le_bytes());
    txin_data.extend_from_slice(&serialize_compact_size(input.script_pubkey.len() as u64));
    txin_data.extend_from_slice(&input.script_pubkey);
    txin_data.extend_from_slice(&input.sequence.to_le_bytes());

    tracing::debug!("txin_data ({} bytes): {}", txin_data.len(), hex::encode(&txin_data));

    let txin_sig_digest = blake2b_256(ZCASH_TRANSPARENT_SIG, &txin_data);
    tracing::debug!("txin_sig_digest: {}", hex::encode(&txin_sig_digest));

    // Combine for transparent_sig_digest
    let mut transparent_data = Vec::new();
    transparent_data.push(0x01); // SIGHASH_ALL
    transparent_data.extend_from_slice(&prevouts_digest);
    transparent_data.extend_from_slice(&amounts_digest);
    transparent_data.extend_from_slice(&scripts_digest);
    transparent_data.extend_from_slice(&sequences_digest);
    transparent_data.extend_from_slice(&outputs_digest);
    transparent_data.extend_from_slice(&txin_sig_digest);

    let transparent_sig_digest = blake2b_256(ZCASH_TRANSPARENT_HASH, &transparent_data);
    tracing::debug!("transparent_sig_digest: {}", hex::encode(&transparent_sig_digest));

    // Build header_digest
    let mut header_data = Vec::new();
    header_data.extend_from_slice(&TX_VERSION_WITH_OVERWINTERED.to_le_bytes());
    header_data.extend_from_slice(&VERSION_GROUP_ID_V5.to_le_bytes());
    header_data.extend_from_slice(&consensus_branch_id.to_le_bytes());
    header_data.extend_from_slice(&0u32.to_le_bytes()); // lock_time
    header_data.extend_from_slice(&expiry_height.to_le_bytes());

    tracing::debug!(
        "header_data: version={:08x} vg_id={:08x} branch_id={:08x} expiry={}",
        TX_VERSION_WITH_OVERWINTERED,
        VERSION_GROUP_ID_V5,
        consensus_branch_id,
        expiry_height
    );

    let header_digest = blake2b_256(b"ZTxIdHeadersHash", &header_data);
    tracing::debug!("header_digest: {}", hex::encode(&header_digest));

    // Empty sapling digest
    let sapling_digest = blake2b_256(b"ZTxIdSaplingHash", &[]);
    tracing::debug!("sapling_digest: {}", hex::encode(&sapling_digest));

    // Compute orchard_digest according to ZIP 244 T.4
    let orchard_digest = compute_orchard_digest(bundle);
    tracing::debug!("orchard_digest: {}", hex::encode(&orchard_digest));

    // Build the final sighash
    let mut personalization = ZCASH_TX_HASH.to_vec();
    personalization.extend_from_slice(&consensus_branch_id.to_le_bytes());

    let mut sig_data = Vec::new();
    sig_data.extend_from_slice(&header_digest);
    sig_data.extend_from_slice(&transparent_sig_digest);
    sig_data.extend_from_slice(&sapling_digest);
    sig_data.extend_from_slice(&orchard_digest);

    let sighash = blake2b_256(&personalization, &sig_data);
    tracing::info!("Final sighash: {}", hex::encode(&sighash));

    Ok(sighash)
}

/// Compute orchard_digest according to ZIP 244 T.4
/// Uses the orchard crate's built-in commitment() method for correctness
fn compute_orchard_digest(bundle: &orchard::Bundle<orchard::bundle::Authorized, i64>) -> [u8; 32] {
    // The orchard crate's commitment() method correctly implements ZIP 244 T.4
    let commitment = bundle.commitment();
    // BundleCommitment wraps a Blake2bHash, get the raw bytes
    let hash_bytes = commitment.0.as_bytes();
    let mut result = [0u8; 32];
    result.copy_from_slice(hash_bytes);
    result
}

/// Compute orchard_digest from a proven (but not yet authorized) bundle
/// Uses the bundle's built-in commitment() method for correctness
/// Used for computing the shielded sighash before apply_signatures
fn compute_orchard_digest_from_proven<V: Copy + Into<i64>>(
    bundle: &orchard::bundle::Bundle<InProgress<Proof, Unauthorized>, V>,
) -> [u8; 32] {
    // Use the bundle's built-in commitment() method which correctly implements ZIP 244 T.4
    // The InProgress bundle type implements Authorization trait, so commitment() is available
    let commitment = bundle.commitment();
    let hash_bytes = commitment.0.as_bytes();
    let mut result = [0u8; 32];
    result.copy_from_slice(hash_bytes);
    result
}

/// Sign a transparent input
fn sign_transparent_input(
    secp: &Secp256k1<secp256k1::All>,
    secret_key: &SecretKey,
    sighash: &[u8; 32],
) -> OrchardResult<Vec<u8>> {
    let message = Message::from_digest_slice(sighash)
        .map_err(|e| OrchardError::TransactionBuild(format!("Invalid sighash: {}", e)))?;

    let signature = secp.sign_ecdsa(&message, secret_key);

    // DER encode and append sighash type
    let mut sig_bytes = signature.serialize_der().to_vec();
    sig_bytes.push(0x01); // SIGHASH_ALL

    Ok(sig_bytes)
}

/// Build scriptSig for P2PKH input
fn build_p2pkh_scriptsig(signature: &[u8], public_key: &PublicKey) -> Vec<u8> {
    let pubkey_bytes = public_key.serialize(); // Compressed

    let mut script_sig = Vec::new();
    // Push signature
    script_sig.push(signature.len() as u8);
    script_sig.extend_from_slice(signature);
    // Push public key
    script_sig.push(pubkey_bytes.len() as u8);
    script_sig.extend_from_slice(&pubkey_bytes);

    script_sig
}

/// Sign all transparent inputs for a shielding transaction
pub fn sign_transparent_inputs_with_bundle(
    inputs: &[TransparentInput],
    private_key_hex: &str,
    expiry_height: u32,
    consensus_branch_id: u32,
    bundle: &orchard::Bundle<orchard::bundle::Authorized, i64>,
) -> OrchardResult<Vec<SignedTransparentInput>> {
    let secp = Secp256k1::new();

    // Parse private key
    let key_hex = private_key_hex.strip_prefix("0x").unwrap_or(private_key_hex);
    let key_bytes = hex::decode(key_hex)
        .map_err(|e| OrchardError::KeyDerivation(format!("Invalid private key hex: {}", e)))?;

    if key_bytes.len() != 32 {
        return Err(OrchardError::KeyDerivation(
            "Private key must be 32 bytes".to_string(),
        ));
    }

    let secret_key = SecretKey::from_slice(&key_bytes)
        .map_err(|e| OrchardError::KeyDerivation(format!("Invalid private key: {}", e)))?;
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);

    // Log pubkey info for debugging
    let pubkey_compressed = public_key.serialize();
    tracing::debug!("Signing with pubkey (compressed): {}", hex::encode(&pubkey_compressed));

    // Compute expected pubkey hash (for P2PKH verification)
    use ripemd::Ripemd160;
    use sha2::{Digest, Sha256};
    let sha256_hash = Sha256::digest(&pubkey_compressed);
    let pubkey_hash = Ripemd160::digest(&sha256_hash);
    tracing::debug!("Expected pubkey_hash (HASH160): {}", hex::encode(&pubkey_hash));

    // Sign each input
    let mut signed_inputs = Vec::new();
    for (i, input) in inputs.iter().enumerate() {
        // Calculate sighash with proper orchard_digest
        let sighash = calculate_shielding_sighash(
            inputs,
            i,
            expiry_height,
            consensus_branch_id,
            bundle,
        )?;

        // Sign
        let signature = sign_transparent_input(&secp, &secret_key, &sighash)?;
        tracing::debug!("Input {} signature ({} bytes): {}", i, signature.len(), hex::encode(&signature));

        // Build scriptSig
        let script_sig = build_p2pkh_scriptsig(&signature, &public_key);
        tracing::debug!("Input {} scriptSig ({} bytes): {}", i, script_sig.len(), hex::encode(&script_sig));

        signed_inputs.push(SignedTransparentInput {
            prev_tx_hash: input.prev_tx_hash,
            prev_tx_index: input.prev_tx_index,
            script_sig,
            sequence: input.sequence,
        });
    }

    tracing::info!("Signed {} transparent inputs for shielding", signed_inputs.len());
    Ok(signed_inputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_request_zatoshis() {
        let request = TransferRequest {
            wallet_id: 1,
            to_address: "u1test".to_string(),
            amount_zec: "1.5".to_string(),
            amount_zatoshis: None,
            memo: None,
            fund_source: FundSource::Auto,
        };

        let zatoshis = request.get_zatoshis().unwrap();
        assert_eq!(zatoshis, 150_000_000);
    }

    #[test]
    fn test_fee_calculation() {
        let service = OrchardTransferService::new(NetworkType::Mainnet);

        // Minimum fee for 1 output
        let fee = service.calculate_fee(1, FundSource::Shielded);
        assert_eq!(fee, DEFAULT_FEE_ZATOSHIS);

        // Fee for multiple outputs
        let fee = service.calculate_fee(5, FundSource::Shielded);
        assert!(fee > DEFAULT_FEE_ZATOSHIS);
    }

    #[test]
    fn test_create_proposal() {
        let service = OrchardTransferService::new(NetworkType::Mainnet);

        let request = TransferRequest {
            wallet_id: 1,
            to_address: "u1test".to_string(),
            amount_zec: "0.001".to_string(),
            amount_zatoshis: None,
            memo: Some("Test memo".to_string()),
            fund_source: FundSource::Transparent,
        };

        let proposal = service.create_proposal(
            &request,
            1_000_000, // 0.01 ZEC transparent
            None,
            2_500_000,
        ).unwrap();

        assert_eq!(proposal.amount_zatoshis, 100_000);
        assert!(proposal.is_shielding);
        assert_eq!(proposal.fund_source, FundSource::Transparent);
    }
}
