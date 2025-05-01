table! {
    use crate::models::package::PackageVisibilityMapping;
    use diesel::sql_types::{BigInt, Bool, Nullable, Text, Timestamptz};

    origin_package_settings {
        id -> BigInt,
        origin -> Text,
        name -> Text,
        visibility -> PackageVisibilityMapping,
        owner_id -> BigInt,
        hidden -> Bool,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}
