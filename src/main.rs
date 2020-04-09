mod loader;

use clap::{App, Arg};
use dotenv::dotenv;
use glob::glob;
use helium_api::{Account, Client, Hnt};
use helium_wallet::{
    cmd_balance, cmd_create, cmd_hotspots, cmd_htlc, cmd_info, cmd_pay, cmd_pay::Payee, cmd_verify,
    mnemonic, result, traits, wallet,
};
use helium_wallet::{result::Result, traits::ReadWrite, wallet::Wallet};
use loader::Load;
use std::str::FromStr;
use std::thread;
use std::time::{Duration, Instant};
use std::{env, fs, path::PathBuf, process};

fn main() {
    dotenv().ok();

    let api_url = api_url();
    let password = password();

    create_wallets(50, &password);

    let wallets = collect_wallets();

    let key_wallet = wallets.first().unwrap();

    loop {
        print_all_balances();
        println!("Fanning out...");
        let watch_bal = get_wallet_balance(&key_wallet);
        fan_out();

        loop {
            if watch_bal != get_wallet_balance(&key_wallet) {
                break;
            }

            println!("Sleeping...");
            thread::sleep(Duration::from_secs(30));
        }
    }
}

fn create_wallets(count: usize, password: &str) {
    for i in 1..=count {
        let n = format!("wallet_{}.key", i);
        let path = PathBuf::from(n);
        if cmd_create::cmd_basic(password, 1_000, path.clone(), false, None).is_err() {
            println!("{:?} already exists.", path.display())
        }
    }
}

/// Find all .key files in dir
fn collect_wallets() -> Vec<Wallet> {
    let mut wallets: Vec<Wallet> = vec![];

    for entry in glob("*.key").expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                println!("Found wallet: {:?}", path.display());
                let mut reader = fs::File::open(path).unwrap();
                wallets.push(Wallet::read(&mut reader).unwrap());
            }
            Err(e) => println!("{:?}", e),
        }
    }
    wallets
}

fn collect_addresses() -> Vec<String> {
    collect_wallets()
        .iter()
        .filter(|w| w.address().is_ok())
        .map(|w| w.address().unwrap())
        .collect()
}

fn get_account(address: &str) -> Option<Account> {
    let client = Client::new_with_base_url(api_url());

    match client.get_account(&address) {
        Ok(account) => Some(account),
        _ => None,
    }
}

fn get_account_balance(address: &str) -> u64 {
    match get_account(address) {
        Some(account) => account.balance,
        _ => 0,
    }
}

fn get_wallet_balance(wallet: &Wallet) -> u64 {
    get_account_balance(&wallet.address().unwrap())
}

fn api_url() -> String {
    env::var("API_URL").expect("Missing API_URL env var.")
}

fn password() -> String {
    env::var("PASSWORD").expect("Missing PASSWORD env var.")
}

fn print_all_balances() {
    let addresses = collect_addresses();
    cmd_balance::cmd_balance(api_url(), addresses);
}

fn fan_out() {
    let wallets = &collect_wallets();
    let wallet_count: u64 = wallets.len() as u64;

    for payer_wallet in wallets {
        if let Ok(payer_address) = payer_wallet.address() {
            let share: Hnt = Hnt::from_bones(get_account_balance(&payer_address) / wallet_count);
            if share.to_bones() > 0 {
                let payees: Vec<cmd_pay::Payee> = wallets
                    .iter()
                    .filter(|w| w.address().is_ok() && w.address().unwrap() != payer_address)
                    .map(|w| {
                        Payee::from_str(&format!("{}={}", w.address().unwrap(), share.to_string()))
                            .unwrap()
                    })
                    .collect();

                let now = Instant::now();
                let r = cmd_pay::cmd_pay(api_url(), payer_wallet, &password(), payees, true, true);

                println!("Elapsed Time: {} ms.", now.elapsed().as_millis());
                println!("Payment result: {:?}", r);
            }
        }
    }
}
