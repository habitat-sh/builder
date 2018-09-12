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

// sql_function!{
//     fn upsert_origin_integration_v1(TEXT, TEXT, TEXT, TEXT) -> (i64, Nullable<Text>, Nullable<Text>, Nullable<Text>, Nullable<Text>, Nullable<Timestamptz>, Nullable<Timestamptz>)
// }
// sql_function!{
//     fn get_origin_integrations_v1($1, $2) -> ()
// }
// sql_function!{
//     fn get_origin_integrations_for_origin_v1($1) -> ()
// }
// sql_function!{
//     fn delete_origin_integration_v1($1, $2, $3) -> ()
// }
