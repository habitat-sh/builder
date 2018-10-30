table! {
    origin_members (origin_id, account_id) {
        origin_id -> BigInt,
        account_id -> BigInt,
        origin_name -> Text,
        account_name -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

use super::origin::origins;
joinable!(origin_members -> origins (origin_id));
allow_tables_to_appear_in_same_query!(origin_members, origins);
