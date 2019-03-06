table! {
    use crate::models::package::PackageVisibilityMapping;
    use diesel::sql_types::{Array, BigInt, Integer, Text, Nullable, Timestamptz};
    packages_with_channel_platform {
        id -> BigInt,
        owner_id -> BigInt,
        name -> Text,
        ident -> Text,
        ident_array -> Array<Text>,
        checksum -> Text,
        manifest -> Text,
        config -> Text,
        target -> Text,
        deps -> Array<Text>,
        tdeps -> Array<Text>,
        exposes -> Array<Integer>,
        visibility -> PackageVisibilityMapping,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        origin -> Text,
        channels -> Array<Text>,
        platforms -> Array<Text>,
    }
}

table! {
    use crate::models::package::PackageVisibilityMapping;
    use diesel::sql_types::{Array, BigInt, Integer, Text, Nullable, Timestamptz};
    use diesel_full_text_search::TsVector;
    origin_packages {
        id -> BigInt,
        owner_id -> BigInt,
        name -> Text,
        ident -> Text,
        ident_array -> Array<Text>,
        checksum -> Text,
        manifest -> Text,
        config -> Text,
        target -> Text,
        deps -> Array<Text>,
        tdeps -> Array<Text>,
        exposes -> Array<Integer>,
        visibility -> PackageVisibilityMapping,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        origin -> Text,
        ident_vector -> TsVector,
    }
}

table! {
    use crate::models::package::PackageVisibilityMapping;
    use diesel::sql_types::{Array, BigInt, Text};
    origin_package_versions (origin, name) {
        origin -> Text,
        name -> Text,
        version -> Text,
        release_count -> BigInt,
        latest -> Text,
        platforms -> Array<Text>,
        visibility -> PackageVisibilityMapping,
    }
}

use super::origin::{origins,
                    origins_with_stats};

joinable!(origin_packages -> origins (origin));
joinable!(origin_packages -> origins_with_stats (origin));
