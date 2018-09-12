use diesel::sql_types::*;

table! {
    origin_channels (id) {
        id -> BigInt,
        origin_id -> BigInt,
        owner_id -> BigInt,
        name -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

table! {
    origin_channel_packages (channel_id, package_id) {
        channel_id -> BigInt,
        package_id -> BigInt,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

sql_function!{
    fn get_origin_channels_for_origin_v2(origin_id: BigInt, include_sandbox_channels: Bool)
        -> (BigInt, BigInt, BigInt, Text, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function!{
    fn get_origin_channel_v1(origin_name: Text, name: Text)
        -> (BigInt, BigInt, BigInt, Text, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function!{
    fn insert_origin_channel_v1(origin_id: BigInt, owner_id: BigInt, name: Text)
        -> (BigInt, BigInt, BigInt, Text, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function!{
    fn delete_origin_channel_v1(channel_id: BigInt) -> ()
}
