table! {
    use crate::models::channel::{PackageChannelOperationMapping, PackageChannelTriggerMapping};
    use diesel::sql_types::{BigInt, Text, Nullable, Timestamptz};
    audit_package (origin, package_ident, channel) {
        package_ident -> Text,
        channel -> Text,
        operation -> PackageChannelOperationMapping,
        trigger -> PackageChannelTriggerMapping,
        requester_id -> BigInt,
        requester_name -> Text,
        created_at -> Nullable<Timestamptz>,
        origin -> Text,
    }
}

table! {
    use crate::models::channel::{PackageChannelOperationMapping, PackageChannelTriggerMapping};
    use diesel::sql_types::{BigInt, Array, Text, Nullable, Timestamptz};
    audit_package_group (origin, channel) {
        channel -> Text,
        package_ids -> Array<BigInt>,
        operation -> PackageChannelOperationMapping,
        trigger -> PackageChannelTriggerMapping,
        requester_id -> BigInt,
        requester_name -> Text,
        group_id -> BigInt,
        created_at -> Nullable<Timestamptz>,
        origin -> Text,
    }
}

table! {
    use crate::models::origin::OriginOperationMapping;
    use diesel::sql_types::{BigInt, Text, Nullable, Timestamptz};
    audit_origin (id) {
        id -> BigInt,
        operation -> OriginOperationMapping,
        origin -> Text,
        requester_id -> BigInt,
        requester_name -> Text,
        target_object -> Text,
        created_at -> Nullable<Timestamptz>,
    }
}

use super::{member::origin_members,
            origin::origins,
            package::origin_packages};

allow_tables_to_appear_in_same_query!(audit_package, origin_packages);
allow_tables_to_appear_in_same_query!(audit_package, origins);
allow_tables_to_appear_in_same_query!(audit_package, origin_members);
