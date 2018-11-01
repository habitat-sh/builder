table! {
    use models::channel::{PackageChannelOperationMapping, PackageChannelTriggerMapping};
    use diesel::sql_types::{BigInt, Text, Nullable, Timestamptz};
    audit_package (origin_id, package_id, channel_id) {
        origin_id -> BigInt,
        package_id -> BigInt,
        channel_id -> BigInt,
        operation -> PackageChannelOperationMapping,
        trigger -> PackageChannelTriggerMapping,
        requester_id -> BigInt,
        requester_name -> Text,
        created_at -> Nullable<Timestamptz>,
    }
}

table! {
    use models::channel::{PackageChannelOperationMapping, PackageChannelTriggerMapping};
    use diesel::sql_types::{BigInt, Array, Text, Nullable, Timestamptz};
    audit_package_group (origin_id, channel_id) {
        origin_id -> BigInt,
        channel_id -> BigInt,
        package_ids -> Array<BigInt>,
        operation -> PackageChannelOperationMapping,
        trigger -> PackageChannelTriggerMapping,
        requester_id -> BigInt,
        requester_name -> Text,
        group_id -> BigInt,
        created_at -> Nullable<Timestamptz>,
    }
}
