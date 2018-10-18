table! {
    origins (id) {
        id -> BigInt,
        name -> Text,
        owner_id -> BigInt,
        default_package_visibility -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

table! {
    origins_with_stats (id) {
        id -> BigInt,
        name -> Text,
        owner_id -> BigInt,
        default_package_visibility -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        package_count -> BigInt,
    }
}
