table! {
    origin_project_integrations (id) {
        id -> BigInt,
        origin -> Text,
        name -> Text,
        integration -> Text,
        integration_name -> Text,
        body -> Text,
        project_id -> BigInt,
        integration_id -> BigInt,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

// sql_function!{
//     fn upsert_origin_project_integration_v3($1, $2, $3, $4) -> ()
// }

// sql_function!{
//     fn delete_origin_project_integration_v1($1, $2, $3) -> ()
// }

// sql_function!{
//     fn get_origin_project_integrations_v2($1, $2, $3) -> ()
// }

// sql_function!{
//     fn get_origin_project_integrations_for_project_v2($1, $2) -> ()
// }
