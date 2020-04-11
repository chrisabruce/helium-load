use glob::glob;
use helium_api::{Account, Client, Hnt};
use helium_wallet::{
    cmd_balance, cmd_create, cmd_pay, cmd_pay::Payee, traits::ReadWrite, wallet::Wallet,
};
use itertools::Itertools;
use std::{
    fmt, fs,
    path::PathBuf,
    str::FromStr,
    thread,
    time::{Duration, Instant},
};

const MAX_MULTIPAY: usize = 50;

pub struct Banker {
    api_url: String,
    password: String,
    working_dir: String,
    threads: usize,
    wallets: Vec<Wallet>,
    client: Client,
}

impl Banker {
    pub fn new(api_url: &str, password: &str, working_dir: &str, threads: usize) -> Self {
        let wallets = Self::collect_wallets(working_dir);
        Self {
            api_url: api_url.to_string(),
            password: password.to_string(),
            working_dir: working_dir.to_string(),
            threads: threads,
            wallets: wallets,
            client: Client::new_with_base_url(api_url.to_string()),
        }
    }

    pub fn create_wallets(&self, count: usize) {
        for i in 1..=count {
            let n = format!("wallet_{:04}.key", i);
            let path = PathBuf::from(n);
            if cmd_create::cmd_basic(&self.password, 2, path.clone(), false, None).is_err() {
                println!("{:?} already exists.", path.display())
            }
        }
    }

    /// Find all .key files in dir
    pub fn collect_wallets(dir: &str) -> Vec<Wallet> {
        let mut wallets: Vec<Wallet> = vec![];

        let mut path = PathBuf::from(dir);
        path.push("*.key");

        for entry in glob(&path.to_string_lossy()).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    //println!("Found wallet: {:?}", path.display());
                    let mut reader = fs::File::open(path).unwrap();
                    wallets.push(Wallet::read(&mut reader).unwrap());
                }
                Err(e) => println!("{:?}", e),
            }
        }
        wallets
    }

    pub fn collect_addresses(&self) -> Vec<String> {
        self.wallets
            .iter()
            .filter(|w| w.address().is_ok())
            .map(|w| w.address().unwrap())
            .collect()
    }

    pub fn get_account(&self, address: &str) -> Option<Account> {
        match self.client.get_account(&address) {
            Ok(account) => Some(account),
            _ => None,
        }
    }

    pub fn get_account_balance(&self, address: &str) -> u64 {
        match self.get_account(address) {
            Some(account) => account.balance,
            _ => 0,
        }
    }

    pub fn get_wallet_balance(&self, wallet: &Wallet) -> u64 {
        self.get_account_balance(&wallet.address().unwrap())
    }

    /// Returns the wallet with the highest balance
    pub fn max_bal_wallet(&self) -> &Wallet {
        self.wallets
            .iter()
            .max_by(|x, y| self.get_wallet_balance(x).cmp(&self.get_wallet_balance(y)))
            .unwrap()
    }

    pub fn print_all_balances(&self) {
        let addresses = self.collect_addresses();
        let _ = cmd_balance::cmd_balance(self.api_url.clone(), addresses);
    }

    pub fn fan_out(&self) {
        let wallets = &self.wallets;
        let key_wallet = wallets.first().unwrap();

        loop {
            self.print_all_balances();
            println!("Fanning out...");
            let watch_bal = self.get_wallet_balance(&key_wallet);
            let wallet_count: u64 = wallets.len() as u64;

            for payer_wallet in wallets {
                if let Ok(payer_address) = payer_wallet.address() {
                    let bones = self.get_account_balance(&payer_address) / wallet_count;
                    let hnt: Hnt = Hnt::from_bones(bones);
                    if bones > 0 {
                        println!("Paying out: {} from {}", hnt.to_string(), payer_address);
                        let payees: Vec<cmd_pay::Payee> = wallets
                            .iter()
                            .filter(|w| {
                                w.address().is_ok() && w.address().unwrap() != payer_address
                            })
                            .map(|w| {
                                Payee::from_str(&format!(
                                    "{}={}",
                                    w.address().unwrap(),
                                    hnt.to_string()
                                ))
                                .unwrap()
                            })
                            .collect();
                        for chunk in &payees.into_iter().chunks(MAX_MULTIPAY) {
                            let now = Instant::now();
                            let r = cmd_pay::cmd_pay(
                                self.api_url.clone(),
                                payer_wallet,
                                &self.password,
                                chunk.collect(),
                                true,
                                true,
                            );

                            println!("Elapsed Time: {} ms.", now.elapsed().as_millis());
                            println!("Payment result: {:?}", r);
                        }
                    }
                }
            }

            loop {
                if watch_bal != self.get_wallet_balance(&key_wallet) {
                    break;
                }

                println!("Sleeping...");
                thread::sleep(Duration::from_secs(30));
            }
        }
    }

    pub fn seed(&self) {
        let seed_wallet = self.max_bal_wallet();
        let seed_address = seed_wallet.address().unwrap();

        let wallet_count: u64 = self.wallets.len() as u64;
        let bones = self.get_account_balance(&seed_address) / wallet_count;

        let hnt: Hnt = Hnt::from_bones(bones);
        if bones > 0 {
            println!("Paying out: {} from {}", hnt.to_string(), seed_address);
            let payees: Vec<cmd_pay::Payee> = self
                .wallets
                .iter()
                .filter(|w| w.address().is_ok() && w.address().unwrap() != seed_address)
                .map(|w| {
                    Payee::from_str(&format!("{}={}", w.address().unwrap(), hnt.to_string()))
                        .unwrap()
                })
                .collect();
            for chunk in &payees.into_iter().chunks(MAX_MULTIPAY) {
                let now = Instant::now();
                let r = cmd_pay::cmd_pay(
                    self.api_url.clone(),
                    &seed_wallet,
                    &self.password,
                    chunk.collect(),
                    true,
                    false,
                );

                println!("Elapsed Time: {} ms.", now.elapsed().as_millis());
                println!("Payment result: {:?}", r);
            }
        }
    }

    /// Collects all wallet balances into a single wallet
    pub fn collect(&self, address: &str) {
        let payee_wallet = self
            .wallets
            .iter()
            .find(|x| x.address().unwrap() == address)
            .unwrap();

        for payer_wallet in &self.wallets {
            if payer_wallet.address().unwrap() != address {
                let bones = self.get_wallet_balance(&payer_wallet);
                self.pay(bones, &payer_wallet, &payee_wallet)
            }
        }
    }

    pub fn pay(&self, bones: u64, payer: &Wallet, payee: &Wallet) {
        if bones > 0 {
            let hnt = Hnt::from_bones(bones);
            let payer_address = payer.address().unwrap();
            let payee_address = payee.address().unwrap();

            println!("Sending {} from {}", hnt.to_string(), payer_address);
            let payee = Payee::from_str(&format!("{}={}", payee_address, hnt.to_string())).unwrap();
            let now = Instant::now();
            let r = cmd_pay::cmd_pay(
                self.api_url.clone(),
                &payer,
                &self.password,
                vec![payee],
                true,
                false,
            );

            println!("Elapsed Time: {} ms.", now.elapsed().as_millis());
            println!("Payment result: {:?}", r);
        }
    }
}

impl fmt::Display for Banker {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} wallets, in the \"{}\" directory using {} with {} threads.",
            self.wallets.len(),
            self.working_dir,
            self.api_url,
            self.threads
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payee_amount() {
        let addr = "13Ad3bq7UDGYUG7xkKGAQX3vJkWQ3B5ERR3FGhhvqnEktnRNtw2";
        let bones = 3774430;

        let payee = Payee::from_str(&format!("{}={}", addr, bones));

        assert!(payee.is_ok());
    }

    #[test]
    fn test_hnt_to_bones() {
        let hnt = Hnt::from_bones(203130111);
        assert_eq!("2.03130111", format!("{}", hnt.to_string()));
    }
}
