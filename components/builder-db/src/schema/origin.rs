table! {
    use crate::models::package::PackageVisibilityMapping;
    use diesel::sql_types::{BigInt, Text, Nullable, Timestamptz};
    origins (name) {
        owner_id -> BigInt,
        name -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        default_package_visibility -> PackageVisibilityMapping,
    }
}

table! {
    use crate::models::package::PackageVisibilityMapping;
    use diesel::sql_types::{BigInt, Text, Nullable};
    origins_with_secret_key (name) {
        owner_id -> BigInt,
        name -> Text,
        private_key_name -> Nullable<Text>,
        default_package_visibility -> PackageVisibilityMapping,
    }
}

table! {
    use crate::models::package::PackageVisibilityMapping;
    use diesel::sql_types::{BigInt, Text, Nullable, Timestamptz};
    origins_with_stats (name) {
        owner_id -> BigInt,
        name -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        default_package_visibility -> PackageVisibilityMapping,
        package_count -> BigInt,
    }
}
