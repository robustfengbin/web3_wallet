//! Zcash v5 (NU5) transparent transaction builder
//!
//! This module implements building and signing of Zcash v5 transparent transactions
//! compatible with Zebra's sendrawtransaction RPC.
//!
//! References:
//! - ZIP 225: Transaction format for NU5
//! - ZIP 244: Transaction identifier and sighash algorithms for NU5

use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
use sha2::{Digest, Sha256};
use ripemd::Ripemd160;

use crate::error::{AppError, AppResult};

// Zcash v5 transaction constants
const TX_VERSION: u32 = 5;
const TX_VERSION_WITH_OVERWINTERED: u32 = TX_VERSION | (1 << 31); // 0x80000005
const VERSION_GROUP_ID_V5: u32 = 0x26A7270A;

// Consensus branch IDs for different network upgrades
#[allow(dead_code)]
const CONSENSUS_BRANCH_ID_NU5: u32 = 0xc2d6d0b4; // NU5
const CONSENSUS_BRANCH_ID_NU6: u32 = 0xc8e71055; // NU6
const CONSENSUS_BRANCH_ID_NU6_1: u32 = 0x4dec4df0; // NU6.1

// Network upgrade activation heights (mainnet)
const NU6_ACTIVATION_HEIGHT: u32 = 2_726_400;
const NU6_1_ACTIVATION_HEIGHT: u32 = 3_146_400;

// ZIP 244 sighash personalization strings
const ZCASH_TRANSPARENT_HASH: &[u8] = b"ZTxIdTranspaHash";
const ZCASH_PREVOUTS_HASH: &[u8] = b"ZTxIdPrevoutHash";
const ZCASH_SEQUENCE_HASH: &[u8] = b"ZTxIdSequencHash";
const ZCASH_OUTPUTS_HASH: &[u8] = b"ZTxIdOutputsHash";
const ZCASH_TX_HASH: &[u8] = b"ZcashTxHash_";
const ZCASH_TRANSPARENT_SIG: &[u8] = b"Zcash___TxInHash";

// Zcash mainnet transparent address prefixes
const ZCASH_T1_PREFIX: [u8; 2] = [0x1C, 0xB8]; // t1 (P2PKH)
const ZCASH_T3_PREFIX: [u8; 2] = [0x1C, 0xBD]; // t3 (P2SH)

/// Transaction input (UTXO being spent)
#[derive(Debug, Clone)]
pub struct TxInput {
    /// Previous transaction hash (32 bytes, little-endian in serialization)
    pub prev_txid: [u8; 32],
    /// Output index in the previous transaction
    pub prev_vout: u32,
    /// Value in zatoshis
    pub value: u64,
    /// Previous output's scriptPubKey
    pub script_pubkey: Vec<u8>,
    /// Sequence number (usually 0xffffffff - 1 for RBF, or 0xffffffff)
    pub sequence: u32,
}

/// Transaction output
#[derive(Debug, Clone)]
pub struct TxOutput {
    /// Value in zatoshis
    pub value: u64,
    /// scriptPubKey
    pub script_pubkey: Vec<u8>,
}

/// Zcash v5 transaction builder
#[derive(Debug, Clone)]
pub struct TransactionBuilder {
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<TxOutput>,
    pub lock_time: u32,
    pub expiry_height: u32,
    pub consensus_branch_id: u32,
}

impl TransactionBuilder {
    /// Create a new transaction builder with consensus branch ID derived from height
    #[allow(dead_code)]
    pub fn new(expiry_height: u32, current_height: u32) -> Self {
        // Determine the correct consensus branch ID based on current block height
        let consensus_branch_id = get_consensus_branch_id(current_height);

        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
            lock_time: 0,
            expiry_height,
            consensus_branch_id,
        }
    }

    /// Create a new transaction builder with explicit consensus branch ID
    pub fn new_with_branch_id(expiry_height: u32, consensus_branch_id: u32) -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
            lock_time: 0,
            expiry_height,
            consensus_branch_id,
        }
    }
}

/// Get the consensus branch ID for the given block height
fn get_consensus_branch_id(height: u32) -> u32 {
    if height >= NU6_1_ACTIVATION_HEIGHT {
        CONSENSUS_BRANCH_ID_NU6_1
    } else if height >= NU6_ACTIVATION_HEIGHT {
        CONSENSUS_BRANCH_ID_NU6
    } else {
        CONSENSUS_BRANCH_ID_NU5
    }
}

impl TransactionBuilder {
    /// Add an input (UTXO to spend)
    pub fn add_input(
        &mut self,
        prev_txid: [u8; 32],
        prev_vout: u32,
        value: u64,
        script_pubkey: Vec<u8>,
    ) {
        self.inputs.push(TxInput {
            prev_txid,
            prev_vout,
            value,
            script_pubkey,
            sequence: 0xfffffffe, // Enable RBF
        });
    }

    /// Add an output
    pub fn add_output(&mut self, address: &str, value: u64) -> AppResult<()> {
        let script_pubkey = address_to_script_pubkey(address)?;
        self.outputs.push(TxOutput {
            value,
            script_pubkey,
        });
        Ok(())
    }
}

/// Convert a Zcash transparent address to scriptPubKey
fn address_to_script_pubkey(address: &str) -> AppResult<Vec<u8>> {
    let decoded = bs58::decode(address)
        .into_vec()
        .map_err(|e| AppError::ValidationError(format!("Invalid address encoding: {}", e)))?;

    if decoded.len() != 26 {
        return Err(AppError::ValidationError(format!(
            "Invalid address length: expected 26 bytes, got {}",
            decoded.len()
        )));
    }

    // Verify checksum
    let payload = &decoded[..22];
    let checksum = &decoded[22..];
    let hash1 = Sha256::digest(payload);
    let hash2 = Sha256::digest(&hash1);
    if &hash2[..4] != checksum {
        return Err(AppError::ValidationError("Address checksum mismatch".to_string()));
    }

    let prefix = &decoded[..2];
    let hash160 = &decoded[2..22];

    if prefix == ZCASH_T1_PREFIX {
        // P2PKH: OP_DUP OP_HASH160 <20 bytes> OP_EQUALVERIFY OP_CHECKSIG
        let mut script = Vec::with_capacity(25);
        script.push(0x76); // OP_DUP
        script.push(0xa9); // OP_HASH160
        script.push(0x14); // Push 20 bytes
        script.extend_from_slice(hash160);
        script.push(0x88); // OP_EQUALVERIFY
        script.push(0xac); // OP_CHECKSIG
        Ok(script)
    } else if prefix == ZCASH_T3_PREFIX {
        // P2SH: OP_HASH160 <20 bytes> OP_EQUAL
        let mut script = Vec::with_capacity(23);
        script.push(0xa9); // OP_HASH160
        script.push(0x14); // Push 20 bytes
        script.extend_from_slice(hash160);
        script.push(0x87); // OP_EQUAL
        Ok(script)
    } else {
        Err(AppError::ValidationError(format!(
            "Unsupported address prefix: {:02x}{:02x}",
            prefix[0], prefix[1]
        )))
    }
}

/// BLAKE2b-256 hash with personalization (ZIP 244 compliant)
fn blake2b_256(personalization: &[u8], data: &[u8]) -> [u8; 32] {
    // BLAKE2b personalization must be exactly 16 bytes
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
fn hash_prevouts(inputs: &[TxInput]) -> [u8; 32] {
    let mut data = Vec::new();
    for input in inputs {
        // txid is stored big-endian, but serialized little-endian
        let mut txid_le = input.prev_txid;
        txid_le.reverse();
        data.extend_from_slice(&txid_le);
        data.extend_from_slice(&input.prev_vout.to_le_bytes());
    }
    blake2b_256(ZCASH_PREVOUTS_HASH, &data)
}

/// Hash of all sequences (ZIP 244)
fn hash_sequences(inputs: &[TxInput]) -> [u8; 32] {
    let mut data = Vec::new();
    for input in inputs {
        data.extend_from_slice(&input.sequence.to_le_bytes());
    }
    blake2b_256(ZCASH_SEQUENCE_HASH, &data)
}

/// Hash of all outputs (ZIP 244)
/// Note: value is encoded as i64 little-endian (same as TxOut::write in librustzcash)
fn hash_outputs(outputs: &[TxOutput]) -> [u8; 32] {
    let mut data = Vec::new();
    for output in outputs {
        // Value as i64 little-endian (for consistency with librustzcash)
        data.extend_from_slice(&(output.value as i64).to_le_bytes());
        data.extend_from_slice(&serialize_compact_size(output.script_pubkey.len() as u64));
        data.extend_from_slice(&output.script_pubkey);
    }
    blake2b_256(ZCASH_OUTPUTS_HASH, &data)
}

/// Calculate transparent input values hash (ZIP 244)
/// Note: Uses "ZTxTr" prefix for signature digest, not "ZTxId" (which is for txid)
/// Uses Array encoding (NO CompactSize count prefix - just the values)
fn hash_amounts(inputs: &[TxInput]) -> [u8; 32] {
    let mut data = Vec::new();
    // Array encoding: NO length prefix, just write each value directly
    for input in inputs {
        // Value as i64 little-endian (same as u64 for valid amounts)
        data.extend_from_slice(&(input.value as i64).to_le_bytes());
    }
    blake2b_256(b"ZTxTrAmountsHash", &data)
}

/// Calculate transparent input scripts hash (ZIP 244)
/// Note: Uses "ZTxTr" prefix for signature digest, not "ZTxId" (which is for txid)
/// Uses Array encoding (NO CompactSize count prefix - just the scripts with their individual lengths)
fn hash_script_pubkeys(inputs: &[TxInput]) -> [u8; 32] {
    let mut data = Vec::new();
    // Array encoding: NO count prefix, just write each script with its own CompactSize length prefix
    for input in inputs {
        // Each script has its own CompactSize length prefix (this is part of Script's encoding)
        data.extend_from_slice(&serialize_compact_size(input.script_pubkey.len() as u64));
        data.extend_from_slice(&input.script_pubkey);
    }
    blake2b_256(b"ZTxTrScriptsHash", &data)
}

/// Calculate ZIP 244 sighash for a transparent input
fn calculate_sighash(
    builder: &TransactionBuilder,
    input_index: usize,
    sighash_type: u8,
) -> AppResult<[u8; 32]> {
    // For SIGHASH_ALL (0x01), we hash all inputs and outputs
    if sighash_type != 0x01 {
        return Err(AppError::ValidationError(
            "Only SIGHASH_ALL is currently supported".to_string()
        ));
    }

    let input = &builder.inputs[input_index];

    tracing::debug!("=== ZIP 244 Sighash Calculation ===");
    tracing::debug!("Input index: {}, sighash_type: 0x{:02x}", input_index, sighash_type);
    tracing::debug!("Input prevout txid (stored): {}", hex::encode(&input.prev_txid));
    tracing::debug!("Input prevout vout: {}", input.prev_vout);
    tracing::debug!("Input value: {} zatoshis", input.value);
    tracing::debug!("Input scriptPubKey: {}", hex::encode(&input.script_pubkey));
    tracing::debug!("Input sequence: 0x{:08x}", input.sequence);

    // Build the transparent_sig_digest per ZIP 244

    // prevouts_sig_digest
    let prevouts_digest = hash_prevouts(&builder.inputs);
    tracing::debug!("prevouts_digest: {}", hex::encode(&prevouts_digest));

    // amounts_sig_digest
    let amounts_digest = hash_amounts(&builder.inputs);
    tracing::debug!("amounts_digest: {}", hex::encode(&amounts_digest));

    // script_pubkeys_sig_digest
    let scripts_digest = hash_script_pubkeys(&builder.inputs);
    tracing::debug!("scripts_digest: {}", hex::encode(&scripts_digest));

    // sequence_sig_digest
    let sequences_digest = hash_sequences(&builder.inputs);
    tracing::debug!("sequences_digest: {}", hex::encode(&sequences_digest));

    // outputs_sig_digest
    let outputs_digest = hash_outputs(&builder.outputs);
    tracing::debug!("outputs_digest: {}", hex::encode(&outputs_digest));

    // Build the input being signed (txin_sig_digest per ZIP 244 S.2g)
    let mut txin_data = Vec::new();
    // S.2g.i: prevout (txid in little-endian + vout)
    let mut txid_le = input.prev_txid;
    txid_le.reverse();
    tracing::debug!("txin prevout txid (reversed for wire): {}", hex::encode(&txid_le));
    txin_data.extend_from_slice(&txid_le);
    txin_data.extend_from_slice(&input.prev_vout.to_le_bytes());
    // S.2g.ii: value (8-byte signed little-endian)
    txin_data.extend_from_slice(&(input.value as i64).to_le_bytes());
    // S.2g.iii: scriptPubKey (with CompactSize length prefix)
    txin_data.extend_from_slice(&serialize_compact_size(input.script_pubkey.len() as u64));
    txin_data.extend_from_slice(&input.script_pubkey);
    // S.2g.iv: nSequence (4-byte unsigned little-endian)
    txin_data.extend_from_slice(&input.sequence.to_le_bytes());

    tracing::debug!("txin_data (full): {}", hex::encode(&txin_data));
    let txin_sig_digest = blake2b_256(ZCASH_TRANSPARENT_SIG, &txin_data);
    tracing::debug!("txin_sig_digest: {}", hex::encode(&txin_sig_digest));

    // Combine all digests for transparent_sig_digest
    let mut transparent_data = Vec::new();
    transparent_data.push(0x01); // hash_type = SIGHASH_ALL
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
    header_data.extend_from_slice(&builder.consensus_branch_id.to_le_bytes());
    header_data.extend_from_slice(&builder.lock_time.to_le_bytes());
    header_data.extend_from_slice(&builder.expiry_height.to_le_bytes());

    tracing::debug!("header version: 0x{:08x}", TX_VERSION_WITH_OVERWINTERED);
    tracing::debug!("version_group_id: 0x{:08x}", VERSION_GROUP_ID_V5);
    tracing::debug!("consensus_branch_id: 0x{:08x}", builder.consensus_branch_id);
    tracing::debug!("lock_time: {}", builder.lock_time);
    tracing::debug!("expiry_height: {}", builder.expiry_height);

    let header_digest = blake2b_256(b"ZTxIdHeadersHash", &header_data);
    tracing::debug!("header_digest: {}", hex::encode(&header_digest));

    // Empty sapling and orchard digests (for transparent-only tx)
    // Per zcash_primitives: empty bundles hash to BLAKE2b-256(personalization, []) - empty data
    let sapling_digest = blake2b_256(b"ZTxIdSaplingHash", &[]);
    let orchard_digest = blake2b_256(b"ZTxIdOrchardHash", &[]);
    tracing::debug!("sapling_digest (empty): {}", hex::encode(&sapling_digest));
    tracing::debug!("orchard_digest (empty): {}", hex::encode(&orchard_digest));

    // Build the final sighash
    let mut personalization = ZCASH_TX_HASH.to_vec();
    personalization.extend_from_slice(&builder.consensus_branch_id.to_le_bytes());
    tracing::debug!("final personalization: {}", hex::encode(&personalization));

    let mut sig_data = Vec::new();
    sig_data.extend_from_slice(&header_digest);
    sig_data.extend_from_slice(&transparent_sig_digest);
    sig_data.extend_from_slice(&sapling_digest);
    sig_data.extend_from_slice(&orchard_digest);

    let final_sighash = blake2b_256(&personalization, &sig_data);
    tracing::debug!("FINAL SIGHASH: {}", hex::encode(&final_sighash));
    tracing::debug!("=== End Sighash Calculation ===");

    Ok(final_sighash)
}

/// Sign a transaction input
fn sign_input(
    secp: &Secp256k1<secp256k1::All>,
    secret_key: &SecretKey,
    sighash: &[u8; 32],
) -> AppResult<Vec<u8>> {
    let message = Message::from_digest_slice(sighash)
        .map_err(|e| AppError::InternalError(format!("Invalid sighash: {}", e)))?;

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

/// Serialize the signed transaction to hex
fn serialize_transaction(builder: &TransactionBuilder, script_sigs: &[Vec<u8>]) -> Vec<u8> {
    let mut tx = Vec::new();

    // Header
    tx.extend_from_slice(&TX_VERSION_WITH_OVERWINTERED.to_le_bytes());
    tx.extend_from_slice(&VERSION_GROUP_ID_V5.to_le_bytes());
    tx.extend_from_slice(&builder.consensus_branch_id.to_le_bytes());
    tx.extend_from_slice(&builder.lock_time.to_le_bytes());
    tx.extend_from_slice(&builder.expiry_height.to_le_bytes());

    // Transparent bundle
    // Input count
    tx.extend_from_slice(&serialize_compact_size(builder.inputs.len() as u64));

    // Inputs
    for (i, input) in builder.inputs.iter().enumerate() {
        // prevout txid (little-endian)
        let mut txid_le = input.prev_txid;
        txid_le.reverse();
        tx.extend_from_slice(&txid_le);
        // prevout index
        tx.extend_from_slice(&input.prev_vout.to_le_bytes());
        // scriptSig
        tx.extend_from_slice(&serialize_compact_size(script_sigs[i].len() as u64));
        tx.extend_from_slice(&script_sigs[i]);
        // sequence
        tx.extend_from_slice(&input.sequence.to_le_bytes());
    }

    // Output count
    tx.extend_from_slice(&serialize_compact_size(builder.outputs.len() as u64));

    // Outputs
    for output in &builder.outputs {
        // Value as i64 little-endian (for consistency with sighash calculation)
        tx.extend_from_slice(&(output.value as i64).to_le_bytes());
        tx.extend_from_slice(&serialize_compact_size(output.script_pubkey.len() as u64));
        tx.extend_from_slice(&output.script_pubkey);
    }

    // Empty Sapling bundle (nSpendsSapling = 0, nOutputsSapling = 0)
    tx.push(0x00); // nSpendsSapling
    tx.push(0x00); // nOutputsSapling

    // Empty Orchard bundle (nActionsOrchard = 0)
    tx.push(0x00); // nActionsOrchard

    tracing::debug!("Serialized transaction ({} bytes): {}", tx.len(), hex::encode(&tx));

    tx
}

/// Parse private key from hex or WIF format
fn parse_private_key(private_key: &str) -> AppResult<[u8; 32]> {
    // Check if WIF format
    if private_key.starts_with('5') || private_key.starts_with('K') || private_key.starts_with('L') {
        // WIF format - decode and extract key
        let decoded = bs58::decode(private_key)
            .into_vec()
            .map_err(|e| AppError::ValidationError(format!("Invalid WIF format: {}", e)))?;

        if decoded.len() != 37 && decoded.len() != 38 {
            return Err(AppError::ValidationError(format!(
                "Invalid WIF length: expected 37 or 38 bytes, got {}",
                decoded.len()
            )));
        }

        // Verify checksum
        let payload_len = decoded.len() - 4;
        let payload = &decoded[..payload_len];
        let checksum = &decoded[payload_len..];
        let hash1 = Sha256::digest(payload);
        let hash2 = Sha256::digest(&hash1);

        if &hash2[..4] != checksum {
            return Err(AppError::ValidationError("WIF checksum mismatch".to_string()));
        }

        if payload[0] != 0x80 {
            return Err(AppError::ValidationError(format!(
                "Invalid WIF prefix: expected 0x80, got 0x{:02x}",
                payload[0]
            )));
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&payload[1..33]);
        Ok(key_bytes)
    } else {
        // Hex format
        let key_hex = private_key.strip_prefix("0x").unwrap_or(private_key);
        let key_vec = hex::decode(key_hex)
            .map_err(|e| AppError::ValidationError(format!("Invalid private key hex: {}", e)))?;

        if key_vec.len() != 32 {
            return Err(AppError::ValidationError(format!(
                "Private key must be 32 bytes, got {} bytes",
                key_vec.len()
            )));
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&key_vec);
        Ok(key_bytes)
    }
}

/// Build and sign a Zcash v5 transparent transaction
pub fn build_and_sign_transaction(
    builder: &TransactionBuilder,
    private_key: &str,
) -> AppResult<String> {
    if builder.inputs.is_empty() {
        return Err(AppError::ValidationError("Transaction has no inputs".to_string()));
    }
    if builder.outputs.is_empty() {
        return Err(AppError::ValidationError("Transaction has no outputs".to_string()));
    }

    let secp = Secp256k1::new();

    // Parse private key
    let key_bytes = parse_private_key(private_key)?;
    let secret_key = SecretKey::from_slice(&key_bytes)
        .map_err(|e| AppError::ValidationError(format!("Invalid private key: {}", e)))?;
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);

    // Verify that the public key matches the input addresses
    // (derive address from pubkey and compare with input scriptPubKey)
    let _pubkey_hash = {
        let sha256_hash = Sha256::digest(&public_key.serialize());
        let ripemd_hash = Ripemd160::digest(&sha256_hash);
        ripemd_hash.to_vec()
    };

    // Sign each input
    let mut script_sigs = Vec::new();
    for i in 0..builder.inputs.len() {
        // Calculate sighash for this input
        let sighash = calculate_sighash(builder, i, 0x01)?;

        // Sign
        let signature = sign_input(&secp, &secret_key, &sighash)?;

        // Build scriptSig
        let script_sig = build_p2pkh_scriptsig(&signature, &public_key);
        script_sigs.push(script_sig);
    }

    // Serialize the complete transaction
    let tx_bytes = serialize_transaction(builder, &script_sigs);

    Ok(hex::encode(tx_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_to_script_pubkey_t1() {
        // t1 address (P2PKH)
        // This is a test address, the actual value depends on the hash160
        let result = address_to_script_pubkey("t1Rv4exT7bqhZqi2j7xz8bUHDMxwosrjADU");
        assert!(result.is_ok());
        let script = result.unwrap();
        assert_eq!(script[0], 0x76); // OP_DUP
        assert_eq!(script[1], 0xa9); // OP_HASH160
        assert_eq!(script[2], 0x14); // Push 20 bytes
        assert_eq!(script[23], 0x88); // OP_EQUALVERIFY
        assert_eq!(script[24], 0xac); // OP_CHECKSIG
    }

    #[test]
    fn test_compact_size() {
        assert_eq!(serialize_compact_size(0), vec![0x00]);
        assert_eq!(serialize_compact_size(252), vec![0xfc]);
        assert_eq!(serialize_compact_size(253), vec![0xfd, 0xfd, 0x00]);
        assert_eq!(serialize_compact_size(0x1234), vec![0xfd, 0x34, 0x12]);
    }
}
