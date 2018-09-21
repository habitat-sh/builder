se diesel::sql_types::*;

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

sql_function!{
    upsert_origin_project_integration_v3(in_origin: Text, in_name: Text, in_integration: Text, in_body: Text) -> (BigInt, Text, Text, Text, Text, Text, BigInt, BigInt, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function!{
    fn delete_origin_project_integration_v1(p_origin: Text, p_package: Text, p_integration: Text) -> ()
}

sql_function!{
    fn get_origin_project_integrations_v2(in_origin: Text, in_name: Text) -> (BigInt, Text, Text, Text, Text, Text, BigInt, BigInt, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function!{
    fn get_origin_project_integrations_for_project_v2(p_origin: Text, p_package: Text, p_integration: Text) -> (BigInt, Text, Text, Text, Text, Text, BigInt, BigInt, Nullable<Timestamptz>, Nullable<Timestamptz>)
}
