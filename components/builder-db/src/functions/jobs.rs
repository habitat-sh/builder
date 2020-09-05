use diesel::sql_types::{BigInt,
                        Integer};

use crate::models::jobs;

sql_function! {
  fn job_graph_mark_complete(in_id: BigInt) -> Integer
}
