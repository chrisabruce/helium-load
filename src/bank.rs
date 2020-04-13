use glob::glob;
use helium_api::{Account, Client, Hnt};
use helium_wallet::{cmd_create, cmd_pay, cmd_pay::Payee, traits::ReadWrite, wallet::Wallet};
use itertools::Itertools;
use rayon::prelude::*;
use std::{
    fmt, fs,
    path::PathBuf,
    str::FromStr,
    thread,
    time::{Duration, Instant},
};

const MAX_MULTIPAY: usize = 50;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Balance {
    pub key_file: String,
    pub address: String,
    pub balance: Option<u64>,
    pub error: Option<String>,
}

pub struct Banker {
    api_url: String,
    password: String,
    working_dir: String,
    threads: usize,
    key_paths: Vec<PathBuf>,
}

impl Banker {
    pub fn new(api_url: &str, password: &str, working_dir: &str, threads: usize) -> Self {
        // Set the global threads.  If `0` then uses number of threads equal to logical cores
        if threads > 0 {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build_global()
                .unwrap();
        }
        Self {
            api_url: api_url.to_string(),
            password: password.to_string(),
            working_dir: working_dir.to_string(),
            threads: threads,
            key_paths: Self::get_key_paths(working_dir),
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

    /// Finds and returns a list of all the keyfiles found
    /// in the directory.
    pub fn get_key_paths(dir: &str) -> Vec<PathBuf> {
        let mut path = PathBuf::from(dir);
        path.push("*.key");

        let mut key_paths = vec![];

        for entry in glob(&path.to_string_lossy()).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    key_paths.push(path);
                    //println!("Found wallet: {:?}", path.display());
                }
                Err(e) => println!("{:?}", e),
            }
        }

        key_paths
    }

    /// Loads a wallet from file path
    pub fn load_wallet(key_file: &PathBuf) -> Wallet {
        let mut reader = fs::File::open(key_file).unwrap();
        Wallet::read(&mut reader).unwrap()
    }

    /// Get a list of wallets from key file paths
    pub fn collect_wallets(&self) -> Vec<Wallet> {
        self.key_paths
            .iter()
            .map(|p| Self::load_wallet(p))
            .collect()
    }

    pub fn wallet_from_address(&self, address: &str) -> Option<Wallet> {
        self.collect_wallets()
            .into_iter()
            .find(|x| x.address().unwrap() == address)
    }

    pub fn get_account(&self, address: &str) -> Option<Account> {
        let client = Client::new_with_base_url(self.api_url.clone());
        match client.get_account(&address) {
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
    pub fn max_bal_wallet(&self) -> Wallet {
        self.collect_wallets()
            .into_iter()
            .max_by(|x, y| self.get_wallet_balance(x).cmp(&self.get_wallet_balance(y)))
            .unwrap()
    }

    pub fn print_all_balances(&self) {
        let mut balances: Vec<Balance> = self
            .key_paths
            .par_iter()
            .map(|p| {
                let wallet = Self::load_wallet(p);
                let address = wallet.address().unwrap();

                let client = Client::new_with_base_url(self.api_url.clone());

                let mut b = Balance {
                    key_file: p.to_string_lossy().to_string(),
                    address: address.clone(),
                    balance: None,
                    error: None,
                };

                match client.get_account(&address) {
                    Ok(account) => b.balance = Some(account.balance),
                    Err(e) => b.error = Some(e.to_string()),
                };

                b
            })
            .collect();

        balances.sort();

        let mut table = prettytable::Table::new();
        table.add_row(row!["Key", "Address", "Bones", "Error"]);
        for b in balances {
            table.add_row(row![
                b.key_file,
                b.address,
                b.balance.unwrap_or(0),
                b.error.unwrap_or("n/a".to_string())
            ]);
        }
        table.printstd();
    }

    pub fn fan_out(&self) {
        let wallets = self.collect_wallets();
        let key_wallet = wallets.first().unwrap();

        loop {
            self.print_all_balances();
            println!("Fanning out...");
            let watch_bal = self.get_wallet_balance(&key_wallet);
            let wallet_count: u64 = self.key_paths.len() as u64;

            for payer_wallet in self.collect_wallets() {
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
                                &payer_wallet,
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
                //TODO fix this as balance might remain same
                if watch_bal != self.get_wallet_balance(&key_wallet) {
                    break;
                }

                println!("Sleeping...");
                thread::sleep(Duration::from_secs(30));
            }
        }
    }

    /// Seeds with independent process, will sleep until
    /// seed accounts are complete.
    pub fn seed_independent(&self, from_address: &str) {}

    /// Will take and evenly distribute funds from either the
    /// highest balance wallet  or from the `from_address`
    pub fn seed(&self, from_address: &str) {
        let seed_wallet = self.wallet_from_address(from_address).unwrap();

        let seed_address = seed_wallet.address().unwrap();

        let wallet_count: u64 = self.key_paths.len() as u64;
        let bones = self.get_account_balance(&seed_address) / wallet_count;

        let hnt: Hnt = Hnt::from_bones(bones);
        if bones > 0 {
            println!("Paying out: {} from {}", hnt.to_string(), seed_address);
            let payees: Vec<cmd_pay::Payee> = self
                .collect_wallets()
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
        self.key_paths.par_iter().for_each(|p| {
            let payee_wallet = self.wallet_from_address(address).unwrap();
            let payer_wallet = Self::load_wallet(p);
            if payer_wallet.address().unwrap() != payee_wallet.address().unwrap() {
                let bones = self.get_wallet_balance(&payer_wallet);
                self.pay(bones, &payer_wallet, &payee_wallet)
            };
        });
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

    pub fn current_height(&self) -> u64 {
        unimplemented!()
    }
}

impl fmt::Display for Banker {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} wallets, in the \"{}\" directory using {} with {} threads.",
            self.key_paths.len(),
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
