use diesel::sql_types::*;

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

sql_function! {
    fn check_account_in_origin_members_v1(om_origin_name: Text, om_account_id: BigInt) -> Bool
}

sql_function! {
    fn insert_origin_member_v1(om_origin_id: BigInt, om_origin_name Text, om_account_id: BigInt, om_account_name: Text) -> ()
}

sql_function! {
    fn delete_origin_member_v1(om_origin_id: BigInt, om_account_name: Text) -> Nullable<Text>
}

sql_function! {
    fn list_origin_by_account_id_v1(o_account_id: BigInt) -> Nullable<Text>
}

sql_function! {
    fn list_origin_members_v1(om_origin_id: BigInt) -> Nullable<Text>
}
