use clap::Clap;

/// This tool assists users in managing a "bank" of wallets. It
/// is very useful for load testing and bulk processing.
#[derive(Clap)]
#[clap(version = "1.0", author = "Chris Bruce")]
pub struct Opts {
    /// Sets the working directory (place where all wallet files are)
    #[clap(short = "d", long = "dir", default_value = ".")]
    pub working_dir: String,
    /// The number of threads to use while processing data.
    /// NOTE: some processes can't make that much use of more than
    /// 1 in order not to have nonce conflicts. Defaults to use same
    /// number of threads as logical CPU cores.
    #[clap(short = "t", long = "threads", default_value = "0")]
    pub threads: usize,
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

    /// Prints the wallet with the highest balance
    #[clap(name = "max-balance")]
    MaxBalance,

    /// Seeds all wallets with equal share from seed account balance.
    /// Will not wait for independent transactions.
    #[clap(name = "seed")]
    Seed(SeedOpts),

    /// Seeds all wallets with equal share from seed account balance.
    /// Will wait for txns to complete so txns are non-dependent.
    #[clap(name = "seed-independent")]
    SeedIndependent(SeedOpts),

    /// Creates a sustained volume of txns/block.
    #[clap(name = "sustained")]
    Sustained(SustainedOpts),
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

/// A subcommand for seeding wallets
#[derive(Clap)]
pub struct SeedOpts {
    /// Seeds all the keys in working director with equal
    /// division of balance, from the address provided.
    pub address: String,
}

/// A subcommand for creating sustained txn volume
#[derive(Clap)]
pub struct SustainedOpts {
    /// The total txns to sustain per block.
    pub count: usize,
}
