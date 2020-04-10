use glob::glob;
use helium_api::{Account, Client, Hnt};
use helium_wallet::{
    cmd_balance, cmd_create, cmd_pay, cmd_pay::Payee, traits::ReadWrite, wallet::Wallet,
};
use std::{fs, path::PathBuf, str::FromStr, time::Instant};

pub struct Banker {
    api_url: String,
    password: String,
    working_dir: String,
    client: Client,
}

impl Banker {
    pub fn new(api_url: &str, password: &str) -> Self {
        Self {
            api_url: api_url.to_string(),
            password: password.to_string(),
            working_dir: ".".to_string(),
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
    pub fn collect_wallets(&self) -> Vec<Wallet> {
        let mut wallets: Vec<Wallet> = vec![];

        let mut path = PathBuf::from(&self.working_dir);
        path.push("*.key");

        for entry in glob(&path.to_string_lossy()).expect("Failed to read glob pattern") {
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

    pub fn collect_addresses(&self) -> Vec<String> {
        self.collect_wallets()
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

    pub fn print_all_balances(&self) {
        let addresses = self.collect_addresses();
        let _ = cmd_balance::cmd_balance(self.api_url.clone(), addresses);
    }

    pub fn fan_out(&self) {
        let wallets = &self.collect_wallets();
        let wallet_count: u64 = wallets.len() as u64;

        for payer_wallet in wallets {
            if let Ok(payer_address) = payer_wallet.address() {
                let share: Hnt =
                    Hnt::from_bones(self.get_account_balance(&payer_address) / wallet_count);
                if share.to_bones() > 0 {
                    println!("Paying out: {:?} from {}", share, payer_address);
                    let payees: Vec<cmd_pay::Payee> = wallets
                        .iter()
                        .filter(|w| w.address().is_ok() && w.address().unwrap() != payer_address)
                        .map(|w| {
                            Payee::from_str(&format!(
                                "{}={}",
                                w.address().unwrap(),
                                share.to_string()
                            ))
                            .unwrap()
                        })
                        .collect();

                    let now = Instant::now();
                    let r = cmd_pay::cmd_pay(
                        self.api_url.clone(),
                        payer_wallet,
                        &self.password,
                        payees,
                        true,
                        true,
                    );

                    println!("Elapsed Time: {} ms.", now.elapsed().as_millis());
                    println!("Payment result: {:?}", r);
                }
            }
        }
    }
}
