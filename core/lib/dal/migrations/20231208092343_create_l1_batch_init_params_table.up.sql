CREATE TABLE IF NOT EXISTS l1_batch_init_params(
    number BIGINT NOT NULL PRIMARY KEY,
    timestamp BIGINT NOT NULL,
    fee_account_address BYTEA NOT NULL,
    l1_gas_price BIGINT NOT NULL DEFAULT 0,
    l2_fair_gas_price BIGINT NOT NULL DEFAULT 0,
    base_fee_per_gas NUMERIC(80) NOT NULL DEFAULT 1,
    protocol_version INTEGER,
    bootloader_code_hash BYTEA,
    default_aa_code_hash BYTEA,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);
ALTER TABLE l1_batch_init_params
    ADD CONSTRAINT "l1_batch_init_params_protocol_version_fkey"
    FOREIGN KEY (protocol_version) REFERENCES protocol_versions(id);

INSERT INTO l1_batch_init_params(
    number,
    timestamp,
    fee_account_address,
    l1_gas_price,
    l2_fair_gas_price,
    base_fee_per_gas,
    protocol_version,
    bootloader_code_hash,
    default_aa_code_hash,
    created_at,
    updated_at
) SELECT number,
    timestamp,
    fee_account_address,
    l1_gas_price,
    l2_fair_gas_price,
    base_fee_per_gas,
    protocol_version,
    bootloader_code_hash,
    default_aa_code_hash,
    NOW(),
    NOW()
FROM l1_batches;

ALTER TABLE l1_batches
    DROP CONSTRAINT "l1_batches_protocol_version_fkey";
ALTER TABLE l1_batches
    DROP COLUMN timestamp,
    DROP COLUMN fee_account_address,
    DROP COLUMN l1_gas_price,
    DROP COLUMN l2_fair_gas_price,
    DROP COLUMN base_fee_per_gas,
    DROP COLUMN protocol_version,
    DROP COLUMN bootloader_code_hash,
    DROP COLUMN default_aa_code_hash;
ALTER TABLE l1_batches
    ADD CONSTRAINT "l1_batch_number_fkey"
    FOREIGN KEY (number) REFERENCES l1_batch_init_params(number);
