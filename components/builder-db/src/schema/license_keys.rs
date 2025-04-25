table! {
    license_keys (id) {
        id -> BigInt,
        account_id -> BigInt,
        license_key -> Text,
        expiration_date -> Date,
        created_at -> Nullable<Timestamptz>,
    }
}
