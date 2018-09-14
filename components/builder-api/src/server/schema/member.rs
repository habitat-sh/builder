table! {
    origin_members (origin_id, account_id) {
        origin_id -> BigInt,
        origin_name -> Nullable<Text>,
        account_id -> BigInt,
        account_name -> Nullable<Text>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}
