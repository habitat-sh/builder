table! {
    accounts (id) {
        id -> BigInt,
        email -> Text,
        name -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        provider -> Nullable<Text>,
        extern_id -> Nullable<Text>,
    }
}

table! {
    account_tokens (id) {
        id -> BigInt,
        account_id -> BigInt,
        token -> Text,
        created_at -> Nullable<Timestamptz>,
    }
}
