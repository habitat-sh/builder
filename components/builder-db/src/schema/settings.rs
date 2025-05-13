table! {
    use crate::schema::sql_types::origin_package_visibility;
    use diesel::sql_types::{BigInt, Bool, Nullable, Text, Timestamptz};

    origin_package_settings {
        id -> BigInt,
        origin -> Text,
        name -> Text,
        visibility -> origin_package_visibility,
        owner_id -> BigInt,
        hidden -> Bool,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}
