ALTER TABLE miniblocks
    ADD COLUMN base_fee_per_gas NUMERIC(80) NOT NULL DEFAULT 1,
    ADD COLUMN l1_gas_price BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN l2_fair_gas_price BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN bootloader_code_hash BYTEA,
    ADD COLUMN default_aa_code_hash BYTEA,
    ADD COLUMN protocol_version INTEGER;
ALTER TABLE miniblocks
    ADD CONSTRAINT "miniblocks_protocol_version_fkey"
    FOREIGN KEY (protocol_version) REFERENCES protocol_versions(id);

UPDATE miniblocks SET (
    base_fee_per_gas,
    l1_gas_price,
    l2_fair_gas_price,
    bootloader_code_hash,
    default_aa_code_hash,
    protocol_version
) = (
    SELECT base_fee_per_gas,
        l1_gas_price,
        l2_fair_gas_price,
        bootloader_code_hash,
        default_aa_code_hash,
        protocol_version
    FROM l1_batch_init_params
    WHERE l1_batch_init_params.number = COALESCE(miniblocks.l1_batch_number, (SELECT MAX(number) FROM l1_batch_init_params))
);
