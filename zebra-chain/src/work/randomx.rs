//! RandomX Proof-of-Work implementation for Juno Cash.
//!
//! RandomX is a CPU-friendly proof-of-work algorithm that uses random code execution
//! and memory-hard techniques. This module provides the Juno Cash-specific configuration
//! and integration with the randomx-rs crate.

use std::sync::{Arc, Mutex, OnceLock};

use crate::block::Height;
use crate::work::difficulty::{ExpandedDifficulty, U256};

/// The size of a RandomX solution/hash in bytes.
pub const RANDOMX_SOLUTION_SIZE: usize = 32;

/// Number of blocks per RandomX epoch (seed changes every 2048 blocks).
pub const RANDOMX_SEEDHASH_EPOCH_BLOCKS: u64 = 2048;

/// Lag before a new seed becomes active (96 blocks).
pub const RANDOMX_SEEDHASH_EPOCH_LAG: u64 = 96;

/// Calculate the seed height for a given block height.
///
/// The seed changes every 2048 blocks with a 96-block lag.
/// First epoch transition occurs at block 2144 (2048 + 96).
///
/// For heights <= EPOCH_BLOCKS + LAG, returns 0 (genesis seed).
/// Otherwise, returns the most recent epoch boundary that is at least LAG blocks behind.
///
/// This matches Juno Cash's RandomX_SeedHeight() function exactly.
pub fn seed_height(height: u64) -> u64 {
    if height <= RANDOMX_SEEDHASH_EPOCH_BLOCKS + RANDOMX_SEEDHASH_EPOCH_LAG {
        return 0;
    }
    // Calculate: (height - LAG - 1) & ~(EPOCH_BLOCKS - 1)
    // This rounds down to the nearest epoch boundary
    // The -1 is important to match Juno Cash's formula exactly
    (height - RANDOMX_SEEDHASH_EPOCH_LAG - 1) & !(RANDOMX_SEEDHASH_EPOCH_BLOCKS - 1)
}

/// Returns the genesis epoch seed.
///
/// For the first epoch (blocks 0 through 2143), the seed is 0x08 followed by 31 zero bytes.
/// This matches the Juno Cash implementation.
pub fn genesis_seed() -> [u8; 32] {
    let mut seed = [0u8; 32];
    seed[0] = 0x08;
    seed
}

/// Get the seed hash for a given block height.
///
/// For heights in the genesis epoch, returns the genesis seed.
/// For later heights, returns the block hash at the seed height.
///
/// Note: This function requires access to block hashes, so it returns
/// the seed height and whether to use genesis seed. The actual seed
/// lookup must be done by the caller with access to the chain state.
pub fn get_seed_info(height: u64) -> SeedInfo {
    let seed_h = seed_height(height);
    if seed_h == 0 {
        SeedInfo::Genesis(genesis_seed())
    } else {
        SeedInfo::BlockHash(Height(seed_h as u32))
    }
}

/// Information about which seed to use for RandomX.
#[derive(Debug, Clone)]
pub enum SeedInfo {
    /// Use the genesis seed (first epoch).
    Genesis([u8; 32]),
    /// Use the block hash at the given height as the seed.
    BlockHash(Height),
}

/// Global RandomX VM cache manager.
///
/// This manages RandomX VMs and caches to avoid expensive reinitialization.
/// VMs are cached per seed hash.
static RANDOMX_CACHE: OnceLock<Arc<Mutex<RandomXCache>>> = OnceLock::new();

/// RandomX cache manager for efficient VM reuse.
pub struct RandomXCache {
    /// Current seed hash
    current_seed: Option<[u8; 32]>,
    /// Whether we're in fast mode (full dataset) or light mode (cache only)
    fast_mode: bool,
}

impl RandomXCache {
    /// Create a new RandomX cache manager.
    pub fn new(fast_mode: bool) -> Self {
        Self {
            current_seed: None,
            fast_mode,
        }
    }

    /// Get the global cache instance.
    pub fn global() -> Arc<Mutex<RandomXCache>> {
        RANDOMX_CACHE
            .get_or_init(|| Arc::new(Mutex::new(RandomXCache::new(false))))
            .clone()
    }
}

/// Calculate a RandomX hash.
///
/// # Arguments
/// * `seed` - The 32-byte seed hash (block hash at seed height or genesis seed)
/// * `input` - The input data to hash (typically serialized block header)
///
/// # Returns
/// The 32-byte RandomX hash, or an error if hashing fails.
pub fn randomx_hash(seed: &[u8; 32], input: &[u8]) -> Result<[u8; 32], RandomXError> {
    use randomx_rs::{RandomXCache, RandomXFlag, RandomXVM};

    // Create flags for light mode (no full dataset)
    let flags = RandomXFlag::get_recommended_flags();

    // Create cache with the seed
    let cache = RandomXCache::new(flags, seed).map_err(|e| RandomXError::CacheInit(e.to_string()))?;

    // Create VM
    let vm = RandomXVM::new(flags, Some(cache), None).map_err(|e| RandomXError::VmInit(e.to_string()))?;

    // Calculate hash
    let hash = vm.calculate_hash(input).map_err(|e| RandomXError::Hash(e.to_string()))?;

    // Convert to fixed-size array
    let mut result = [0u8; 32];
    result.copy_from_slice(&hash);
    Ok(result)
}

/// Verify a RandomX proof of work.
///
/// # Arguments
/// * `seed` - The 32-byte seed hash
/// * `input` - The input data (serialized block header without solution)
/// * `expected_hash` - The expected hash value (stored in the solution field)
///
/// # Returns
/// `Ok(())` if the hash matches, or an error if verification fails.
pub fn verify_randomx(
    seed: &[u8; 32],
    input: &[u8],
    expected_hash: &[u8; 32],
) -> Result<(), RandomXError> {
    let computed_hash = randomx_hash(seed, input)?;

    if computed_hash == *expected_hash {
        Ok(())
    } else {
        Err(RandomXError::HashMismatch {
            expected: hex::encode(expected_hash),
            computed: hex::encode(computed_hash),
        })
    }
}

/// Check if a RandomX hash meets the difficulty target.
///
/// The hash is interpreted as a little-endian 256-bit integer and compared
/// against the expanded difficulty threshold. Returns `true` if the hash
/// represents valid proof-of-work (hash <= target).
///
/// This matches the difficulty check in Bitcoin/Zcash where lower hash values
/// represent more work.
pub fn hash_meets_target(hash: &[u8; 32], target: &ExpandedDifficulty) -> bool {
    // Convert hash to U256 using little-endian interpretation
    let hash_value = U256::from_little_endian(hash);
    let hash_as_difficulty = ExpandedDifficulty::from(hash_value);

    // Hash must be less than or equal to target (lower = more work)
    hash_as_difficulty <= *target
}

/// Errors that can occur during RandomX operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RandomXError {
    /// Failed to initialize RandomX cache.
    #[error("failed to initialize RandomX cache: {0}")]
    CacheInit(String),

    /// Failed to initialize RandomX VM.
    #[error("failed to initialize RandomX VM: {0}")]
    VmInit(String),

    /// Failed to calculate hash.
    #[error("failed to calculate RandomX hash: {0}")]
    Hash(String),

    /// Hash verification failed.
    #[error("RandomX hash mismatch: expected {expected}, computed {computed}")]
    HashMismatch { expected: String, computed: String },

    /// Invalid seed.
    #[error("invalid RandomX seed")]
    InvalidSeed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_height_genesis_epoch() {
        // First epoch: heights 0-2143 should use seed height 0
        assert_eq!(seed_height(0), 0);
        assert_eq!(seed_height(1), 0);
        assert_eq!(seed_height(2048), 0);
        assert_eq!(seed_height(2143), 0);
        assert_eq!(seed_height(2144), 0); // 2048 + 96 = 2144, still uses 0
    }

    #[test]
    fn test_seed_height_second_epoch() {
        // Second epoch starts at 2145, uses seed from height 2048
        assert_eq!(seed_height(2145), 2048);
        assert_eq!(seed_height(4000), 2048);
        assert_eq!(seed_height(4192), 2048); // 4096 + 96 = 4192, still uses 2048
    }

    #[test]
    fn test_seed_height_later_epochs() {
        // Third epoch starts at 4193
        assert_eq!(seed_height(4193), 4096);
        assert_eq!(seed_height(6000), 4096);
        assert_eq!(seed_height(6240), 4096); // 6144 + 96 = 6240, still uses 4096

        // Fourth epoch starts at 6241
        assert_eq!(seed_height(6241), 6144);
    }

    #[test]
    fn test_genesis_seed() {
        let seed = genesis_seed();
        assert_eq!(seed[0], 0x08);
        assert!(seed[1..].iter().all(|&b| b == 0));
    }

    #[test]
    fn test_get_seed_info() {
        // Genesis epoch
        match get_seed_info(100) {
            SeedInfo::Genesis(seed) => {
                assert_eq!(seed[0], 0x08);
            }
            _ => panic!("expected genesis seed"),
        }

        // Later epoch (height 5000 is in epoch 2, uses seed from block 2048)
        match get_seed_info(5000) {
            SeedInfo::BlockHash(height) => {
                assert_eq!(height.0, 2048);
            }
            _ => panic!("expected block hash seed"),
        }
    }
}
