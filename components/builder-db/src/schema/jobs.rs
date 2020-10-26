table! {
    use diesel::sql_types::{Bool, Array, Integer, BigInt, Text, Nullable, Timestamptz};

    jobs (id) {
        id -> BigInt,
        owner_id -> BigInt,
        job_state -> Text,
        project_id -> BigInt,
        project_name -> Text,
        project_owner_id -> BigInt,
        project_plan_path -> Text,
        vcs -> Text,
        vcs_arguments-> Array<Nullable<Text>>,
        net_error_code -> Nullable<Integer>,
        net_error_msg -> Nullable<Text>,
        scheduler_sync -> Bool,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        build_started_at -> Nullable<Timestamptz>,
        build_finished_at -> Nullable<Timestamptz>,
        package_ident -> Nullable<Text>,
        archived -> Bool,
        channel -> Nullable<Text>,
        sync_count -> Integer,
        worker -> Nullable<Text>,
        target -> Text,
    }
}

table! {
    use diesel::sql_types::{BigInt, Text, Nullable, Timestamptz};

    groups (id) {
        id -> BigInt,
        group_state -> Text,
        project_name -> Text,
        target -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

table! {
    use diesel::sql_types::{BigInt, Text, Nullable, Timestamptz};

    group_projects (id) {
        id -> BigInt,
        owner_id -> BigInt,
        project_name -> Text,
        project_ident -> Text,
        project_state -> Text,
        job_id -> BigInt,
        target -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

table! {
    use diesel::sql_types::{BigInt, Bool, Text, Nullable, Timestamptz};

    busy_workers (ident, job_id) {
        target -> Text,
        ident -> Text,
        job_id -> BigInt,
        quarantined -> Bool,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

table! {
    use diesel::sql_types::{BigInt, SmallInt, Text, Nullable, Timestamptz};
    audit_jobs (group_id) { // TODO THIS IS WRONG!!!! there isn't a primary key on this table.
        group_id -> BigInt,
        operation -> SmallInt,
        trigger -> SmallInt,
        requester_id -> BigInt,
        requester_name -> Text,
        created_at -> Nullable<Timestamptz>,
    }
}

table! {
    use diesel::sql_types::{Array, BigInt, Int4, Text, Nullable, Timestamptz};
    use crate::models::jobs::JobExecStateMapping;
    job_graph(id) {
        id -> BigInt,
        group_id -> BigInt,
        project_name -> Text,
        job_id -> Nullable<BigInt>,
        job_state -> JobExecStateMapping,
        manifest_ident -> Text,
        as_built_ident -> Nullable<Text>,
        dependencies -> Array<BigInt>,
        waiting_on_count -> Int4,
        target_platform -> Text, // Should be enum
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }

}
