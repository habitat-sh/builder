use diesel::sql_types::*;

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

sql_function! {
    fn insert_origin_invitation_v1(oi_origin_id: BigInt, oi_origin_name: Text, oi_account_id: BigInt, oi_account_name: Text, oi_owner_id: BigInt) -> (BigInt, Nullable<BigInt>, Nullable<Text>, Nullable<BigInt>, Nullable<Text>, Nullable<BigInt>, Nullable<Bool>, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn get_origin_invitation_v1(oi_invitation_id: BigInt) -> (BigInt, Nullable<BigInt>, Nullable<Text>, Nullable<BigInt>, Nullable<Text>, Nullable<BigInt>, Nullable<Bool>, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn get_origin_invitations_for_account_v1(oi_account_id: BigInt) -> (BigInt, Nullable<BigInt>, Nullable<Text>, Nullable<BigInt>, Nullable<Text>, Nullable<BigInt>, Nullable<Bool>, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn get_origin_invitations_for_origin_v1(oi_origin_id: BigInt) -> (BigInt, Nullable<BigInt>, Nullable<Text>, Nullable<BigInt>, Nullable<Text>, Nullable<BigInt>, Nullable<Bool>, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn validate_origin_invitation_v1(oi_invite_id: BigInt, oi_account_id: BigInt) -> ()

}

sql_function! {
    fn search_all_origin_packages_dynamic_v7(op_query: Text, op_my_origins: Text) -> Text
}
