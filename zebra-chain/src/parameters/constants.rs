//! Definitions of Zebra chain constants, including:
//! - slow start interval,
//! - slow start shift

use crate::block::Height;

/// An initial period from Genesis to this Height where the block subsidy is gradually incremented. [What is slow-start mining][slow-mining]
///
/// [slow-mining]: https://z.cash/support/faq/#what-is-slow-start-mining
pub const SLOW_START_INTERVAL: Height = Height(20_000);

/// `SlowStartShift()` as described in [protocol specification §7.8][7.8]
///
/// [7.8]: https://zips.z.cash/protocol/protocol.pdf#subsidies
///
/// This calculation is exact, because `SLOW_START_INTERVAL` is divisible by 2.
pub const SLOW_START_SHIFT: Height = Height(SLOW_START_INTERVAL.0 / 2);

/// Magic numbers used to identify different Juno Cash networks.
pub mod magics {
    use crate::parameters::network::magic::Magic;

    /// The production mainnet.
    pub const MAINNET: Magic = Magic([0xb5, 0x0c, 0x07, 0x02]);
    /// The testnet.
    pub const TESTNET: Magic = Magic([0xa7, 0x23, 0xe1, 0x6c]);
    /// The regtest.
    pub const REGTEST: Magic = Magic([0x81, 0x1d, 0x21, 0xf6]);
}

/// The block heights at which network upgrades activate.
/// Juno Cash: All pre-NU5 upgrades are active from block 1 (ALWAYS_ACTIVE).
/// NU5 activates at height 1, NU6 at height 2, NU6.1 at height 3.
pub mod activation_heights {
    /// Network upgrade activation heights for Testnet (Juno Cash testnet).
    /// All upgrades are ALWAYS_ACTIVE from genesis on testnet.
    pub mod testnet {
        use crate::block::Height;

        /// The block height at which `BeforeOverwinter` activates on Testnet.
        pub const BEFORE_OVERWINTER: Height = Height(1);
        /// The block height at which `Overwinter` activates on Testnet (ALWAYS_ACTIVE).
        pub const OVERWINTER: Height = Height(1);
        /// The block height at which `Sapling` activates on Testnet (ALWAYS_ACTIVE).
        pub const SAPLING: Height = Height(1);
        /// The block height at which `Blossom` activates on Testnet (ALWAYS_ACTIVE).
        pub const BLOSSOM: Height = Height(1);
        /// The block height at which `Heartwood` activates on Testnet (ALWAYS_ACTIVE).
        pub const HEARTWOOD: Height = Height(1);
        /// The block height at which `Canopy` activates on Testnet (ALWAYS_ACTIVE).
        pub const CANOPY: Height = Height(1);
        /// The block height at which `NU5` activates on Testnet (ALWAYS_ACTIVE).
        pub const NU5: Height = Height(1);
        /// The block height at which `NU6` activates on Testnet (ALWAYS_ACTIVE).
        pub const NU6: Height = Height(1);
        /// The block height at which `NU6.1` activates on Testnet (ALWAYS_ACTIVE).
        pub const NU6_1: Height = Height(1);
    }

    /// Network upgrade activation heights for Mainnet (Juno Cash mainnet).
    pub mod mainnet {
        use crate::block::Height;

        /// The block height at which `BeforeOverwinter` activates on Mainnet.
        pub const BEFORE_OVERWINTER: Height = Height(1);
        /// The block height at which `Overwinter` activates on Mainnet (ALWAYS_ACTIVE).
        pub const OVERWINTER: Height = Height(1);
        /// The block height at which `Sapling` activates on Mainnet (ALWAYS_ACTIVE).
        pub const SAPLING: Height = Height(1);
        /// The block height at which `Blossom` activates on Mainnet (ALWAYS_ACTIVE).
        pub const BLOSSOM: Height = Height(1);
        /// The block height at which `Heartwood` activates on Mainnet (ALWAYS_ACTIVE).
        pub const HEARTWOOD: Height = Height(1);
        /// The block height at which `Canopy` activates on Mainnet (ALWAYS_ACTIVE).
        pub const CANOPY: Height = Height(1);
        /// The block height at which `NU5` activates on Mainnet.
        pub const NU5: Height = Height(1);
        /// The block height at which `NU6` activates on Mainnet.
        pub const NU6: Height = Height(2);
        /// The block height at which `NU6.1` activates on Mainnet.
        pub const NU6_1: Height = Height(3);
    }
}
