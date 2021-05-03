use diesel::sql_types::{Array,
                        BigInt,
                        Integer,
                        Text};

use crate::models::jobs;

// Intended mostly for diagnostics and tests
sql_function! {
  // You would think this function returns an array of BigInt, but providing a scalar, and using get_results works.
  fn t_rdeps_for_id(in_id: BigInt) -> BigInt
}

sql_function! {
  fn t_deps_for_id(in_id: BigInt) -> BigInt
}

sql_function! {
  fn t_deps_for_id_group(in_id: BigInt, in_group: BigInt) -> BigInt
}

no_arg_sql_function!(job_graph_fixup_waiting_on_count, Integer);

sql_function! {
  fn job_graph_mark_complete(in_id: BigInt, as_built: Text) -> Integer
}

sql_function! {
  fn job_graph_mark_failed(in_id: BigInt) -> Integer
}

sql_function! {
  fn next_pending_job_v2(p_worker: Text, p_target: Text) -> jobs::JobRecord;
}

sql_function! {
  fn insert_group_v3(root_project: Text, project_names: Array<Text>, project_idents: Array<Text>, p_target: Text)
  -> jobs::GroupRecord;
}

sql_function! {
  fn pending_groups_v1(count: Integer) -> Array<jobs::GroupRecord>;
}
