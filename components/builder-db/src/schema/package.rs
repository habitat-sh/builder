table! {
    use models::package::PackageVisibilityMapping;
    use diesel::sql_types::{Array, BigInt, Integer, Text, Nullable, Timestamptz};
    use diesel_full_text_search::TsVector;
    origin_packages {
        id -> BigInt,
        origin_id -> BigInt,
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
        ident_vector -> TsVector,
    }
}

table! {
    origin_package_versions (origin, name) {
        origin -> Text,
        name -> Text,
        version -> Text,
        release_count -> BigInt,
        latest -> Text,
        platforms -> Array<Text>,
    }
}

use super::origin::origins;

joinable!(origin_packages -> origins (origin_id));
