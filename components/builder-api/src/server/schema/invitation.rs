table! {
    origin_invitations (id) {
        id -> BigInt,
        origin_id -> Nullable<BigInt>,
        origin_name -> Nullable<Text>,
        account_id -> Nullable<BigInt>,
        account_name -> Nullable<Text>,
        owner_id -> Nullable<BigInt>,
        ignored -> Nullable<Bool>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}
