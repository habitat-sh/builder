table! {
    use diesel::sql_types::{Bool, BigInt, Text, Nullable, Timestamptz};
    use models::package::PackageVisibilityMapping;

    origin_projects (id) {
        id -> BigInt,
        origin_id -> BigInt,
        origin_name -> Text,
        package_name -> Text,
        name -> Text,
        plan_path -> Text,
        owner_id -> BigInt,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        visibility -> PackageVisibilityMapping,
        vcs_type -> Text,
        vcs_data -> Text,
        vcs_installation_id -> BigInt,
        auto_build -> Bool,
    }
}
