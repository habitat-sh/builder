table! {
    origin_packages (id) {
        id -> BigInt,
        origin_id -> BigInt,
        owner_id -> BigInt,
        name -> Text,
        ident -> Text,
        checksum -> Text,
        manifest -> Text,
        config -> Text,
        target -> Text,
        deps -> Text,
        tdeps -> Text,
        exposes -> Text,
        visibility -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}
