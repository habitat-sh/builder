use diesel::sql_types::{Array,
                        BigInt,
                        Integer,
                        Text};

use crate::models::jobs;

sql_function! {
    fn cancel_group_v1(in_gid: BigInt);
}

sql_function! {
    fn insert_group_v3(root_project: Text, project_names: Array<Text>, project_idents: Array<Text>, p_target: Text)
    -> jobs::GroupRecord;
}

sql_function! {
    fn pending_groups_v1(count: Integer) -> Array<jobs::GroupRecord>;
}

sql_function! {
    fn next_pending_job_v2(p_worker: Text, p_target: Text) -> jobs::JobRecord;
}
