use clap::Clap;

/// This tool assists users in managing a "bank" of wallets. It
/// is very useful for load testing and bulk processing.
#[derive(Clap)]
#[clap(version = "1.0", author = "Chris Bruce")]
pub struct Opts {
    /// Sets the working directory (place where all wallet files are)
    #[clap(short = "d", long = "dir", default_value = ".")]
    pub working_dir: String,
    /// Group into n wallets per thread, othwise use all
    /// wallets in single thread
    #[clap(short = "n", long = "num-wallets", default_value = "0")]
    pub num_wallets: usize,
    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

#[derive(Clap)]
pub enum SubCommand {
    /// Prints the balance of all wallets
    #[clap(name = "balances")]
    Balances,

    /// Collect all wallet balances into a single wallet
    #[clap(name = "collect")]
    Collect(CollectOpts),

    /// Create x number of wallets
    #[clap(name = "create")]
    Create(CreateOpts),

    /// Distributes each wallet's balance amongst all other wallets
    #[clap(name = "fanout")]
    Fanout,

    // Prints the balance of all wallets
    #[clap(name = "max-balance")]
    MaxBalance,

    /// Find wallet with highest balance and seeds all other wallets
    #[clap(name = "seed")]
    Seed,
}

/// A subcommand for controlling wallet creation
#[derive(Clap)]
pub struct CreateOpts {
    /// The number of wallets to create
    pub count: usize,
}

/// A subcommand for collecting wallet balances
#[derive(Clap)]
pub struct CollectOpts {
    /// The address to collect all balances into
    pub address: String,
}
