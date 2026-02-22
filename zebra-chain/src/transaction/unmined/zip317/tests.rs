//! ZIP-317 tests.

use super::{
    effective_marginal_fee, mempool_checks, Amount, Error, MARGINAL_FEE, SHIELDING_MARGINAL_FEE,
};

#[test]
fn zip317_unpaid_actions_err() {
    let check = mempool_checks(1, Amount::try_from(1).unwrap(), 1);

    assert!(check.is_err());
    assert_eq!(check.err(), Some(Error::UnpaidActions));
}

#[test]
fn zip317_minimum_rate_fee_err() {
    let check = mempool_checks(0, Amount::try_from(1).unwrap(), 1000);

    assert!(check.is_err());
    assert_eq!(check.err(), Some(Error::FeeBelowMinimumRate));
}

#[test]
fn zip317_mempool_checks_ok() {
    assert!(mempool_checks(0, Amount::try_from(100).unwrap(), 1000).is_ok())
}

#[test]
fn effective_marginal_fee_standard() {
    assert_eq!(effective_marginal_fee(false), MARGINAL_FEE);
    assert_eq!(effective_marginal_fee(false), 100_000);
}

#[test]
fn effective_marginal_fee_shielding() {
    assert_eq!(effective_marginal_fee(true), SHIELDING_MARGINAL_FEE);
    assert_eq!(effective_marginal_fee(true), 5_000);
}

#[test]
fn conventional_fee_for_tx_shielding_is_lower() {
    use super::conventional_fee_for_tx;
    use crate::serialization::ZcashDeserializeInto;

    // Use a real mainnet transaction from test vectors
    let block_bytes: &[u8] = zebra_test::vectors::BLOCK_MAINNET_1_BYTES.as_ref();
    let block: crate::block::Block = block_bytes.zcash_deserialize_into().unwrap();

    for tx in &block.transactions {
        let standard_fee = conventional_fee_for_tx(tx, false);
        let shielding_fee = conventional_fee_for_tx(tx, true);
        assert!(
            shielding_fee <= standard_fee,
            "shielding fee {} should be <= standard fee {}",
            i64::from(shielding_fee),
            i64::from(standard_fee),
        );
    }
}

#[test]
fn conventional_fee_for_tx_values_are_correct() {
    use super::{conventional_fee_for_tx, conventional_actions, GRACE_ACTIONS};
    use crate::serialization::ZcashDeserializeInto;

    let block_bytes: &[u8] = zebra_test::vectors::BLOCK_MAINNET_1_BYTES.as_ref();
    let block: crate::block::Block = block_bytes.zcash_deserialize_into().unwrap();

    for tx in &block.transactions {
        let actions = conventional_actions(tx);
        let expected_actions = std::cmp::max(GRACE_ACTIONS, actions);

        let standard_fee = conventional_fee_for_tx(tx, false);
        let shielding_fee = conventional_fee_for_tx(tx, true);

        assert_eq!(
            i64::from(standard_fee),
            (MARGINAL_FEE as i64) * i64::from(expected_actions),
        );
        assert_eq!(
            i64::from(shielding_fee),
            (SHIELDING_MARGINAL_FEE as i64) * i64::from(expected_actions),
        );
    }
}
