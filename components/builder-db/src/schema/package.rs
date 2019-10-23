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
        build_deps -> Array<Text>,
        build_tdeps -> Array<Text>,
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
    use diesel::sql_types::{Array, BigInt, Integer, Text,  Nullable, Timestamptz};
    origin_packages_with_version_array {
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
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        visibility -> PackageVisibilityMapping,
        origin -> Text,
        build_deps -> Array<Text>,
        build_tdeps -> Array<Text>,
        version_array -> Array<Nullable<Text>>,
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
        build_deps -> Array<Text>,
        build_tdeps -> Array<Text>,
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

table! {
    use crate::models::package::OriginPackageSettings;
    use crate::models::package::PackageVisibilityMapping;
    use diesel::sql_types::{BigInt, Nullable, Text, Timestamptz};

    origin_package_settings {
        id -> BigInt,
        origin -> Text,
        name -> Text,
        visibility -> PackageVisibilityMapping,
        owner_id -> BigInt,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

use super::origin::{origins,
                    origins_with_stats};

joinable!(origin_packages -> origins (origin));
joinable!(origin_packages -> origins_with_stats (origin));
