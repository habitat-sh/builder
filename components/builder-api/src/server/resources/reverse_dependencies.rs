use diesel::{debug_query,
             pg::Pg,
             r2d2::ConnectionManager,
             sql_query,
             sql_types::Text,
             PgConnection,
             QueryableByName,
             RunQueryDsl};

use crate::server::error::{Error,
                           Result};

use r2d2::PooledConnection;

#[derive(Clone, Debug, QueryableByName, Serialize, Deserialize)]
pub(crate) struct Dependent {
    #[sql_type = "Text"]
    pub short_id: String, // "origin/name"
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ReverseDependencies {
    pub origin: String,
    pub name:   String,
    pub rdeps:  Vec<String>,
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) async fn get_rdeps(conn: &PooledConnection<ConnectionManager<PgConnection>>,
                              origin: &str,
                              name: &str,
                              target: &str)
                              -> Result<ReverseDependencies> {
    trace!("builder_api::server::resources::reverse_dependencies::get_rdeps");

    let sql_stmt = r###"
        select * from (
            select distinct op3.origin||'/'||op3.name as short_id
              from origin_packages as op1,
           lateral (select op2.id, op2.origin, op2.name from origin_packages as op2 where op2.tdeps @> (ARRAY[op1.ident])) as op3
             where op1.origin = $1 and op1.name = $2 and op1.target = $3
                union distinct
            select distinct op3.origin||'/'||op3.name as short_id
              from origin_packages as op1,
           lateral (select op2.id, op2.origin, op2.name from origin_packages as op2 where op2.tdeps @> (ARRAY[op1.ident])) as op3
             where op1.origin = $1 and op1.name = $2 and op1.target = $3
        ) as ordered_rdeps order by short_id"###;

    let query = sql_query(sql_stmt).bind::<Text, _>(&origin)
                                   .bind::<Text, _>(&name)
                                   .bind::<Text, _>(&target);

    debug!("debug_query {}", debug_query::<Pg, _>(&query));

    let rdeps = query.load::<Dependent>(conn).map_err(Error::DieselError)?;

    let reverse_dependencies =
        ReverseDependencies { origin: origin.to_string(),
                              name:   name.to_string(),
                              rdeps:  rdeps.iter().map(|d| d.short_id.clone()).collect(), };
    debug!("reverse_dependencies: {:?} ", reverse_dependencies);
    Ok(reverse_dependencies)
}
