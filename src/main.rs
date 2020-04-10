mod bank;
mod cmd;

use bank::Banker;
use clap::Clap;
use dotenv::dotenv;
use std::env;

fn main() {
    dotenv().ok();

    let opts = cmd::Opts::parse();
    let banker = Banker::new(&api_url(), &password(), &opts.working_dir);

    match opts.subcmd {
        cmd::SubCommand::Create(opts) => banker.create_wallets(opts.count),
        cmd::SubCommand::Balances => banker.print_all_balances(),
        cmd::SubCommand::Collect(opts) => banker.collect(&opts.address),
        cmd::SubCommand::Fanout => banker.fan_out(),
        cmd::SubCommand::MaxBalance => {
            let rich_one = banker.max_bal_wallet();
            let addr = rich_one.address().unwrap();
            let bal = banker.get_wallet_balance(&rich_one);
            println!("Richest Wallet: {}: {}", addr, bal);
        }
        cmd::SubCommand::Seed => banker.seed(),
    }
}

fn api_url() -> String {
    env::var("API_URL").expect("Missing API_URL env var.")
}

fn password() -> String {
    env::var("PASSWORD").expect("Missing PASSWORD env var.")
}
