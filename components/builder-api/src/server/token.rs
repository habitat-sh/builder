use crate::server::error::Error;
use builder_core::{access_token::AccessToken, privilege::FeatureFlags};
use habitat_builder_db::models::{account::{Account, AccountToken, NewAccount, NewAccountToken}, origin::{NewOrigin, Origin}, package::PackageVisibility};
use std::{fs::{self, File}, io::Write};
use super::AppState;

const BLDR_USER_NAME: &str = "chef-platform";
const BLDR_USER_EMAIL: &str = "chef-platform@progress.com";

pub fn provision_bldr_environment(app_state: AppState) -> Result<String, Error> {
    // Get or Create Account
    let conn = app_state.db.get_conn().map_err(Error::DbError)?;
    let account = match Account::find_or_create(&NewAccount { name: BLDR_USER_NAME, email: BLDR_USER_EMAIL }, &conn) {
        Ok(account) => account,
        Err(e) => {
            error!("Failed to find or create account: {}", e);
            return Err(e.into());
        }
    };
    println!("Account: {:?}", account);

    // Create origin
    let new_origin = NewOrigin { name: "neurosis",
                                 owner_id: account.id as i64,
                                 default_package_visibility: &PackageVisibility::Public, };

    let origin = Origin::create(&new_origin, &conn).map_err(Error::DieselError)?;
    println!("Origin: {:?}", origin);

    // Create token
    let token = AccessToken::user_token(&app_state.config.api.key_path, account.id as u64, FeatureFlags::all().bits())?;

    // Prepare to store the token
    let new_token = NewAccountToken { account_id: account.id as i64,
                                      token: &token.to_string(), };

    let acc_tkn = AccountToken::create(&new_token, &conn).map_err(Error::DieselError)?;
    println!("Token: {:?}", acc_tkn);

    // Ensure the directory exists from the config path
    fs::create_dir_all(&app_state.config.provision.token_path).map_err(Error::IO)?;

    // Construct the file path using the token path from the config
    let token_file_path = app_state.config.provision.token_path.join("HAB_AUTH_TOKEN");

    // Create or open the file and write the token
    let mut file = File::create(token_file_path).map_err(Error::IO)?;
    file.write_all(token.to_string().as_bytes()).map_err(Error::IO)?;

    Ok(token.to_string())
}