use super::db_id_format;
use chrono::NaiveDateTime;
use diesel;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use diesel::{ExpressionMethods, NullableExpressionMethods, QueryDsl, RunQueryDsl, Table};

use crate::protocol::originsrv;
use crate::schema::integration::origin_integrations;
use crate::schema::project::origin_projects;
use crate::schema::project_integration::origin_project_integrations;

use crate::bldr_core::metrics::CounterMetric;
use crate::metrics::Counter;

#[derive(Debug, Serialize, Deserialize, QueryableByName, Queryable, Identifiable)]
#[table_name = "origin_project_integrations"]
pub struct ProjectIntegration {
    #[serde(with = "db_id_format")]
    pub id: i64,
    #[serde(with = "db_id_format")]
    pub project_id: i64,
    #[serde(with = "db_id_format")]
    pub integration_id: i64,
    pub origin: String,
    pub body: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

pub struct NewProjectIntegration<'a> {
    pub origin: &'a str,
    pub name: &'a str,
    pub integration: &'a str,
    pub body: &'a str,
}

impl ProjectIntegration {
    pub fn get(
        origin: &str,
        name: &str,
        integration: &str,
        conn: &PgConnection,
    ) -> QueryResult<ProjectIntegration> {
        Counter::DBCall.increment();
        origin_project_integrations::table
            .inner_join(origin_integrations::table)
            .inner_join(origin_projects::table)
            .select(origin_project_integrations::table::all_columns())
            .filter(origin_project_integrations::origin.eq(origin))
            .filter(origin_projects::package_name.eq(name))
            .filter(origin_integrations::name.eq(integration))
            .get_result(conn)
    }

    pub fn list(
        origin: &str,
        name: &str,
        conn: &PgConnection,
    ) -> QueryResult<Vec<ProjectIntegration>> {
        Counter::DBCall.increment();
        origin_project_integrations::table
            .inner_join(origin_projects::table)
            .select(origin_project_integrations::table::all_columns())
            .filter(origin_project_integrations::origin.eq(origin))
            .filter(origin_projects::package_name.eq(name))
            .get_results(conn)
    }

    pub fn delete(
        origin: &str,
        name: &str,
        integration: &str,
        conn: &PgConnection,
    ) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::delete(
            origin_project_integrations::table
                .filter(origin_project_integrations::origin.eq(origin))
                .filter(
                    origin_project_integrations::project_id
                        .nullable()
                        .eq(origin_projects::table
                            .select(origin_projects::id)
                            .filter(origin_projects::package_name.eq(name))
                            .single_value()),
                )
                .filter(
                    origin_project_integrations::integration_id.nullable().eq(
                        origin_integrations::table
                            .select(origin_integrations::id)
                            .filter(origin_integrations::name.eq(integration))
                            .single_value(),
                    ),
                ),
        )
        .execute(conn)
    }

    pub fn create(req: &NewProjectIntegration, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        // We currently support running only one publish step per build job. This
        // temporary fix ensures we store (and can retrieve) only one project integration.
        // Don't care what the result is here, it's just a precaution
        let _ = Self::delete(req.origin, req.name, req.integration, conn);

        let project_id = origin_projects::table
            .select(origin_projects::id)
            .filter(origin_projects::package_name.eq(req.name))
            .limit(1)
            .get_result::<i64>(conn)?;

        let integration_id = origin_integrations::table
            .select(origin_integrations::id)
            .filter(origin_integrations::name.eq(req.integration))
            .limit(1)
            .get_result::<i64>(conn)?;

        diesel::insert_into(origin_project_integrations::table)
            .values((
                origin_project_integrations::origin.eq(req.origin),
                origin_project_integrations::body.eq(req.body),
                origin_project_integrations::project_id.eq(project_id),
                origin_project_integrations::integration_id.eq(integration_id),
            ))
            .on_conflict((
                origin_project_integrations::project_id,
                origin_project_integrations::integration_id,
            ))
            .do_update()
            .set(origin_project_integrations::body.eq(req.body))
            .execute(conn)
    }
}

impl Into<originsrv::OriginProjectIntegration> for ProjectIntegration {
    fn into(self) -> originsrv::OriginProjectIntegration {
        let mut opi = originsrv::OriginProjectIntegration::new();
        opi.set_origin(self.origin);
        opi.set_body(self.body);
        opi
    }
}
