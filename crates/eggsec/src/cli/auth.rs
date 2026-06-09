use super::timeout::*;
use clap::Args;

pub(crate) const AUTH_TEST_ABOUT: &str =
    "Validate authentication controls in authorized environments

Tests authentication mechanisms including credential validation, rate limiting,
session handling, timing behavior, and multi-factor authentication in lab settings.

⚠️  WARNING: Only use against systems you have explicit permission to test.

Examples:
  eggsec auth-test https://example.com/login --all -u admin
  eggsec auth-test https://example.com/login --credential-stuffing --wordlist passwords.txt
  eggsec auth-test https://example.com/login --mfa-test
  eggsec auth-test https://example.com/login --all --max-attempts 1000";

#[derive(Args, Clone)]
pub struct AuthTestArgs {
    #[arg(help = "Target authentication endpoint URL")]
    pub target: String,

    #[arg(long, help = "Username for authentication testing")]
    pub username: Option<String>,

    #[arg(long, help = "Password wordlist file path")]
    pub wordlist: Option<String>,

    #[arg(long, help = "Credential pairs file (username:password format)")]
    pub credential_file: Option<String>,

    #[arg(
        long,
        default_value = "100",
        help = "Maximum authentication attempts before stopping"
    )]
    pub max_attempts: usize,

    #[arg(short = 'c', long, default_value = "1", help = "Concurrent requests")]
    pub concurrency: usize,

    #[arg(long, default_value_t = AUTH_TEST_TIMEOUT, help = "Request timeout in seconds")]
    pub timeout: u64,

    #[arg(long, help = "Run authentication attempt testing")]
    pub brute_force: bool,

    #[arg(long, help = "Run credential validation testing")]
    pub credential_stuffing: bool,

    #[arg(long, help = "Run account lockout detection")]
    pub lockout_detection: bool,

    #[arg(long, help = "Test rate limiting effectiveness")]
    pub rate_limit_bypass: bool,

    #[arg(long, help = "Test MFA enforcement strength")]
    pub mfa_bypass: bool,

    #[arg(long, help = "Run session fixation testing")]
    pub session_fixation: bool,

    #[arg(long, help = "Run timing attack analysis")]
    pub timing_attack: bool,

    #[arg(long, help = "Run all authentication tests")]
    pub all: bool,

    #[arg(long, help = "Output results as JSON")]
    pub json: bool,

    #[arg(long, help = "Verbose output")]
    pub verbose: bool,

    #[arg(long, short = 'o', help = "Output file path")]
    pub output: Option<String>,

    #[arg(long, short = 'y', help = "Skip confirmation prompt")]
    pub yes: bool,

    #[arg(long, short = 'q', help = "Suppress non-essential output")]
    pub quiet: bool,
}
