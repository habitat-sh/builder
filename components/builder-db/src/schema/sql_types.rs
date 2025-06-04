use diesel::{query_builder::QueryId,
             sql_types::SqlType};

/// Backing Postgres enum for PackageVisibility
#[derive(SqlType, QueryId)]
#[diesel(postgres_type(name = "origin_package_visibility"))]
pub struct OriginPackageVisibility;

/// Backing Postgres enum for PackageChannelOperation
#[derive(SqlType, QueryId)]
#[diesel(postgres_type(name = "package_channel_operation"))]
pub struct PackageChannelOperation;

/// Backing Postgres enum for PackageChannelTrigger
#[derive(SqlType, QueryId)]
#[diesel(postgres_type(name = "package_channel_trigger"))]
pub struct PackageChannelTrigger;

/// Backing Postgres enum for audit_origin.operation
#[derive(SqlType, QueryId)]
#[diesel(postgres_type(name = "origin_operation"))]
pub struct OriginOperation;

/// Backing Postgres enum for origin_members.member_role
#[derive(SqlType, QueryId)]
#[diesel(postgres_type(name = "origin_member_role"))]
pub struct OriginMemberRole;
