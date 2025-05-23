use near_chain::{ChainStore, ChainStoreAccess, Provenance};
use near_chain_configs::Genesis;
use near_epoch_manager::EpochManagerAdapter;
use near_o11y::testonly::init_test_logger;
use near_store::Store;
use near_store::test_utils::create_test_store;
use near_undo_block::undo_block;
use std::sync::Arc;

use crate::env::nightshade_setup::TestEnvNightshadeSetupExt;
use crate::env::test_env::TestEnv;

/// Setup environment with one Near client for testing.
fn setup_env(genesis: &Genesis, store: Store) -> (TestEnv, Arc<dyn EpochManagerAdapter>) {
    let env =
        TestEnv::builder(&genesis.config).stores(vec![store]).nightshade_runtimes(genesis).build();
    let epoch_manager = env.clients[0].epoch_manager.clone();
    (env, epoch_manager)
}

// Checks that Near client can successfully undo block on given height and then produce and process block normally after restart
fn test_undo_block(epoch_length: u64, stop_height: u64) {
    init_test_logger();

    let save_trie_changes = true;

    let mut genesis = Genesis::test(vec!["test0".parse().unwrap(), "test1".parse().unwrap()], 1);
    genesis.config.epoch_length = epoch_length;

    let store = create_test_store();
    let (mut env, epoch_manager) = setup_env(&genesis, store.clone());

    for i in 1..=stop_height {
        let block = env.clients[0].produce_block(i).unwrap().unwrap();
        env.process_block(0, block, Provenance::PRODUCED);
    }

    let mut chain_store = ChainStore::new(
        store.clone(),
        save_trie_changes,
        genesis.config.transaction_validity_period,
    );

    let current_head = chain_store.head().unwrap();
    let prev_block_hash = current_head.prev_block_hash;

    undo_block(&mut chain_store, &*epoch_manager).unwrap();

    // after undo, the current head should be the prev_block_hash
    assert_eq!(chain_store.head().unwrap().last_block_hash.as_bytes(), prev_block_hash.as_bytes());
    assert_eq!(chain_store.head().unwrap().height, stop_height - 1);

    // set up an environment again with the same store
    let (mut env, _) = setup_env(&genesis, store);
    // the new env should be able to produce block normally
    let block = env.clients[0].produce_block(stop_height).unwrap().unwrap();
    env.process_block(0, block, Provenance::PRODUCED);

    // after processing the new block, the head should now be at stop_height
    assert_eq!(chain_store.head().unwrap().height, stop_height);
}

#[test]
fn test_undo_block_middle_of_epoch() {
    test_undo_block(5, 3)
}

#[test]
fn test_undo_block_end_of_epoch() {
    test_undo_block(5, 5)
}

#[test]
fn test_undo_block_start_of_epoch() {
    test_undo_block(5, 6)
}
