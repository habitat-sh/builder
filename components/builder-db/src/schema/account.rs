table! {
    accounts (id) {
        id -> BigInt,
        name -> Text,
        email -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
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
