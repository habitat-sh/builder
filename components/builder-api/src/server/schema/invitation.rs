table! {
    origin_invitations {
        id -> BigInt,
        origin_id -> BigInt,
        origin_name -> Text,
        account_id -> BigInt,
        account_name -> Text,
        owner_id -> BigInt,
        ignored -> Bool,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}
