table! {
    origin_integrations (id) {
        id -> BigInt,
        origin -> Text,
        integration -> Text,
        name -> Text,
        body -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}
