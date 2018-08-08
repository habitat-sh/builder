CREATE OR REPLACE FUNCTION insert_origin_package_v4 (
  op_origin_id bigint,
  op_owner_id bigint,
  op_name text,
  op_ident text,
  op_checksum text,
  op_manifest text,
  op_config text,
  op_target text,
  op_deps text,
  op_tdeps text,
  op_exposes text,
  op_visibility text
) RETURNS SETOF origin_packages AS $$
    DECLARE
      inserted_package origin_packages;
      channel_id bigint;
    BEGIN
        INSERT INTO origin_packages (origin_id, owner_id, name, ident, checksum, manifest, config, target, deps, tdeps, exposes, visibility)
              VALUES (op_origin_id, op_owner_id, op_name, op_ident, op_checksum, op_manifest, op_config, op_target, op_deps, op_tdeps, op_exposes, op_visibility)
              ON CONFLICT ON CONSTRAINT origin_packages_ident_key DO
                UPDATE set checksum=op_checksum
              RETURNING * into inserted_package;

        SELECT id FROM origin_channels WHERE origin_id = op_origin_id AND name = 'unstable' INTO channel_id;
        PERFORM promote_origin_package_v1(channel_id, inserted_package.id);

        RETURN NEXT inserted_package;
        RETURN;
    END
$$ LANGUAGE plpgsql VOLATILE;