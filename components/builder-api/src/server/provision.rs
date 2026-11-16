use super::AppState;
use crate::server::error::Error;
use builder_core::{access_token::{AccessToken,
                                 BUILDER_ACCOUNT_ID},
                   privilege::FeatureFlags};
use diesel::{pg::PgConnection,
             result::{DatabaseErrorKind,
                      Error as DieselError}};
use habitat_core::crypto::keys::KeyCache;
use habitat_builder_db::models::{account::{Account,
                                           AccountToken,
                                           NewAccount,
                                           NewAccountToken},
                                 channel::{Channel,
                                           CreateChannel},
                                 origin::{NewOrigin,
                                          Origin},
                                 package::PackageVisibility};
use std::{convert::TryFrom,
          fs,
          io::ErrorKind,
          path::Path};

const BLDR_TOKEN_FILE_NAME: &str = "HAB_AUTH_TOKEN";
const BLDR_ACCOUNT_ID_FILE_NAME: &str = "PROVISIONED_ACCOUNT_ID";

/// This file is written only after provisioning selects an account. Its
/// directory is operator controlled, so it is a trusted account identity on
/// later starts even if the access token has been rotated.
fn account_from_provisioned_id(id_file: &Path,
                               conn: &mut PgConnection)
                               -> Result<Option<Account>, Error> {
    let id = match fs::read_to_string(id_file) {
        Ok(id) => id,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(Error::IO(err)),
    };
    let id = id.trim().parse::<i64>().map_err(|_| Error::Authorization)?;
    if id <= 0 {
        return Err(Error::Authorization);
    }
    Account::get_by_id(id, conn).map(Some).map_err(Error::DieselError)
}

/// The provisioned token identifies its account even after OAuth changes the
/// account name. Check both its signature and the current database token so a
/// revoked or unrelated token cannot select an account at startup.
fn account_from_provisioned_token(token_file: &Path,
                                  key_cache: &KeyCache,
                                  conn: &mut PgConnection)
                                  -> Result<Option<Account>, Error> {
    let token = match fs::read_to_string(token_file) {
        Ok(token) => token,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(Error::IO(err)),
    };
    let token = token.trim();
    let session = AccessToken::validate_access_token(token, key_cache)
        .map_err(|_| Error::Authorization)?;
    if session.id() == BUILDER_ACCOUNT_ID {
        return Err(Error::Authorization);
    }
    let account_id = i64::try_from(session.id()).map_err(|_| Error::Authorization)?;
    let account = Account::get_by_id(account_id, conn).map_err(Error::DieselError)?;
    let tokens = AccountToken::list(session.id(), conn).map_err(Error::DieselError)?;
    if tokens.len() != 1
       || tokens[0].token.trim_end_matches('=') != token.trim_end_matches('=')
    {
        return Err(Error::Authorization);
    }
    Ok(Some(account))
}

fn resolve_provisioning_account(id_file: &Path,
                                token_file: &Path,
                                key_cache: &KeyCache,
                                username: &str,
                                email: &str,
                                conn: &mut PgConnection)
                                -> Result<Account, Error> {
    if let Some(account) = account_from_provisioned_id(id_file, conn)? {
        return Ok(account);
    }
    match account_from_provisioned_token(token_file, key_cache, conn)? {
        Some(account) => Ok(account),
        None => Account::find_or_create(&NewAccount { name: username,
                                                      email },
                                        conn).map_err(Error::DieselError),
    }
}

/// This function handles the provisioning of the Builder environment.
/// It performs multiple tasks including:
/// 1. Ensuring the creation of a new account if one doesn't exist.
/// 2. Creating origins for the account.
/// 3. Setting up channels for the origins.
/// 4. Generating a user token for authentication.
/// 5. Storing the generated token in a specified file location as defined in the provision config.
pub fn provision_bldr_environment(app_state: &AppState) -> Result<String, Error> {
    let id_file_path = app_state.config
                                 .provision
                                 .token_path
                                 .join(BLDR_ACCOUNT_ID_FILE_NAME);
    let token_file_path = app_state.config
                                   .provision
                                   .token_path
                                   .join(BLDR_TOKEN_FILE_NAME);
    let username = &app_state.config.provision.username;
    let email = &app_state.config.provision.email;
    let mut conn = app_state.db.get_conn().map_err(Error::DbError)?;
    let account = resolve_provisioning_account(&id_file_path,
                                                &token_file_path,
                                                &app_state.config.api.key_path,
                                                username,
                                                email,
                                                &mut conn)?;

    for origin in &app_state.config.provision.origins {
        let new_origin = NewOrigin { name: origin,
                                     owner_id: account.id,
                                     default_package_visibility: &PackageVisibility::Public, };

        match Origin::create(&new_origin, &mut conn) {
            Ok(_) => {}
            Err(DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
                // If there is a unique violation error (conflict), log that the origin already
                // exists.
                debug!("Origin {} already exists.", origin);
            }
            Err(err) => {
                error!("Failed to create origin {}, err={:?}", origin, err);
                return Err(Error::DieselError(err));
            }
        }
    }

    // We automatically create stable and unstable channels when an origin is created.
    let filtered_channels: Vec<String> =
        app_state.config
                 .provision
                 .channels
                 .iter()
                 .filter(|&channel| channel != "stable" && channel != "unstable")
                 .cloned()
                 .collect();

    for (origin, channel) in app_state.config
                                      .provision
                                      .origins
                                      .iter()
                                      .zip(&filtered_channels)
    {
        let new_channel = CreateChannel { name: channel,
                                          origin,
                                          owner_id: account.id };

        match Channel::create(&new_channel, &mut conn) {
            Ok(_) => {}
            Err(DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
                // If there is a unique violation error (conflict), log that the origin already
                // exists.
                debug!("Channel {} for Origin {} already exists, skipping creation.",
                       channel, origin);
            }
            Err(err) => {
                error!("Failed to create channel {} for origin {}, err={:?}",
                       channel, origin, err); // Log the error
                return Err(Error::DieselError(err));
            }
        }
    }

    let tokens = AccountToken::list(account.id as u64, &mut conn).map_err(Error::DieselError)?;
    assert!(tokens.len() <= 1); // Can only have max of 1 token

    let token = if let Some(access_token) = tokens.first() {
        info!("An existing auth token is already present, skipping create");
        access_token.token.clone()
    } else {
        let token = AccessToken::user_token(&app_state.config.api.key_path,
                                            account.id as u64,
                                            FeatureFlags::all().bits())?.to_string();
        let new_token = NewAccountToken { account_id: account.id,
                                          token:      &token, };
        AccountToken::create(&new_token, &mut conn).map_err(Error::DieselError)?;
        token
    };

    // Store the token in a file
    fs::create_dir_all(&app_state.config.provision.token_path).map_err(Error::IO)?;
    fs::write(token_file_path, &token).map_err(Error::IO)?;
    fs::write(id_file_path, account.id.to_string()).map_err(Error::IO)?;

    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::Connection;
    use habitat_core::crypto::keys::generate_builder_encryption_key;

    #[test]
    fn restart_uses_provisioned_token_after_oauth_rename() {
        let url = match std::env::var("BUILDER_TEST_DATABASE_URL") {
            Ok(url) => url,
            Err(_) => return,
        };
        let mut conn = PgConnection::establish(&url).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let cache = KeyCache::new(directory.path());
        cache.setup().unwrap();
        cache.write_key(&generate_builder_encryption_key().unwrap()).unwrap();
        let id_file = directory.path().join(BLDR_ACCOUNT_ID_FILE_NAME);
        let token_file = directory.path().join(BLDR_TOKEN_FILE_NAME);
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        let provisioned_name = format!("provisioned_{}", suffix);
        let renamed_name = format!("renamed_{}", suffix);

        conn.test_transaction::<_, DieselError, _>(|conn| {
            let account = resolve_provisioning_account(&id_file,
                                                         &token_file,
                                                         &cache,
                                                         &provisioned_name,
                                                         "provisioned@example.com",
                                                         conn).unwrap();
            let token = AccessToken::user_token(&cache,
                                                account.id as u64,
                                                FeatureFlags::all().bits()).unwrap().to_string();
            AccountToken::create(&NewAccountToken { account_id: account.id,
                                                    token: &token },
                                 conn)?;
            fs::write(&token_file, &token).unwrap();

            let first_login = Account::find_or_create_oauth(&NewAccount { name: &provisioned_name,
                                                                          email: "" },
                                                             "github",
                                                             "provisioned-subject",
                                                             conn)?;
            assert_eq!(first_login.id, account.id);
            let renamed = Account::find_or_create_oauth(&NewAccount { name: &renamed_name,
                                                                      email: "" },
                                                         "github",
                                                         "provisioned-subject",
                                                         conn)?;
            assert_eq!(renamed.id, account.id);
            let other = Account::find_or_create_oauth(&NewAccount { name: &provisioned_name,
                                                                    email: "" },
                                                       "github",
                                                       "other-subject",
                                                       conn)?;
            assert_ne!(other.id, account.id);

            let resolved = resolve_provisioning_account(&id_file,
                                                         &token_file,
                                                         &cache,
                                                         &provisioned_name,
                                                         "provisioned@example.com",
                                                         conn).unwrap();
            assert_eq!(resolved.id, account.id);
            assert_eq!(resolved.name, renamed_name);

            fs::remove_file(&token_file).unwrap();
            assert!(matches!(resolve_provisioning_account(&id_file,
                                                           &token_file,
                                                           &cache,
                                                           &provisioned_name,
                                                           "provisioned@example.com",
                                                           conn),
                             Err(Error::DieselError(DieselError::NotFound))));

            fs::write(&id_file, account.id.to_string()).unwrap();
            let from_id = resolve_provisioning_account(&id_file,
                                                        &token_file,
                                                        &cache,
                                                        &provisioned_name,
                                                        "provisioned@example.com",
                                                        conn).unwrap();
            assert_eq!(from_id.id, account.id);

            fs::write(&token_file, &token).unwrap();
            let replacement = AccessToken::user_token(&cache,
                                                      account.id as u64,
                                                      FeatureFlags::all().bits()).unwrap().to_string();
            AccountToken::create(&NewAccountToken { account_id: account.id,
                                                    token: &replacement },
                                 conn)?;
            assert_eq!(resolve_provisioning_account(&id_file,
                                                    &token_file,
                                                    &cache,
                                                    &provisioned_name,
                                                    "provisioned@example.com",
                                                    conn).unwrap().id,
                       account.id);

            fs::remove_file(&id_file).unwrap();
            assert!(matches!(resolve_provisioning_account(&id_file,
                                                           &token_file,
                                                           &cache,
                                                           &provisioned_name,
                                                           "provisioned@example.com",
                                                           conn),
                             Err(Error::Authorization)));

            fs::write(&id_file, "not-an-id").unwrap();
            assert!(matches!(resolve_provisioning_account(&id_file,
                                                           &token_file,
                                                           &cache,
                                                           &provisioned_name,
                                                           "provisioned@example.com",
                                                           conn),
                             Err(Error::Authorization)));
            Ok(())
        });
    }
}
