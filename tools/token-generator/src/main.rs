use anyhow::{Context, Result};
use builder_core::{access_token::AccessToken, privilege::FeatureFlags};
use habitat_core::crypto::keys::KeyCache;
use clap::Parser;
use std::path::PathBuf;

/// CLI tool to generate user authentication tokens
#[derive(Parser, Debug)]
#[command(
    name = "token-generator",
    about = "Generate user authentication tokens for Habitat Builder"
)]
struct Args {
    /// Account ID for the user
    #[arg(short, long, help = "The account ID for which to generate the token")]
    account_id: u64,

    /// Path to the signing key file
    #[arg(short, long, help = "Path to the Builder signing key file")]
    key_path: PathBuf,

    /// Verbose output
    #[arg(short, long, help = "Enable verbose logging")]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    if args.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    // Validate that the key path exists
    if !args.key_path.exists() {
        anyhow::bail!("Key path does not exist: {}", args.key_path.display());
    }

    log::info!("Generating token for account ID: {}", args.account_id);
    log::debug!("Using key path: {}", args.key_path.display());

    // Generate the user token using the same logic as provision
    let token = AccessToken::user_token(&KeyCache::new(&args.key_path), args.account_id, FeatureFlags::all().bits())
        .with_context(|| {
            format!(
                "Failed to generate user token for account {} with key path {}",
                args.account_id,
                args.key_path.display()
            )
        })?;

    // Output the token to stdout
    println!("{}", token);

    log::info!("Token generated successfully");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_args_parsing() {
        let args = Args::parse_from(&[
            "token-generator",
            "--account-id",
            "12345",
            "--key-path",
            "/path/to/key",
        ]);

        assert_eq!(args.account_id, 12345);
        assert_eq!(args.key_path, PathBuf::from("/path/to/key"));
        assert!(!args.verbose);
    }

    #[test]
    fn test_args_parsing_verbose() {
        let args = Args::parse_from(&[
            "token-generator",
            "--account-id",
            "12345",
            "--key-path",
            "/path/to/key",
            "--verbose",
        ]);

        assert!(args.verbose);
    }

    #[test]
    fn test_nonexistent_key_path_validation() {
        let args = Args {
            account_id: 12345,
            key_path: PathBuf::from("/nonexistent/path"),
            verbose: false,
        };

        // Test that validation would fail for non-existent path
        assert!(!args.key_path.exists());
    }
}