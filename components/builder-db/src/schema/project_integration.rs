table! {
    origin_project_integrations (id) {
        id -> BigInt,
        origin -> Text,
        name -> Text,
        integration -> Text,
        integration_name -> Text,
        body -> Text,
        project_id -> BigInt,
        integration_id -> BigInt,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}
