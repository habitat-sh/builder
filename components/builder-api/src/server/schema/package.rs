table! {
    use server::models::package::PackageVisibilityMapping;
    use diesel::sql_types::{Array, BigInt, SmallInt, Text, Nullable, Timestamptz};
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
        exposes -> Array<SmallInt>,
        visibility -> PackageVisibilityMapping,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}
