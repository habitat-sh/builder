table! {
    use server::models::channel::{PackageChannelOperationMapping, PackageChannelTriggerMapping};
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
