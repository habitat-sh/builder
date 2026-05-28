use anyhow::{Context,
             Result};
use builder_core::{access_token::AccessToken,
                   privilege::FeatureFlags};
use clap::Parser;
use habitat_core::crypto::keys::KeyCache;
use std::path::PathBuf;

/// CLI tool to generate user authentication tokens
#[derive(Parser, Debug)]
#[command(name = "token-generator",
          about = "Generate user authentication tokens for Habitat Builder")]
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

/// Maps the CLI verbosity flag to the logger's default filter level.
fn log_level(verbose: bool) -> log::LevelFilter {
    if verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    }
}

/// Initializes env_logger once using the verbosity requested by the caller.
fn init_logging(verbose: bool) {
    env_logger::Builder::from_default_env().filter_level(log_level(verbose))
                                           .init();
}

/// Validates CLI input before attempting token generation.
fn validate_args(args: &Args) -> Result<()> {
    if !args.key_path.exists() {
        anyhow::bail!("Key path does not exist: {}", args.key_path.display());
    }

    if !args.key_path.is_dir() {
        anyhow::bail!("Key path must be a directory: {}", args.key_path.display());
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    init_logging(args.verbose);
    validate_args(&args)?;

    log::info!("Generating token for account ID: {}", args.account_id);
    log::debug!("Using key path: {}", args.key_path.display());

    // Reuse Builder's access token helper so the CLI issues tokens in the same
    // format as the provisioning path.
    let token = AccessToken::user_token(&KeyCache::new(&args.key_path),
                                        args.account_id,
                                        FeatureFlags::empty().bits()).with_context(|| {
                                                                         format!(
            "Failed to generate user token for account {} with key path {}",
            args.account_id,
            args.key_path.display()
        )
                                                                     })?;

    println!("{}", token);

    log::info!("Token generated successfully");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{hint::black_box,
              time::Instant};

    #[test]
    fn test_args_parsing() {
        let args = Args::parse_from(["token-generator",
                                     "--account-id",
                                     "12345",
                                     "--key-path",
                                     "/path/to/key"]);

        assert_eq!(args.account_id, 12345);
        assert_eq!(args.key_path, PathBuf::from("/path/to/key"));
        assert!(!args.verbose);
    }

    #[test]
    fn test_args_parsing_verbose() {
        let args = Args::parse_from(["token-generator",
                                     "--account-id",
                                     "12345",
                                     "--key-path",
                                     "/path/to/key",
                                     "--verbose"]);

        assert!(args.verbose);
    }

    #[test]
    fn test_missing_key_path_is_rejected() {
        let err =
            Args::try_parse_from(["token-generator", "--account-id", "12345"]).unwrap_err();

        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn test_log_level_defaults_to_info() {
        assert_eq!(log_level(false), log::LevelFilter::Info);
    }

    #[test]
    fn test_log_level_verbose_is_debug() {
        assert_eq!(log_level(true), log::LevelFilter::Debug);
    }

    #[test]
    fn test_nonexistent_key_path_validation() {
        let args = Args { account_id: 12345,
                          key_path:   PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                                          .join("does-not-exist"),
                          verbose:    false, };

        let err = validate_args(&args).unwrap_err();

        assert_eq!(err.to_string(),
                   format!("Key path does not exist: {}", args.key_path.display()));
    }

    #[test]
    fn test_key_path_must_be_a_directory() {
        let args = Args { account_id: 12345,
                          key_path:   PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                                          .join("Cargo.toml"),
                          verbose:    false, };

        let err = validate_args(&args).unwrap_err();

        assert_eq!(err.to_string(),
                   format!("Key path must be a directory: {}", args.key_path.display()));
    }

    #[test]
    #[ignore = "micro-benchmark; run explicitly in release mode"]
    fn benchmark_log_level() {
        const ITERATIONS: u32 = 5_000_000;

        let start = Instant::now();
        let mut checksum = 0_u32;

        for i in 0..ITERATIONS {
            let level = black_box(log_level(black_box(i % 2 == 0)));
            checksum ^= match level {
                log::LevelFilter::Debug => 1,
                log::LevelFilter::Info => 2,
                _ => 0,
            };
        }

        let elapsed = start.elapsed();
        let ns_per_iter = elapsed.as_nanos() as f64 / ITERATIONS as f64;

        println!(
            "benchmark_log_level iterations={} elapsed_ns={} ns_per_iter={:.2} checksum={}",
            ITERATIONS,
            elapsed.as_nanos(),
            ns_per_iter,
            checksum
        );
    }
}
