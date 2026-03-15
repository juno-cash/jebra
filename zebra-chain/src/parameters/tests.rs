//! Consensus parameter tests for Zebra.

#![allow(clippy::unwrap_in_result)]

use std::collections::HashSet;

use crate::block;

use super::*;

use Network::*;
use NetworkUpgrade::*;

/// Check that the activation list entries are consistent.
///
/// Juno Cash has multiple upgrades at the same height (ALWAYS_ACTIVE = 0),
/// so the BTreeMap has fewer entries than the source array. This test checks
/// the BTreeMap's own internal consistency.
#[test]
fn activation_bijective() {
    let _init_guard = zebra_test::init();

    let mainnet_activations = Mainnet.activation_list();
    // Each height maps to exactly one upgrade in the BTreeMap
    let mainnet_heights: HashSet<&block::Height> = mainnet_activations.keys().collect();
    let mainnet_nus: HashSet<&NetworkUpgrade> = mainnet_activations.values().collect();
    assert_eq!(mainnet_heights.len(), mainnet_nus.len());

    let testnet_activations = Network::new_default_testnet().activation_list();
    let testnet_heights: HashSet<&block::Height> = testnet_activations.keys().collect();
    let testnet_nus: HashSet<&NetworkUpgrade> = testnet_activations.values().collect();
    assert_eq!(testnet_heights.len(), testnet_nus.len());
}

#[test]
fn activation_extremes_mainnet() {
    let _init_guard = zebra_test::init();
    activation_extremes(Mainnet)
}

#[test]
fn activation_extremes_testnet() {
    let _init_guard = zebra_test::init();
    activation_extremes(Network::new_default_testnet())
}

/// Test the activation_list, activation_height, current, and next functions
/// for `network` with extreme values.
fn activation_extremes(network: Network) {
    let activation_list = network.activation_list();

    // Height 0 should always have an activation
    let height_0_nu = activation_list
        .get(&block::Height(0))
        .expect("height 0 should have a network upgrade");
    assert!(NetworkUpgrade::is_activation_height(
        &network,
        block::Height(0)
    ));

    let current_at_0 = NetworkUpgrade::current(&network, block::Height(0));
    assert_eq!(current_at_0, *height_0_nu);

    // We assume that the last upgrade we know about continues forever
    // (even if we suspect that won't be true)
    assert_ne!(
        NetworkUpgrade::current(&network, block::Height::MAX),
        Genesis
    );
    assert!(!NetworkUpgrade::is_activation_height(
        &network,
        block::Height::MAX
    ));
    assert_eq!(NetworkUpgrade::next(&network, block::Height::MAX), None);
}

#[test]
fn activation_consistent_mainnet() {
    let _init_guard = zebra_test::init();
    activation_consistent(Mainnet)
}

#[test]
fn activation_consistent_testnet() {
    let _init_guard = zebra_test::init();
    activation_consistent(Network::new_default_testnet())
}

/// Check that the `activation_height`, `is_activation_height`,
/// `current`, and `next` functions are consistent for `network`.
fn activation_consistent(network: Network) {
    let activation_list = network.activation_list();
    let network_upgrades: HashSet<&NetworkUpgrade> = activation_list.values().collect();

    for &network_upgrade in network_upgrades {
        let height = network_upgrade
            .activation_height(&network)
            .expect("activations must have a height");
        assert!(NetworkUpgrade::is_activation_height(&network, height));

        assert_eq!(NetworkUpgrade::current(&network, height), network_upgrade);
        // Network upgrades don't repeat
        assert_ne!(
            NetworkUpgrade::next(&network, height),
            Some(network_upgrade)
        );
        assert_ne!(
            NetworkUpgrade::next(&network, block::Height(height.0 + 1)),
            Some(network_upgrade)
        );
        assert_ne!(
            NetworkUpgrade::next(&network, block::Height::MAX),
            Some(network_upgrade)
        );
    }
}

/// Check that the network upgrades and branch ids are unique.
#[test]
fn branch_id_bijective() {
    let _init_guard = zebra_test::init();

    let branch_id_list = NetworkUpgrade::branch_id_list();
    let nus: HashSet<&NetworkUpgrade> = branch_id_list.keys().collect();
    assert_eq!(CONSENSUS_BRANCH_IDS.len(), nus.len());

    let branch_ids: HashSet<&ConsensusBranchId> = branch_id_list.values().collect();
    assert_eq!(CONSENSUS_BRANCH_IDS.len(), branch_ids.len());
}

#[test]
fn branch_id_extremes_mainnet() {
    let _init_guard = zebra_test::init();
    branch_id_extremes(Mainnet)
}

#[test]
fn branch_id_extremes_testnet() {
    let _init_guard = zebra_test::init();
    branch_id_extremes(Network::new_default_testnet())
}

/// Test the branch_id_list, branch_id, and current functions for `network` with
/// extreme values.
fn branch_id_extremes(network: Network) {
    // Juno Cash: All upgrades are active from genesis, so there's always a branch ID
    // at height 0 (unlike Zcash where Genesis/BeforeOverwinter have no branch ID).
    let current_at_0 = NetworkUpgrade::current(&network, block::Height(0));
    if current_at_0.branch_id().is_some() {
        assert!(ConsensusBranchId::current(&network, block::Height(0)).is_some());
    }

    // We assume that the last upgrade we know about continues forever
    // (even if we suspect that won't be true)
    assert_ne!(
        NetworkUpgrade::branch_id_list()
            .get(&NetworkUpgrade::current(&network, block::Height::MAX)),
        None
    );
    assert_ne!(
        ConsensusBranchId::current(&network, block::Height::MAX),
        None
    );
}

#[test]
fn branch_id_consistent_mainnet() {
    let _init_guard = zebra_test::init();
    branch_id_consistent(Mainnet)
}

#[test]
fn branch_id_consistent_testnet() {
    let _init_guard = zebra_test::init();
    branch_id_consistent(Network::new_default_testnet())
}

/// Check that the branch_id and current functions are consistent for `network`.
fn branch_id_consistent(network: Network) {
    let activation_list = network.activation_list();

    // Only check upgrades that are actually in the activation list (the BTreeMap).
    // When multiple upgrades share the same height, only the latest one is stored.
    for (&height, &network_upgrade) in &activation_list {
        if let Some(branch_id) = network_upgrade.branch_id() {
            assert_eq!(
                ConsensusBranchId::current(&network, height),
                Some(branch_id)
            );
        }
    }
}

// TODO: split this file in unit.rs and prop.rs
use hex::{FromHex, ToHex};
use proptest::prelude::*;

proptest! {
    #[test]
    fn branch_id_hex_roundtrip(nu in any::<NetworkUpgrade>()) {
        let _init_guard = zebra_test::init();

        if let Some(branch) = nu.branch_id() {
            let hex_branch: String = branch.encode_hex();
            let new_branch = ConsensusBranchId::from_hex(hex_branch.clone()).expect("hex branch_id should parse");
            prop_assert_eq!(branch, new_branch);
            prop_assert_eq!(hex_branch, new_branch.to_string());
        }
    }
}
