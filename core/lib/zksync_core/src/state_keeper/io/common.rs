use std::time::Duration;

use multivm::{
    interface::{L1BatchEnv, L2BlockEnv, SystemEnv, TxExecutionMode},
    vm_latest::constants::BLOCK_GAS_LIMIT,
};
use zksync_contracts::BaseSystemContracts;
use zksync_dal::StorageProcessor;
use zksync_types::{
    block::L1BatchInitialParams, Address, L1BatchNumber, L2ChainId, MiniblockNumber,
    ProtocolVersionId, H256, U256, ZKPORTER_IS_AVAILABLE,
};
use zksync_utils::u256_to_h256;

use super::PendingBatchData;
use crate::state_keeper::extractors;

/// Returns the parameters required to initialize the VM for the next L1 batch.
#[allow(clippy::too_many_arguments)]
pub(crate) fn l1_batch_params(
    current_l1_batch_number: L1BatchNumber,
    fee_account: Address,
    l1_batch_timestamp: u64,
    previous_batch_hash: U256,
    l1_gas_price: u64,
    fair_l2_gas_price: u64,
    first_miniblock_number: MiniblockNumber,
    prev_miniblock_hash: H256,
    base_system_contracts: BaseSystemContracts,
    validation_computational_gas_limit: u32,
    protocol_version: ProtocolVersionId,
    virtual_blocks: u32,
    chain_id: L2ChainId,
) -> (SystemEnv, L1BatchEnv) {
    (
        SystemEnv {
            zk_porter_available: ZKPORTER_IS_AVAILABLE,
            version: protocol_version,
            base_system_smart_contracts: base_system_contracts,
            gas_limit: BLOCK_GAS_LIMIT,
            execution_mode: TxExecutionMode::VerifyExecute,
            default_validation_computational_gas_limit: validation_computational_gas_limit,
            chain_id,
        },
        L1BatchEnv {
            previous_batch_hash: Some(u256_to_h256(previous_batch_hash)),
            number: current_l1_batch_number,
            timestamp: l1_batch_timestamp,
            l1_gas_price,
            fair_l2_gas_price,
            fee_account,
            enforced_base_fee: None,
            first_l2_block: L2BlockEnv {
                number: first_miniblock_number.0,
                timestamp: l1_batch_timestamp,
                prev_block_hash: prev_miniblock_hash,
                max_virtual_blocks_to_create: virtual_blocks,
            },
        },
    )
}

/// Returns the amount of iterations `delay_interval` fits into `max_wait`, rounding up.
pub(crate) fn poll_iters(delay_interval: Duration, max_wait: Duration) -> usize {
    let max_wait_millis = max_wait.as_millis() as u64;
    let delay_interval_millis = delay_interval.as_millis() as u64;
    assert!(delay_interval_millis > 0, "delay interval must be positive");

    ((max_wait_millis + delay_interval_millis - 1) / delay_interval_millis).max(1) as usize
}

pub(crate) async fn load_l1_batch_params(
    storage: &mut StorageProcessor<'_>,
    current_l1_batch_number: L1BatchNumber,
    validation_computational_gas_limit: u32,
    chain_id: L2ChainId,
) -> Option<(SystemEnv, L1BatchEnv)> {
    let init_params = storage
        .blocks_dal()
        .get_l1_batch_initial_params(current_l1_batch_number)
        .await
        .unwrap()?;

    let (_, last_miniblock_number_in_prev_batch) = storage
        .blocks_dal()
        .get_miniblock_range_of_l1_batch(current_l1_batch_number - 1)
        .await
        .unwrap()
        .unwrap();
    let pending_miniblock_number = last_miniblock_number_in_prev_batch + 1;
    // If miniblock doesn't exist (for instance if it's pending), it means that there is no unsynced state (i.e. no transactions
    // were executed after the last sealed batch).
    let virtual_blocks = storage
        .blocks_dal()
        .get_virtual_blocks_for_miniblock(pending_miniblock_number)
        .await
        .unwrap()?;

    tracing::info!("Getting previous batch hash");
    let (previous_l1_batch_hash, prev_l1_batch_timestamp) =
        extractors::wait_for_prev_l1_batch_params(storage, current_l1_batch_number).await;
    assert!(
        prev_l1_batch_timestamp < init_params.timestamp,
        "Cannot seal L1 batch #{current_l1_batch_number}: Timestamp of previous L1 batch ({}) >= provisional L1 batch timestamp ({}), \
         meaning that L1 batch will be rejected by the bootloader",
        extractors::display_timestamp(prev_l1_batch_timestamp),
        extractors::display_timestamp(init_params.timestamp)
    );

    tracing::info!("Getting previous miniblock hash");
    let prev_miniblock_hash = storage
        .blocks_dal()
        .get_miniblock_header(pending_miniblock_number - 1)
        .await
        .unwrap()
        .unwrap()
        .hash;

    let base_system_contracts = storage
        .storage_dal()
        .get_base_system_contracts(
            init_params.base_system_contracts_hashes.bootloader,
            init_params.base_system_contracts_hashes.default_aa,
        )
        .await;

    tracing::info!("Previous l1_batch_hash: {}", previous_l1_batch_hash);
    Some(l1_batch_params(
        current_l1_batch_number,
        init_params.fee_account_address,
        init_params.timestamp,
        previous_l1_batch_hash,
        init_params.l1_gas_price,
        init_params.l2_fair_gas_price,
        pending_miniblock_number,
        prev_miniblock_hash,
        base_system_contracts,
        validation_computational_gas_limit,
        init_params
            .protocol_version
            .expect("`protocol_version` must be set for pending miniblock"),
        virtual_blocks,
        chain_id,
    ))
}

/// Loads the pending L1 batch data from the database.
pub(crate) async fn load_pending_batch(
    storage: &mut StorageProcessor<'_>,
    current_l1_batch_number: L1BatchNumber,
    validation_computational_gas_limit: u32,
    chain_id: L2ChainId,
) -> Option<PendingBatchData> {
    let (system_env, l1_batch_env) = load_l1_batch_params(
        storage,
        current_l1_batch_number,
        validation_computational_gas_limit,
        chain_id,
    )
    .await?;

    let pending_miniblocks = storage
        .transactions_dal()
        .get_miniblocks_to_reexecute()
        .await
        .unwrap();
    Some(PendingBatchData {
        l1_batch_env,
        system_env,
        pending_miniblocks,
    })
}

pub(crate) async fn save_l1_batch_init_params(
    storage: &mut StorageProcessor<'_>,
    system_env: &SystemEnv,
    l1_batch_env: &L1BatchEnv,
) {
    let params = L1BatchInitialParams {
        number: l1_batch_env.number,
        timestamp: l1_batch_env.timestamp,
        fee_account_address: l1_batch_env.fee_account,
        base_fee_per_gas: l1_batch_env.base_fee(),
        l1_gas_price: l1_batch_env.l1_gas_price,
        l2_fair_gas_price: l1_batch_env.fair_l2_gas_price,
        base_system_contracts_hashes: system_env.base_system_smart_contracts.hashes(),
        protocol_version: Some(system_env.version),
    };
    storage
        .blocks_dal()
        .insert_l1_batch_initial_params(&params)
        .await
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[rustfmt::skip] // One-line formatting looks better here.
    fn test_poll_iters() {
        assert_eq!(poll_iters(Duration::from_millis(100), Duration::from_millis(0)), 1);
        assert_eq!(poll_iters(Duration::from_millis(100), Duration::from_millis(100)), 1);
        assert_eq!(poll_iters(Duration::from_millis(100), Duration::from_millis(101)), 2);
        assert_eq!(poll_iters(Duration::from_millis(100), Duration::from_millis(200)), 2);
        assert_eq!(poll_iters(Duration::from_millis(100), Duration::from_millis(201)), 3);
    }
}
