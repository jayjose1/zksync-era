ALTER TABLE l1_batches
    DROP CONSTRAINT "l1_batch_number_fkey";
ALTER TABLE l1_batches
    ADD COLUMN timestamp BIGINT,
    ADD COLUMN fee_account_address BYTEA,
    ADD COLUMN l1_gas_price BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN l2_fair_gas_price BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN base_fee_per_gas NUMERIC(80) NOT NULL DEFAULT 1,
    ADD COLUMN protocol_version INTEGER,
    ADD COLUMN bootloader_code_hash BYTEA,
    ADD COLUMN default_aa_code_hash BYTEA;
ALTER TABLE l1_batches
    ADD CONSTRAINT "l1_batches_protocol_version_fkey"
    FOREIGN KEY (protocol_version) REFERENCES protocol_versions(id);

UPDATE l1_batches SET (
    timestamp,
    fee_account_address,
    l1_gas_price,
    l2_fair_gas_price,
    base_fee_per_gas,
    protocol_version,
    bootloader_code_hash,
    default_aa_code_hash
) = (
    SELECT timestamp,
        fee_account_address,
        l1_gas_price,
        l2_fair_gas_price,
        base_fee_per_gas,
        protocol_version,
        bootloader_code_hash,
        default_aa_code_hash
    FROM l1_batch_init_params
    WHERE l1_batch_init_params.number = l1_batches.number
);
-- Restore non-null constraints for the filled columns
ALTER TABLE l1_batches
    ALTER COLUMN timestamp SET NOT NULL,
    ALTER COLUMN fee_account_address SET NOT NULL;

DROP TABLE l1_batch_init_params;
