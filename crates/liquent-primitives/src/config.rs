//! Configuration options for the Liquent Reth.

use std::sync::OnceLock;

/// Consensus-critical gas limit for every pipe-executed block.
///
/// Must be identical on every node — a per-node flag would let validators with different
/// values produce different block hashes from the same ordered block. Closes
/// liquent-audit#712.
pub const PIPE_BLOCK_GAS_LIMIT: u64 = 1_000_000_000;

/// Configuration options for the Liquent Reth.
#[derive(Debug, Clone)]
pub struct Config {
    /// Whether to disable pipe execution. default false.
    pub disable_pipe_execution: bool,
    /// Whether to disable the Levm executor. default false.
    pub disable_levm: bool,
    /// The max block height between merged and pesist block height.
    pub cache_max_persist_gap: u64,
    /// Persist consecutive blocks in merged groups to amortize per-block fsyncs (much faster
    /// catch-up). Incompatible with Storage V2. default false.
    pub persist_merge_blocks: bool,
    /// The max size of cached items
    pub cache_capacity: u64,
    /// Report db metrics
    pub report_db_metrics: bool,
}

/// Global configuration instance, initialized once.
static GLOBAL_CONFIG: OnceLock<Config> = OnceLock::new();

/// Initialize the global configuration
pub fn init_liquent_config(config: Config) {
    assert!(GLOBAL_CONFIG.set(config).is_ok(), "Global liquent config already initialized");
}

/// Get the global configuration
pub fn get_liquent_config() -> &'static Config {
    GLOBAL_CONFIG.get_or_init(|| Config {
        disable_pipe_execution: std::env::var("LRETH_DISABLE_PIPE_EXECUTION").is_ok(),
        disable_levm: std::env::var("LRETH_DISABLE_LEVM").is_ok(),
        cache_max_persist_gap: 128,
        persist_merge_blocks: false,
        cache_capacity: 2_000_000,
        report_db_metrics: false,
    })
}
