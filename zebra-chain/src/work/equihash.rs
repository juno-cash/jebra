//! Equihash and RandomX Solution types.
//!
//! Juno Cash uses RandomX proof-of-work with 32-byte solutions.
//! Zcash uses Equihash with 1344-byte solutions (or 36-byte on Regtest).

use std::{fmt, io};

use hex::{FromHex, FromHexError, ToHex};
use serde_big_array::BigArray;

use crate::{
    block::Header,
    serialization::{
        zcash_serialize_bytes, SerializationError, ZcashDeserialize, ZcashDeserializeInto,
        ZcashSerialize,
    },
    work::randomx::RANDOMX_SOLUTION_SIZE,
};

#[cfg(feature = "internal-miner")]
use crate::serialization::AtLeastOne;

/// The error type for Equihash validation.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
#[error("invalid equihash solution for BlockHeader")]
pub struct Error(#[from] equihash::Error);

/// The error type for Equihash solving.
#[derive(Copy, Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("solver was cancelled")]
pub struct SolverCancelled;

/// The size of an Equihash solution in bytes (always 1344).
pub(crate) const SOLUTION_SIZE: usize = 1344;

/// The size of an Equihash solution in bytes on Regtest (always 36).
pub(crate) const REGTEST_SOLUTION_SIZE: usize = 36;

/// Proof-of-work solution.
///
/// Juno Cash uses RandomX with 32-byte solutions.
/// Zcash uses Equihash with 1344-byte solutions (or 36-byte on Regtest).
#[derive(Deserialize, Serialize)]
// It's okay to use the extra space on Regtest
#[allow(clippy::large_enum_variant)]
pub enum Solution {
    /// RandomX solution (32 bytes) - used by Juno Cash
    RandomX([u8; RANDOMX_SOLUTION_SIZE]),
    /// Equihash solution on Mainnet or Testnet (1344 bytes)
    Common(#[serde(with = "BigArray")] [u8; SOLUTION_SIZE]),
    /// Equihash solution on Regtest (36 bytes)
    Regtest(#[serde(with = "BigArray")] [u8; REGTEST_SOLUTION_SIZE]),
}

impl Solution {
    /// The length of the portion of the header used as input when verifying
    /// equihash solutions, in bytes.
    ///
    /// Excludes the 32-byte nonce, which is passed as a separate argument
    /// to the verification function.
    pub const INPUT_LENGTH: usize = 4 + 32 * 3 + 4 * 2;

    /// Returns the inner value of the [`Solution`] as a byte slice.
    pub fn value(&self) -> &[u8] {
        match self {
            Solution::RandomX(solution) => solution.as_slice(),
            Solution::Common(solution) => solution.as_slice(),
            Solution::Regtest(solution) => solution.as_slice(),
        }
    }

    /// Returns `true` if this is a RandomX solution.
    pub fn is_randomx(&self) -> bool {
        matches!(self, Solution::RandomX(_))
    }

    /// Returns the RandomX hash if this is a RandomX solution.
    pub fn randomx_hash(&self) -> Option<[u8; RANDOMX_SOLUTION_SIZE]> {
        match self {
            Solution::RandomX(hash) => Some(*hash),
            _ => None,
        }
    }

    /// Returns `Ok(())` if `EquihashSolution` is valid for `header`
    ///
    /// Note: For RandomX solutions, use `check_randomx()` instead.
    #[allow(clippy::unwrap_in_result)]
    pub fn check(&self, header: &Header) -> Result<(), Error> {
        match self {
            Solution::RandomX(_) => {
                // RandomX validation is handled separately via check_randomx()
                // This is because RandomX requires the seed hash which comes from chain state
                Ok(())
            }
            Solution::Common(_) | Solution::Regtest(_) => {
                // Equihash validation
                let n = 200;
                let k = 9;
                let nonce = &header.nonce;

                let mut input = Vec::new();
                header
                    .zcash_serialize(&mut input)
                    .expect("serialization into a vec can't fail");

                // The part of the header before the nonce and solution.
                let input = &input[0..Solution::INPUT_LENGTH];

                equihash::is_valid_solution(n, k, input, nonce.as_ref(), self.value())?;

                Ok(())
            }
        }
    }

    /// Returns a [`Solution`] containing the bytes from `solution`.
    /// Returns an error if `solution` is the wrong length.
    pub fn from_bytes(solution: &[u8]) -> Result<Self, SerializationError> {
        match solution.len() {
            // RandomX solution (32 bytes) - Juno Cash
            RANDOMX_SOLUTION_SIZE => {
                let mut bytes = [0; RANDOMX_SOLUTION_SIZE];
                bytes.copy_from_slice(solution);
                Ok(Self::RandomX(bytes))
            }
            // Equihash solution (1344 bytes) - Zcash Mainnet/Testnet
            SOLUTION_SIZE => {
                let mut bytes = [0; SOLUTION_SIZE];
                bytes.copy_from_slice(solution);
                Ok(Self::Common(bytes))
            }
            // Equihash Regtest solution (36 bytes)
            REGTEST_SOLUTION_SIZE => {
                let mut bytes = [0; REGTEST_SOLUTION_SIZE];
                bytes.copy_from_slice(solution);
                Ok(Self::Regtest(bytes))
            }
            _unexpected_len => Err(SerializationError::Parse(
                "incorrect solution size",
            )),
        }
    }

    /// Returns a [`Solution`] of `[0; RANDOMX_SOLUTION_SIZE]` to be used in block proposals.
    /// For Juno Cash, this is a 32-byte RandomX solution placeholder.
    pub fn for_proposal() -> Self {
        // Juno Cash uses RandomX with 32-byte solutions
        Self::RandomX([0; RANDOMX_SOLUTION_SIZE])
    }

    /// Returns an empty RandomX solution for proposals.
    pub fn randomx_for_proposal() -> Self {
        Self::RandomX([0; RANDOMX_SOLUTION_SIZE])
    }

    /// Returns an empty Equihash solution for proposals (Zcash compatibility).
    pub fn equihash_for_proposal() -> Self {
        Self::Common([0; SOLUTION_SIZE])
    }

    /// Mines and returns one or more [`Solution`]s based on a template `header`.
    /// The returned header contains a valid `nonce` and `solution`.
    ///
    /// If `cancel_fn()` returns an error, returns early with `Err(SolverCancelled)`.
    ///
    /// The `nonce` in the header template is taken as the starting nonce. If you are running multiple
    /// solvers at the same time, start them with different nonces.
    /// The `solution` in the header template is ignored.
    ///
    /// For RandomX solutions (Juno Cash), pass the seed hash from the seed block.
    /// For Equihash solutions (Zcash), the seed parameter is ignored.
    ///
    /// This method is CPU and memory-intensive. RandomX uses ~256 MB RAM per thread.
    /// Equihash uses 144 MB RAM per thread. Mining can run for minutes or hours
    /// if the network difficulty is high.
    #[cfg(feature = "internal-miner")]
    #[allow(clippy::unwrap_in_result)]
    pub fn solve<F>(
        header: Header,
        cancel_fn: F,
        seed: Option<[u8; 32]>,
    ) -> Result<AtLeastOne<Header>, SolverCancelled>
    where
        F: FnMut() -> Result<(), SolverCancelled>,
    {
        // Route to the appropriate solver based on solution type
        match &header.solution {
            Solution::RandomX(_) => {
                let seed = seed.expect("RandomX mining requires a seed hash");
                Self::solve_randomx(header, seed, cancel_fn)
            }
            Solution::Common(_) | Solution::Regtest(_) => {
                Self::solve_equihash(header, cancel_fn)
            }
        }
    }

    /// Mines using the Equihash algorithm (for Zcash compatibility).
    #[cfg(feature = "internal-miner")]
    #[allow(clippy::unwrap_in_result)]
    fn solve_equihash<F>(
        mut header: Header,
        mut cancel_fn: F,
    ) -> Result<AtLeastOne<Header>, SolverCancelled>
    where
        F: FnMut() -> Result<(), SolverCancelled>,
    {
        use crate::shutdown::is_shutting_down;

        let mut input = Vec::new();
        header
            .zcash_serialize(&mut input)
            .expect("serialization into a vec can't fail");
        // Take the part of the header before the nonce and solution.
        // This data is kept constant for this solver run.
        let input = &input[0..Solution::INPUT_LENGTH];

        while !is_shutting_down() {
            // Don't run the solver if we'd just cancel it anyway.
            cancel_fn()?;

            let solutions = equihash::tromp::solve_200_9(input, || {
                // Cancel the solver if we have a new template.
                if cancel_fn().is_err() {
                    return None;
                }

                // This skips the first nonce, which doesn't matter in practice.
                Self::next_nonce(&mut header.nonce);
                Some(*header.nonce)
            });

            let mut valid_solutions = Vec::new();

            for solution in &solutions {
                header.solution = Self::from_bytes(solution)
                    .expect("unexpected invalid solution: incorrect length");

                // TODO: work out why we sometimes get invalid solutions here
                if let Err(error) = header.solution.check(&header) {
                    info!(?error, "found invalid solution for header");
                    continue;
                }

                if Self::difficulty_is_valid(&header) {
                    valid_solutions.push(header);
                }
            }

            match valid_solutions.try_into() {
                Ok(at_least_one_solution) => return Ok(at_least_one_solution),
                Err(_is_empty_error) => debug!(
                    solutions = ?solutions.len(),
                    "found valid solutions which did not pass the validity or difficulty checks"
                ),
            }
        }

        Err(SolverCancelled)
    }

    /// Returns `true` if the `nonce` and `solution` in `header` meet the difficulty threshold.
    ///
    /// # Panics
    ///
    /// - If `header` contains an invalid difficulty threshold.  
    #[cfg(feature = "internal-miner")]
    fn difficulty_is_valid(header: &Header) -> bool {
        // Simplified from zebra_consensus::block::check::difficulty_is_valid().
        let difficulty_threshold = header
            .difficulty_threshold
            .to_expanded()
            .expect("unexpected invalid header template: invalid difficulty threshold");

        // TODO: avoid calculating this hash multiple times
        let hash = header.hash();

        // Note: this comparison is a u256 integer comparison, like zcashd and bitcoin. Greater
        // values represent *less* work.
        hash <= difficulty_threshold
    }

    /// Modifies `nonce` to be the next integer in big-endian order.
    /// Wraps to zero if the next nonce would overflow.
    #[cfg(feature = "internal-miner")]
    fn next_nonce(nonce: &mut [u8; 32]) {
        let _ignore_overflow = crate::primitives::byte_array::increment_big_endian(&mut nonce[..]);
    }

    /// Mines and returns a [`Header`] with a valid RandomX solution.
    ///
    /// This function iterates through nonces, computing RandomX hashes until one
    /// meets the difficulty target. The hash itself becomes the solution.
    ///
    /// # Arguments
    /// * `header` - The block header template with nonce as starting point
    /// * `seed` - The 32-byte RandomX seed (from seed block hash or genesis)
    /// * `cancel_fn` - Called periodically; returns `Err` to cancel mining
    ///
    /// # Returns
    /// * `Ok(headers)` - At least one header with valid nonce and RandomX solution
    /// * `Err(SolverCancelled)` - Mining was cancelled via cancel_fn or shutdown
    #[cfg(feature = "internal-miner")]
    #[allow(clippy::unwrap_in_result)]
    pub fn solve_randomx<F>(
        mut header: Header,
        seed: [u8; 32],
        mut cancel_fn: F,
    ) -> Result<AtLeastOne<Header>, SolverCancelled>
    where
        F: FnMut() -> Result<(), SolverCancelled>,
    {
        use crate::shutdown::is_shutting_down;
        use crate::work::randomx::{hash_meets_target, randomx_hash};

        // Get the expanded difficulty threshold for comparison
        let difficulty_threshold = header
            .difficulty_threshold
            .to_expanded()
            .expect("unexpected invalid header template: invalid difficulty threshold");

        while !is_shutting_down() {
            // Check for cancellation before computing hash
            cancel_fn()?;

            // Get the 140-byte mining input (includes current nonce)
            let mining_input = header.mining_input();

            // Compute the RandomX hash
            match randomx_hash(&seed, &mining_input) {
                Ok(hash) => {
                    // Check if hash meets difficulty target
                    if hash_meets_target(&hash, &difficulty_threshold) {
                        // The hash IS the solution for RandomX
                        header.solution = Solution::RandomX(hash);

                        info!(
                            nonce = %hex::encode(&header.nonce[..]),
                            hash = %hex::encode(&hash),
                            "found valid RandomX solution"
                        );

                        return Ok(vec![header]
                            .try_into()
                            .expect("vec with one element is non-empty"));
                    }
                }
                Err(e) => {
                    // Log error and continue trying with next nonce
                    debug!(?e, "RandomX hash computation failed, trying next nonce");
                }
            }

            // Increment nonce for next iteration
            Self::next_nonce(&mut header.nonce);
        }

        Err(SolverCancelled)
    }
}

impl PartialEq<Solution> for Solution {
    fn eq(&self, other: &Solution) -> bool {
        self.value() == other.value()
    }
}

impl fmt::Debug for Solution {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Solution::RandomX(_) => f
                .debug_tuple("RandomXSolution")
                .field(&hex::encode(self.value()))
                .finish(),
            Solution::Common(_) | Solution::Regtest(_) => f
                .debug_tuple("EquihashSolution")
                .field(&hex::encode(self.value()))
                .finish(),
        }
    }
}

// These impls all only exist because of array length restrictions.

impl Copy for Solution {}

impl Clone for Solution {
    fn clone(&self) -> Self {
        *self
    }
}

impl Eq for Solution {}

#[cfg(any(test, feature = "proptest-impl"))]
impl Default for Solution {
    fn default() -> Self {
        // Juno Cash uses RandomX with 32-byte solutions
        Self::RandomX([0; RANDOMX_SOLUTION_SIZE])
    }
}

impl ZcashSerialize for Solution {
    fn zcash_serialize<W: io::Write>(&self, writer: W) -> Result<(), io::Error> {
        zcash_serialize_bytes(&self.value().to_vec(), writer)
    }
}

impl ZcashDeserialize for Solution {
    fn zcash_deserialize<R: io::Read>(mut reader: R) -> Result<Self, SerializationError> {
        let solution: Vec<u8> = (&mut reader).zcash_deserialize_into()?;
        Self::from_bytes(&solution)
    }
}

impl ToHex for &Solution {
    fn encode_hex<T: FromIterator<char>>(&self) -> T {
        self.value().encode_hex()
    }

    fn encode_hex_upper<T: FromIterator<char>>(&self) -> T {
        self.value().encode_hex_upper()
    }
}

impl ToHex for Solution {
    fn encode_hex<T: FromIterator<char>>(&self) -> T {
        (&self).encode_hex()
    }

    fn encode_hex_upper<T: FromIterator<char>>(&self) -> T {
        (&self).encode_hex_upper()
    }
}

impl FromHex for Solution {
    type Error = FromHexError;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        let bytes = Vec::from_hex(hex)?;
        Solution::from_bytes(&bytes).map_err(|_| FromHexError::InvalidStringLength)
    }
}
