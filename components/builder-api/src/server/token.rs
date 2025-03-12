use super::AppState;
use crate::server::error::Error;
use builder_core::{access_token::AccessToken,
                   privilege::FeatureFlags};
use diesel::result::{DatabaseErrorKind,
                     Error as DieselError};
use habitat_builder_db::models::{account::{Account,
                                           AccountToken,
                                           NewAccount,
                                           NewAccountToken},
                                 channel::{Channel,
                                           CreateChannel},
                                 origin::{NewOrigin,
                                          Origin},
                                 package::PackageVisibility};
use std::{fs::{self,
               File},
          io::Write};

const BLDR_USER_NAME: &str = "chef-platform";
const BLDR_USER_EMAIL: &str = "chef-platform@progress.com";
const BLDR_TOKEN_FILE_NAME: &str = "HAB_AUTH_TOKEN";

/// This function handles the provisioning of the Builder environment.
/// It performs multiple tasks including:
/// 1. Ensuring the creation of a new account if one doesn't exist.
/// 2. Creating origins for the account.
/// 3. Setting up channels for the origins.
/// 4. Generating a user token for authentication.
/// 5. Storing the generated token in a specified file location as defined in the provision config.
pub fn provision_bldr_environment(app_state: &AppState) -> Result<String, Error> {
    // Get or Create Account
    let conn = app_state.db.get_conn().map_err(Error::DbError)?;
    let account = Account::find_or_create(&NewAccount { name:  BLDR_USER_NAME,
                                                        email: BLDR_USER_EMAIL, },
                                          &conn).map_err(Error::DieselError)?;

    for origin in &app_state.config.provision.origins {
        let new_origin = NewOrigin { name: origin,
                                     owner_id: account.id,
                                     default_package_visibility: &PackageVisibility::Public, };

        match Origin::create(&new_origin, &conn) {
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
        let new_channel = CreateChannel { name:     channel,
                                          origin,
                                          owner_id: account.id, };

        match Channel::create(&new_channel, &conn) {
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

    let tokens = AccountToken::list(account.id as u64, &conn).map_err(Error::DieselError)?;
    assert!(tokens.len() <= 1); // Can only have max of 1 token

    // If a token is already found, return it
    if let Some(access_token) = tokens.first() {
        info!("An existing auth token is already present, skipping create");
        return Ok(access_token.token.to_string());
    }

    // Create token
    let token = AccessToken::user_token(&app_state.config.api.key_path,
                                        account.id as u64,
                                        FeatureFlags::all().bits())?;
    let new_token = NewAccountToken { account_id: account.id,
                                      token:      &token.to_string(), };
    AccountToken::create(&new_token, &conn).map_err(Error::DieselError)?;

    // Store the token in a file
    fs::create_dir_all(&app_state.config.provision.token_path).map_err(Error::IO)?;
    let token_file_path = app_state.config
                                   .provision
                                   .token_path
                                   .join(BLDR_TOKEN_FILE_NAME);
    let mut file = File::create(token_file_path).map_err(Error::IO)?;
    file.write_all(token.to_string().as_bytes())
        .map_err(Error::IO)?;

    Ok(token.to_string())
}
