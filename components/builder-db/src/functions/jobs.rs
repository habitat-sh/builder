use diesel::sql_types::{Array,
                        BigInt,
                        Bool,
                        Integer,
                        Nullable,
                        Record,
                        Text,
                        Timestamptz};

use crate::models::jobs;
sql_function! {
    fn get_queued_group_v1(pname: Text) -> Array<jobs::Group>;
}

sql_function! {
    fn insert_job_v3(p_owner: BigInt, p_project_id: BigInt, p_project_name: Text, p_project_owner_id: BigInt, p_project_plan_path: Text, p_vcs: Text, p_vcs_arguments: Array<Text>, p_channel: Text, p_target: Text) -> Array<jobs::Job>;
}

sql_function! {
    fn next_pending_job_v2(p_worker: Text, p_target: Text) -> Array<Record<(BigInt, BigInt, Text,
BigInt, Text, BigInt, Text, Text, Array<Nullable<Text>>, Nullable<Integer>, Nullable<Text>, Bool,
Nullable<Timestamptz>, Nullable<Timestamptz>, Nullable<Timestamptz>, Nullable<Timestamptz>,
Nullable<Text>, Bool, Nullable<Text>, Integer, Nullable<Text>, Text)>>;
}
