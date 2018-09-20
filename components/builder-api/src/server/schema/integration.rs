use diesel::sql_types::*;

table! {
    origin_integrations (id) {
        id -> BigInt,
        origin -> Nullable<Text>,
        integration -> Nullable<Text>,
        name -> Nullable<Text>,
        body -> Nullable<Text>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

sql_function!{
   fn upsert_origin_integration_v1(in_origin: Text, in_integrartion: Text, in_name: Text, in_body: Text) -> (BigInt, Nullable<Text>, Nullable<Text>, Nullable<Text>, Nullable<Text>, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function!{
    fn insert_origin_integration_v1(in_origin: Text, in_integration: Text, in_name: Text, in_body: Text) -> (BigInt, Nullable<Text>, Nullable<Text>, Nullable<Text>, Nullable<Text>, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function!{
    fn get_origin_integration_v1(in_origin: Text, in_integration: Text, in_name: Text) -> (BigInt, Nullable<Text>, Nullable<Text>, Nullable<Text>, Nullable<Text>, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function!{
    fn get_origin_integrations_v1(in_origin: Text, in_integration: Text) -> (i64, Nullable<Text>, Nullable<Text>, Nullable<Text>, Nullable<Text>, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function!{
    fn get_origin_integrations_for_origin_v1(in_origin: Text) -> (i64, Nullable<Text>, Nullable<Text>, Nullable<Text>, Nullable<Text>, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function!{
    fn delete_origin_integration_v1(in_origin: Text, in_integration: Text, in_name: Text) -> ()
}
