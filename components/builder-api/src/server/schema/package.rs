use diesel::sql_types::*;

table! {
    origin_packages (id) {
        id -> BigInt,
        origin_id -> BigInt,
        owner_id -> BigInt,
        name -> Text,
        ident -> Text,
        checksum -> Text,
        manifest -> Text,
        config -> Text,
        target -> Text,
        deps -> Text,
        tdeps -> Text,
        exposes -> Text,
        visibility -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

sql_function! {
    fn get_all_origin_packages_for_ident_v1(op_ident: Text) -> (BigInt, BigInt, BigInt, Text, Text, Text, Text, Text, Text, Text, Text, Text, Text, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn get_all_origin_packages_for_ident_v1(op_ident: Text) -> (BigInt, BigInt, BigInt, Text, Text, Text, Text, Text, Text, Text, Text, Text, Text, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn get_origin_channel_package_latest_v5(op_origin: Text, op_channel: Text, op_ident: Text, op_target: Text, op_visibilities: Text) -> (BigInt, BigInt, BigInt, Text, Text, Text, Text, Text, Text, Text, Text, Text, Text, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn get_origin_channel_package_v4(op_origin: Text, op_channel: Text, op_ident: Text, op_visibilities: Text) -> (BigInt, BigInt, BigInt, Text, Text, Text, Text, Text, Text, Text, Text, Text, Text, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn get_origin_channel_package_v4(op_origin: Text, op_channel: Text, op_ident: Text, op_visibilities: Text) -> (BigInt, BigInt, BigInt, Text, Text, Text, Text, Text, Text, Text, Text, Text, Text, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn get_origin_package_v4(op_ident: Text, op_visibilities: Text) -> (BigInt, BigInt, BigInt, Text, Text, Text, Text, Text, Text, Text, Text, Text, Text, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn insert_origin_package_v3(op_origin_id: BigInt, op_owner_id: BigInt, op_name: Text, op_ident: Text, op_checksum: Text, op_manifest: Text, op_config: Text, op_target: Text, op_deps: Text, op_tdeps: Text, op_exposes: Text, op_visibility: Text) -> (BigInt, BigInt, BigInt, Text, Text, Text, Text, Text, Text, Text, Text, Text, Text, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn update_origin_package_v1(op_id: BigInt, op_owner_id: BigInt, op_name: Text, op_ident: Text, op_checksum: Text, op_manifest: Text, op_config: Text, op_target: Text, op_deps: Text, op_tdeps: Text, op_exposes: Text, op_visibility: Text) -> ()
}

sql_function! {
    fn update_package_visibility_in_bulk_v1(op_visibility: Text, op_ids: BigInt[]) -> ()
}

sql_function! {
    fn promote_origin_package_group_v1(opp_channel_id: BigInt, opp_package_ids: BigInt[]) -> ()
}

sql_function! {
    fn promote_origin_package_v1(opp_channel_id: BigInt, opp_package_id: BigInt) -> ()
}

sql_function! {
    fn set_packages_sync_v1(in_package_id: BigInt) -> ()
}

sql_function! {
    fn sync_packages_v2() -> (BigInt, BigInt, Text, Text, Text)
}

sql_function! {
    fn search_all_origin_packages_v6(op_query: Text, op_my_origins: Text) -> Text
}

sql_function! {
    fn search_all_origin_packages_dynamic_v7(op_query: Text, op_my_origins: Text) -> Text
}
