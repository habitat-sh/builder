table! {
    origin_project_integrations (id) {
        id -> BigInt,
        origin -> Text,
        body -> Text,
        project_id -> BigInt,
        integration_id -> BigInt,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}
