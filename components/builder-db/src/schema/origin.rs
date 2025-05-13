table! {
    use crate::schema::sql_types::origin_package_visibility;
    use diesel::sql_types::{BigInt, Text, Nullable, Timestamptz};
    origins (name) {
        owner_id -> BigInt,
        name -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        default_package_visibility -> origin_package_visibility,
    }
}

table! {
    use crate::schema::sql_types::origin_package_visibility;
    use diesel::sql_types::{BigInt, Text, Nullable};
    origins_with_secret_key (name) {
        owner_id -> BigInt,
        name -> Text,
        private_key_name -> Nullable<Text>,
        default_package_visibility -> origin_package_visibility,
        owner_account -> Text,
    }
}

table! {
    use crate::schema::sql_types::origin_package_visibility;
    use diesel::sql_types::{BigInt, Text, Nullable, Timestamptz};
    origins_with_stats (name) {
        owner_id -> BigInt,
        name -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        default_package_visibility -> origin_package_visibility,
        package_count -> BigInt,
    }
}
