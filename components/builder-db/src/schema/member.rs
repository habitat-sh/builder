table! {
    origin_members (origin, account_id) {
        account_id -> BigInt,
        origin -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

use super::{account::accounts,
            origin::{origins,
                     origins_with_stats}};

joinable!(origin_members -> origins (origin));
joinable!(origin_members -> origins_with_stats (origin));
joinable!(origin_members -> accounts (account_id));
allow_tables_to_appear_in_same_query!(origin_members, origins, origins_with_stats, accounts);
