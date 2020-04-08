use helium_wallet::wallet;
use std::error::Error;

pub struct Load {
    api_url: String,
    password: String,
}

impl Load {
    pub fn new(api_url: &str, password: &str) -> Self {
        Self {
            api_url: api_url.to_string(),
            password: password.to_string(),
        }
    }

    /// Pays back and forth between two accounts.
    pub fn start_pong(&self, _interval: u64) -> Result<(), Box<dyn Error>> {
        // Find or create 2 accounts
        Ok(())
    }
}
