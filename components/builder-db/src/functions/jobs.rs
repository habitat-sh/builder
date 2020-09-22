use diesel::sql_types::{BigInt,
                        Integer};

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
  fn job_graph_mark_complete(in_id: BigInt) -> Integer
}

sql_function! {
  fn job_graph_mark_failed(in_id: BigInt) -> Integer
}
