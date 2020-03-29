table! {
    use crate::models::origin::OriginMemberRoleMapping;
    use diesel::sql_types::{BigInt, Text, Nullable, Timestamptz};
    origin_members (origin, account_id) {
        account_id -> BigInt,
        origin -> Text,
        member_role -> OriginMemberRoleMapping,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

use super::{account::accounts,
            origin::{origins,
                     origins_with_stats},
            package::origin_packages};

joinable!(origin_members -> origins (origin));
joinable!(origin_members -> origins_with_stats (origin));
joinable!(origin_members -> accounts (account_id));
allow_tables_to_appear_in_same_query!(origin_members, origins, origins_with_stats, accounts);
allow_tables_to_appear_in_same_query!(origin_members, origin_packages);
