table! {
    origin_project_integrations (id) {
        id -> BigInt,
        project_id -> BigInt,
        integration_id -> BigInt,
        origin -> Text,
        body -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

use super::integration::origin_integrations;
use super::project::origin_projects;

joinable!(origin_project_integrations -> origin_projects (project_id));
joinable!(origin_project_integrations -> origin_integrations (integration_id));

allow_tables_to_appear_in_same_query!(
    origin_project_integrations,
    origin_projects,
    origin_integrations
);
