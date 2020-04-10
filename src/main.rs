mod bank;
mod cmd;

use bank::Banker;
use clap::Clap;
use dotenv::dotenv;
use std::{env, thread, time::Duration};

fn main() {
    dotenv().ok();
    let opts = cmd::Opts::parse();

    let banker = Banker::new(&api_url(), &password());

    match opts.subcmd {
        cmd::SubCommand::Create(opts) => banker.create_wallets(opts.count),
        cmd::SubCommand::Balance => banker.print_all_balances(),
        cmd::SubCommand::Fanout => {
            run_fanout(&banker);
        }
    }
}

fn api_url() -> String {
    env::var("API_URL").expect("Missing API_URL env var.")
}

fn password() -> String {
    env::var("PASSWORD").expect("Missing PASSWORD env var.")
}

fn run_fanout(banker: &Banker) {
    let wallets = banker.collect_wallets();

    let key_wallet = wallets.first().unwrap();

    loop {
        banker.print_all_balances();
        println!("Fanning out...");
        let watch_bal = banker.get_wallet_balance(&key_wallet);
        banker.fan_out();

        loop {
            if watch_bal != banker.get_wallet_balance(&key_wallet) {
                break;
            }

            println!("Sleeping...");
            thread::sleep(Duration::from_secs(30));
        }
    }
}
