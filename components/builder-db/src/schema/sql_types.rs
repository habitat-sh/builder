use diesel::sql_types::SqlType;
use diesel::query_builder::QueryId;

/// Backing Postgres enum for PackageVisibility
#[derive(SqlType, QueryId, Clone)]
#[diesel(postgres_type(name = "origin_package_visibility"))]
pub struct origin_package_visibility;

/// Backing Postgres enum for PackageChannelOperation
#[derive(SqlType, QueryId, Clone)]
#[diesel(postgres_type(name = "package_channel_operation"))]
pub struct package_channel_operation;

/// Backing Postgres enum for PackageChannelTrigger
#[derive(SqlType, QueryId, Clone)]
#[diesel(postgres_type(name = "package_channel_trigger"))]
pub struct package_channel_trigger;

/// Backing Postgres enum for audit_origin.operation
#[derive(SqlType, QueryId, Clone)]
#[diesel(postgres_type(name = "origin_operation"))]
pub struct origin_operation;

/// Backing Postgres enum for origin_members.member_role
#[derive(SqlType, QueryId, Clone)]
#[diesel(postgres_type(name = "origin_member_role"))]
pub struct origin_member_role;


