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

sql_function!{
    fn update_origin_project_v4(project_id: BigInt, project_origin_id: BigInt, project_package_name: Text, project_plan_path: Text, project_vcs_type: Text, project_vcs_data: Text, project_owner_id: BigInt, project_vcs_installation_id: BigInt, project_visibility: Text, project_auto_build: Bool) -> ()
}

sql_function!{
   fn delete_origin_project_v1(project_name: Text) -> ()
}

sql_function!{
    fn get_origin_project_v1(project_name: Text) -> (BigInt, Nullable<BigInt>, Nullable<Text>, Nullable<Text>, Nullable<Text>, Nullable<Text>, Nullable<BigInt>, Nullable<Timestamptz>, Nullable<Timestamptz>, Text)
}

sql_function!{
    fn insert_origin_project_v5(project_origin_name: Text, project_package_name: Text, project_plan_path: Text, project_vcs_type: Text, project_vcs_data: Text, project_owner_id: BigInt, project_vcs_installation_id: BigInt, project_visibility: Text, project_auto_build: Bool) -> (BigInt, Nullable<BigInt>, Nullable<Text>, Nullable<Text>, Nullable<Text>, Nullable<Text>, Nullable<BigInt>, Nullable<Timestamptz>, Nullable<Timestamptz>, Text)
}

sql_function!{
    fn get_origin_project_list_v2(in_origin: Text) -> (BigInt, Nullable<BigInt>, Nullable<Text>, Nullable<Text>, Nullable<Text>, Nullable<Text>, Nullable<BigInt>, Nullable<Timestamptz>, Nullable<Timestamptz>, Text)
}
