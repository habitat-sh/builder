table! {
    origin_secrets (id) {
        id -> BigInt,
        origin_id -> BigInt,
        owner_id -> Nullable<BigInt>,
        name -> Text,
        value -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}
