WITH legacy AS (
    INSERT INTO accounts (name, email)
    VALUES ('oauth-identity-legacy', 'legacy@example.com')
    RETURNING id
), collision AS (
    INSERT INTO accounts (name, email)
    VALUES ('oauth-identity-collision-legacy', 'oauth-identity-collision-legacy@example.com')
    RETURNING id
), backfilled AS (
    INSERT INTO accounts (name, email, provider, extern_id)
    VALUES ('oauth-identity-backfilled', '', 'github', '1000006')
    RETURNING id
), fixed_user AS (
    INSERT INTO accounts (name, email, provider, extern_id)
    VALUES ('bobo', 'bobo@example.com', 'github', 'oauth-identity-fixed-user')
    RETURNING id
)
SELECT json_build_object(
    'legacy', legacy.id::text,
    'collision', collision.id::text,
    'backfilled', backfilled.id::text,
    'fixed_user', fixed_user.id::text
)
FROM legacy, collision, backfilled, fixed_user;
