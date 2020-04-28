use glob::glob;
use helium_api::{Account, Client, Hnt};
use helium_wallet::{cmd_create, cmd_pay, cmd_pay::Payee, traits::ReadWrite, wallet::Wallet};
use itertools::Itertools;
use rayon::prelude::*;
use std::{
    error::Error,
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

pub struct Payment {
    pub payer_key_file: PathBuf,
    pub payees_key_files: Vec<PathBuf>,
    pub bones: u64,
}

impl Payment {
    pub fn new_single(payer_key_file: PathBuf, payee_key_file: PathBuf, bones: u64) -> Self {
        Self {
            payer_key_file,
            payees_key_files: vec![payee_key_file],
            bones,
        }
    }

    pub fn new_multi(payer_key_file: PathBuf, payees_key_files: Vec<PathBuf>, bones: u64) -> Self {
        Self {
            payer_key_file,
            payees_key_files,
            bones,
        }
    }
}

pub struct Banker {
    api_url: String,
    password: String,
    working_dir: String,
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
            key_paths: Self::get_key_paths(working_dir),
        }
    }

    pub fn create_wallets(&self, count: usize) {
        for i in 1..=count {
            let n = format!("wallet_{:05}.key", i);
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

    /// Groups account by batch_size and pays each other
    /// waits for blocks, then goes to next group
    /// at end circles back to beginning and starts over
    pub fn pay_forward(&self, batch_size: usize) {
        // let's create payments
        let mut payments: Vec<Payment> = Vec::with_capacity(self.key_paths.len());

        for (pos, path) in self.key_paths.iter().enumerate() {
            let payee = if pos + 1 < self.key_paths.len() {
                self.key_paths[pos + 1].clone()
            } else {
                self.key_paths[0].clone()
            };

            payments.push(Payment::new_single(path.clone(), payee.clone(), 1));
        }

        // loop
        let mut last_height: u64 = self.current_height();
        let mut batch_num = 1;
        loop {
            // Group payments into batches of batch_size
            for payments_batch in payments.chunks(batch_size) {
                println!("Processing batch #{}...", batch_num);
                let now = Instant::now();
                // Parallel process these
                payments_batch.par_iter().for_each(|p| {
                    let r = self.send_payment(p);
                    if r.is_err() {
                        println!("Payment result: {:?}", r);
                    }
                });
                println!(
                    "Processed batch #{} in: {} ms.",
                    batch_num,
                    now.elapsed().as_millis()
                );

                // Wait for next block
                loop {
                    thread::sleep(Duration::from_secs(10));
                    println!("Checking Height: {}", last_height);
                    let height = self.current_height();
                    if height > last_height {
                        last_height = height;
                        break;
                    }
                }
                batch_num += 1;
            }
            // Start all over again.
        }
    }

    /// Seeds with independent process, will sleep until
    /// seed accounts are complete.
    pub fn seed_independent(&self, from_address: &str) {
        let mut seeder_keys: Vec<PathBuf> = vec![];
        let mut seedable_keys: Vec<PathBuf> = vec![];

        // One list of payers and one list of receivers
        for key_path in &self.key_paths {
            if Self::load_wallet(&key_path).address().unwrap() == from_address {
                seeder_keys.push(key_path.clone());
            } else {
                seedable_keys.push(key_path.clone());
            }
        }

        let total_seedable_keys = seedable_keys.len();

        // loop and drain the seedable_keys as payments are sent
        while seedable_keys.len() > 0 {
            // each seeder will pay a range of receivers
            // this drains the seedable list
            let mut payments: Vec<(PathBuf, Vec<PathBuf>)> = seeder_keys
                .iter()
                .map(|p| {
                    let mut range = MAX_MULTIPAY;
                    if range > seedable_keys.len() {
                        range = seedable_keys.len();
                    }
                    (p.clone(), seedable_keys.drain(..range).collect())
                })
                .collect();

            // Remove any postential payers that doen't have any more receivers
            payments = payments.into_iter().filter(|p| p.1.len() > 0).collect();

            // Lets loop through each payer and pay
            payments.par_iter().for_each(|payment| {
                let seed_wallet = Self::load_wallet(&payment.0);
                let seed_address = seed_wallet.address().unwrap();
                let seed_bal = self.get_account_balance(&seed_address);

                let wallet_count: u64 = payment.1.len() as u64;
                let bones = seed_bal / (wallet_count + 1); // plus one is to always keep enough for the seeder account

                let hnt: Hnt = Hnt::from_bones(bones);
                if bones > 0 {
                    println!("Paying out: {} from {}", hnt.to_string(), seed_address);
                    let payees: Vec<cmd_pay::Payee> = payment
                        .1
                        .iter()
                        .map(|p| {
                            let w = Self::load_wallet(&p);
                            Payee::from_str(&format!(
                                "{}={}",
                                w.address().unwrap(),
                                hnt.to_string()
                            ))
                            .unwrap()
                        })
                        .collect();

                    let now = Instant::now();
                    let r = cmd_pay::cmd_pay(
                        self.api_url.clone(),
                        &seed_wallet,
                        &self.password,
                        payees,
                        true,
                        false,
                    );
                    println!("Elapsed Time: {} ms.", now.elapsed().as_millis());
                    println!("Payment result: {:?}", r);
                    println!(
                        "Waiting for txn verification (processed {}/{})...",
                        seeder_keys.len(),
                        total_seedable_keys
                    );

                    let mut last_height = self.current_height();
                    // only wait if no error
                    if r.is_ok() {
                        loop {
                            if seed_bal != self.get_account_balance(&seed_address) {
                                break;
                            }
                            thread::sleep(Duration::from_secs(15));
                            let height = self.current_height();
                            if height > last_height {
                                last_height = height;
                                println!("Checking Height: {}", last_height);
                            }
                        }
                    }
                }
            });

            // All payment txns have been completed
            // copy them to seeder keys so they can help
            // seed next batch.
            payments
                .iter()
                .for_each(|payment| seeder_keys.extend(payment.1.iter().cloned()))
        }
    }

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
                //let before_bal = self.get_account_balance(&seed_address);

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

                // loop {
                //     if before_bal != self.get_account_balance(&seed_address) {
                //         break;
                //     }
                //     println!("Waiting for txn to process...");
                //     thread::sleep(Duration::from_secs(30));
                // }
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
        println!("Current height: {}", self.current_height());
    }

    pub fn send_payment(&self, payment: &Payment) -> Result<(), Box<dyn Error>> {
        let hnt = Hnt::from_bones(payment.bones).to_string();
        let payer_wallet = Self::load_wallet(&payment.payer_key_file);

        let payees: Vec<cmd_pay::Payee> = payment
            .payees_key_files
            .iter()
            .map(|kf| {
                let wallet = Self::load_wallet(kf);
                Payee::from_str(&format!("{}={}", wallet.address().unwrap(), hnt)).unwrap()
            })
            .collect();

        cmd_pay::cmd_pay(
            self.api_url.clone(),
            &payer_wallet,
            &self.password,
            payees,
            true,
            true,
        )
    }

    // TODO: Refactor this into send_payment
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
        let client = Client::new_with_base_url(self.api_url.clone());
        client.get_height().unwrap()
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
            rayon::current_num_threads(),
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
