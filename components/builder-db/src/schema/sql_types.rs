#[derive(diesel::sql_types::SqlType, diesel::query_builder::QueryId, Clone)]
#[diesel(postgres_type(name = "origin_package_visibility"))]
pub struct origin_package_visibility;

