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
    #[diesel(sql_type = Text)]
    pub short_id: String, // "origin/name"
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ReverseDependencies {
    pub origin: String,
    pub name:   String,
    pub rdeps:  Vec<String>,
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) async fn get_rdeps(conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
                              origin: &str,
                              name: &str,
                              target: &str)
                              -> Result<ReverseDependencies> {
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

/// Finds reverse dependencies for one specific package release (identified by
/// origin/name/version/release), rather than for the package name as a whole. This is used
/// prior to deleting a specific release, so that the release is only blocked from deletion
/// when another package actually depends on that exact identifier - not merely on some other
/// version/release of the same package name.
#[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
pub(crate) async fn get_rdeps_for_ident(conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
                                        origin: &str,
                                        name: &str,
                                        version: &str,
                                        release: &str,
                                        target: &str)
                                        -> Result<ReverseDependencies> {
    let sql_stmt = r###"
        select distinct op2.origin||'/'||op2.name as short_id
          from origin_packages as op1,
               origin_packages as op2
         where op1.origin = $1
           and op1.name = $2
           and op1.version = $3
           and op1.release = $4
           and op1.target = $5
           and op2.tdeps @> (ARRAY[op1.ident])
         order by short_id"###;

    let query = sql_query(sql_stmt).bind::<Text, _>(&origin)
                                   .bind::<Text, _>(&name)
                                   .bind::<Text, _>(&version)
                                   .bind::<Text, _>(&release)
                                   .bind::<Text, _>(&target);

    debug!("debug_query {}", debug_query::<Pg, _>(&query));

    let rdeps = query.load::<Dependent>(conn).map_err(Error::DieselError)?;

    let reverse_dependencies =
        ReverseDependencies { origin: origin.to_string(),
                              name:   name.to_string(),
                              rdeps:  rdeps.iter().map(|d| d.short_id.clone()).collect(), };
    debug!("reverse_dependencies (exact ident): {:?} ", reverse_dependencies);
    Ok(reverse_dependencies)
}
