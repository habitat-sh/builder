use diesel::sql_types::*;

table! {
    origins (id) {
        id -> BigInt,
        name -> Text,
        owner_id -> BigInt,
        default_package_visibility -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

sql_function! {
    fn insert_origin_v2(origin_name: Text, origin_owner_id: BigInt, origin_owner_name: Text, origin_default_package_visibility: Text) -> (BigInt, Text, BigInt, Text, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn my_origins_v2(om_account_id: BigInt) -> (BigInt, Text, BigInt, Text, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn list_origin_members_v1(om_origin_id: BigInt) -> Text
}

sql_function! {
    fn list_origin_by_account_id_v1(o_account_id: BigInt) -> Text
}

sql_function! {
    fn update_origin_v1(origin_id: BigInt, op_default_package_visibility: Text) -> ()
}

sql_function! {
    fn search_origin_packages_for_origin_distinct_v1(op_origin: Text, op_query: Text, op_limit: BigInt, op_offset: BigInt) -> (BigInt, Text)
}

sql_function! {
    fn search_origin_packages_for_origin_v4(op_origin: Text, op_query: Text, op_limit: BigInt, op_offset: BigInt, op_my_origins: Text) -> (BigInt, Text)
}

sql_function! {
    fn insert_origin_member_v1(om_origin_id: BigInt, om_origin_name: Text, om_account_id: BigInt, om_account_name: Text) -> ()
}
