use clap::Clap;

/// This tool assists users in managing a "bank" of wallets. It
/// is very useful for load testing and bulk processing.
#[derive(Clap)]
#[clap(version = "1.0", author = "Chris Bruce")]
pub struct Opts {
    /// Sets the working directory (place where all wallet files are)
    #[clap(short = "d", long = "dir", default_value = ".")]
    pub working_dir: String,
    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

#[derive(Clap)]
pub enum SubCommand {
    /// Prints the balance of all wallets
    #[clap(name = "balance")]
    Balance,

    /// Create x number of wallets
    #[clap(name = "create")]
    Create(CreateOpts),

    /// Distributes each wallet's balance amongst all other wallets
    #[clap(name = "fanout")]
    Fanout,
}

/// A subcommand for controlling wallet creation
#[derive(Clap)]
pub struct CreateOpts {
    /// The number of wallets to create
    pub count: usize,
}
