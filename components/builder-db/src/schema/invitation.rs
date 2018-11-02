table! {
    origin_invitations {
        id -> BigInt,
        origin -> Text,
        account_id -> BigInt,
        account_name -> Text,
        owner_id -> BigInt,
        ignored -> Bool,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}
