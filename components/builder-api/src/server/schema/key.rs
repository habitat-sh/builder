use diesel::sql_types::*;

table! {
    origin_public_keys (id) {
        id -> BigInt,
        origin_id -> BigInt,
        owner_id -> BigInt,
        name -> Text,
        revision -> Text,
        full_name -> Text,
        body -> Vec<u8>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

table! {
    origin_secret_keys (id) {
        id -> BigInt,
        origin_id -> BigInt,
        owner_id -> BigInt,
        name -> Text,
        revision -> Text,
        full_name -> Text,
        body -> Vec<u8>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

table! {
    origin_public_encryption_keys () {
        id -> BigInt,
        origin_id -> BigInt,
        owner_id -> BigInt,
        name -> Text,
        revision -> Text,
        full_name -> Text,
        body -> Vec<u8>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

table! {
    origin_private_encryption_keys () {
        id -> BigInt,
        origin_id -> BigInt,
        owner_id -> BigInt,
        name -> Text,
        revision -> Text,
        full_name -> Text,
        body -> Vec<u8>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

sql_function! {
    fn get_origin_secret_key_v1(osk_name: Text) -> (BigInt, BigInt, BigInt, Text, Text, Txt, Vec<u8>, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn insert_origin_secret_key_v1(osk_origin_id: BigInt, osk_owner_id: BigInt, osk_name: Text, osk_revision: Text, osk_full_name: Text, osk_body: Vec<u8>) -> (BigInt, BigInt, BigInt, Text, Text, Txt, Vec<u8>, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn get_origin_public_key_latest_v1(opk_name: Text) -> (BigInt, BigInt, BigInt, Text, Text, Txt, Vec<u8>, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn get_origin_public_key_v1(opk_name: Text, opk_revision: Text) -> (BigInt, BigInt, BigInt, Text, Text, Txt, Vec<u8>, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn get_origin_public_keys_for_origin_v1(opk_origin_id: BigInt) -> (BigInt, BigInt, BigInt, Text, Text, Txt, Vec<u8>, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn get_origin_private_encryption_key_v1(opek_name: Text) -> (BigInt, BigInt, BigInt, Text, Text, Txt, Vec<u8>, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn get_origin_public_encryption_key_latest_v1(opek_name: Text) -> (BigInt, BigInt, BigInt, Text, Text, Txt, Vec<u8>, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn get_origin_public_encryption_keys_for_origin_v1(opek_origin_id: BigInt) -> (BigInt, BigInt, BigInt, Text, Text, Txt, Vec<u8>, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn insert_origin_public_key_v1(opk_origin_id: BigInt, opk_owner_id: BigInt, opk_name: Text, opk_revision: Text, opk_full_name: Text, opk_body: Vec<u8>) -> (BigInt, BigInt, BigInt, Text, Text, Txt, Vec<u8>, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn insert_origin_public_encryption_key_v1(opek_origin_id: BigInt, opek_owner_id: BigInt, opek_name: Text, opek_revision: Text, opek_full_name: Text, opek_body: bytea) -> (BigInt, BigInt, BigInt, Text, Text, Txt, Vec<u8>, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function! {
    fn insert_origin_private_encryption_key_v1(opek_origin_id: BigInt, opek_owner_id: BigInt, opek_name: Text, opek_revision: Text, opek_full_name: Text, opek_body: bytea) -> (BigInt, BigInt, BigInt, Text, Text, Txt, Vec<u8>, Nullable<Timestamptz>, Nullable<Timestamptz>)
}
