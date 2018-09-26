table! {
    origin_projects (id) {
        id -> BigInt,
        origin_id -> Nullable<BigInt>,
        origin_name -> Nullable<Text>,
        package_name -> Nullable<Text>,
        name -> Nullable<Text>,
        plan_path -> Nullable<Text>,
        owner_id -> Nullable<BigInt>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        visibility -> Text,
    }
}
