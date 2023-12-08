use zksync_types::{api::en::SyncBlock, Address, MiniblockNumber, Transaction};

use crate::{
    instrument::InstrumentExt,
    metrics::MethodLatency,
    models::{storage_sync::StorageSyncBlock, storage_transaction::StorageTransaction},
    StorageProcessor,
};

/// DAL subset dedicated to the EN synchronization.
#[derive(Debug)]
pub struct SyncDal<'a, 'c> {
    pub storage: &'a mut StorageProcessor<'c>,
}

impl SyncDal<'_, '_> {
    pub async fn sync_block(
        &mut self,
        block_number: MiniblockNumber,
        current_operator_address: Address,
        include_transactions: bool,
    ) -> anyhow::Result<Option<SyncBlock>> {
        let latency = MethodLatency::new("sync_dal_sync_block");
        // FIXME: get rid of `COALESCE`
        let storage_block_details = sqlx::query_as!(
            StorageSyncBlock,
            "SELECT miniblocks.number, \
                COALESCE(miniblocks.l1_batch_number, (SELECT MAX(number) FROM l1_batch_init_params)) AS \"l1_batch_number!\", \
                (SELECT max(m2.number) FROM miniblocks m2 WHERE miniblocks.l1_batch_number = m2.l1_batch_number) as \"last_batch_miniblock?\", \
                miniblocks.timestamp, \
                miniblocks.hash as \"root_hash?\", \
                l1_batch_init_params.l1_gas_price, \
                l1_batch_init_params.l2_fair_gas_price, \
                l1_batch_init_params.bootloader_code_hash, \
                l1_batch_init_params.default_aa_code_hash, \
                miniblocks.virtual_blocks, \
                miniblocks.hash, \
                miniblocks.consensus, \
                l1_batch_init_params.protocol_version as \"protocol_version!\", \
                l1_batch_init_params.fee_account_address as \"fee_account_address?\" \
            FROM miniblocks \
            INNER JOIN l1_batch_init_params ON \
                l1_batch_init_params.number = COALESCE(miniblocks.l1_batch_number, (SELECT MAX(number) FROM l1_batch_init_params)) \
            WHERE miniblocks.number = $1",
            block_number.0 as i64
        )
        .instrument("sync_dal_sync_block.block")
        .with_arg("block_number", &block_number)
        .fetch_optional(self.storage.conn())
        .await?;

        let res = if let Some(storage_block_details) = storage_block_details {
            let transactions = if include_transactions {
                let block_transactions = sqlx::query_as!(
                    StorageTransaction,
                    r#"SELECT * FROM transactions WHERE miniblock_number = $1 ORDER BY index_in_block"#,
                    block_number.0 as i64
                )
                .instrument("sync_dal_sync_block.transactions")
                .with_arg("block_number", &block_number)
                .fetch_all(self.storage.conn())
                .await?
                .into_iter()
                .map(Transaction::from)
                .collect();
                Some(block_transactions)
            } else {
                None
            };
            Some(storage_block_details.into_sync_block(current_operator_address, transactions)?)
        } else {
            None
        };

        drop(latency);
        Ok(res)
    }
}
