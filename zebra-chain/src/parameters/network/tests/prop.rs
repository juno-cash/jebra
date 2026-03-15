use proptest::prelude::*;

use super::super::Network;
use crate::{
    block::Height,
    parameters::{NetworkUpgrade, TESTNET_MAX_TIME_START_HEIGHT},
};

proptest! {
    /// Check that the mandatory checkpoint is at or before Canopy activation.
    #[test]
    fn mandatory_checkpoint_is_at_or_before_canopy(network in any::<Network>()) {
        let _init_guard = zebra_test::init();

        let canopy_activation = NetworkUpgrade::Canopy
            .activation_height(&network)
            .expect("Canopy activation height is set");

        // For Juno Cash where Canopy activates at genesis (Height(0)),
        // mandatory_checkpoint_height() returns Height(0).
        assert!(network.mandatory_checkpoint_height() <= canopy_activation);
    }
    #[test]
    /// Asserts that the activation height is correct for the block
    /// maximum time rule on Testnet is correct.
    fn max_block_times_correct_enforcement(height in any::<Height>()) {
        let _init_guard = zebra_test::init();

        assert!(Network::Mainnet.is_max_block_time_enforced(height));
        assert_eq!(Network::new_default_testnet().is_max_block_time_enforced(height), TESTNET_MAX_TIME_START_HEIGHT <= height);
    }
}
