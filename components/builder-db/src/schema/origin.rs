table! {
    use models::package::PackageVisibilityMapping;
    use diesel::sql_types::{BigInt, Text, Nullable, Timestamptz};
    origins (id) {
        id -> BigInt,
        name -> Text,
        owner_id -> BigInt,
        default_package_visibility -> PackageVisibilityMapping,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

table! {
    use models::package::PackageVisibilityMapping;
    use diesel::sql_types::{BigInt, Text, Nullable};
    origins_with_secret_key (id) {
        id -> BigInt,
        name -> Text,
        owner_id -> BigInt,
        private_key_name -> Nullable<Text>,
        default_package_visibility -> PackageVisibilityMapping,
    }
}

table! {
    use models::package::PackageVisibilityMapping;
    use diesel::sql_types::{BigInt, Text, Nullable, Timestamptz};
    origins_with_stats (id) {
        id -> BigInt,
        name -> Text,
        owner_id -> BigInt,
        default_package_visibility -> PackageVisibilityMapping,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        package_count -> BigInt,
    }
}
