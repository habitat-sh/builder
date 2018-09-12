table! {
    origin_projects (id) {
        id -> BigInt,
        origin_id -> Nullable<BigInt>,
        origin_name -> Nullable<Text>,
        package_name -> Nullable<Text>,
        name -> Nullable<Text>,
        plan_path -> Nullable<Text>,
        owner_id -> Nullable<BigInt>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        visibility -> Text,
    }
}

// sql_function!{
//     fn update_origin_project_v4($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) -> ()
// }

// sql_function!{
//     fn delete_origin_project_v1($1) -> ()
// }

// sql_function!{
//     fn get_origin_project_v1($1) -> ()
// }

// sql_function!{
//     fn insert_origin_project_v5($1, $2, $3, $4, $5, $6, $7, $8, $9) -> ()
// }

// sql_function!{
//     fn get_origin_project_list_v2($1) -> ()
// }
