table! {
    origin_public_keys(id) {
        id -> BigInt,
        owner_id -> BigInt,
        name -> Text,
        revision -> Text,
        full_name -> Text,
        body -> Binary,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        origin -> Text,
    }
}

table! {
    origin_secret_keys(id) {
        id -> BigInt,
        owner_id -> BigInt,
        name -> Text,
        revision -> Text,
        full_name -> Text,
        body -> Binary,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        origin -> Text,
        encryption_key_rev -> Nullable<Text>,
    }
}

table! {
    origin_public_encryption_keys(id) {
        id -> BigInt,
        owner_id -> BigInt,
        name -> Text,
        revision -> Text,
        full_name -> Text,
        body -> Binary,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        origin -> Text,
    }
}

table! {
    origin_private_encryption_keys(id) {
        id -> BigInt,
        owner_id -> BigInt,
        name -> Text,
        revision -> Text,
        full_name -> Text,
        body -> Binary,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        origin -> Text,
    }
}
