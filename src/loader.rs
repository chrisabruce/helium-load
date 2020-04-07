pub struct Load {
    api_url: String,
    password: String,
}

impl Load {
    pub fn new(api_url: &str, password: &str) -> Self {
        Self {
            api_url: api_url.to_string(),
            pasword: password.to_string(),
        }
    }
}
