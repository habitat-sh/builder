table! {
    origin_channels (id) {
        id -> BigInt,
        owner_id -> BigInt,
        name -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        origin -> Text,
    }
}

table! {
    origin_channel_packages (channel_id, package_id) {
        channel_id -> BigInt,
        package_id -> BigInt,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

use super::origin::origins;
use super::package::origin_packages;

joinable!(origin_channel_packages -> origin_packages (package_id));
joinable!(origin_channel_packages -> origin_channels (channel_id));
joinable!(origin_channels -> origins (origin));

allow_tables_to_appear_in_same_query!(
    origin_channels,
    origin_channel_packages,
    origin_packages,
    origins
);
