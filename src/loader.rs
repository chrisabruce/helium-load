use rand::prelude::*;
use std::error::Error;
use std::time::Duration;
use tokio::prelude::*;
use tokio::timer::Interval;

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

    /// Pays back and forth between two accounts.
    pub fn start_pong(&self, interval: u64) -> Result<(), Box<dyn Error>> {
        let node = helium::Node::new("localhost", 4001);
        let accounts = node.list_accounts().unwrap();

        println!("Found {} account(s).", accounts.len());
        if accounts.is_empty() {
            panic!("Requires two existing accounts.");
        }

        let interval = Duration::new(interval, 0);
        let task = Interval::new_interval(interval)
            .for_each(move |_| {
                let mut rng = rand::thread_rng();
                let amt: u64 = rng.gen_range(10_000, 100_000);

                println!("Paying: {}", amt);
                node.pay(&accounts[0].address, &accounts[1].address, amt)
                    .unwrap();

                node.pay(&accounts[1].address, &accounts[0].address, amt)
                    .unwrap();

                Ok(())
            })
            .map_err(|e| println!("interval errored; err={:?}", e));

        tokio::run(task);
    }

    pub fn ping(&self, interval: u64) {
        let node = helium::Node::new("localhost", 4001);
        let accounts = node.list_accounts().unwrap();

        println!("Found {} account(s).", accounts.len());
        if accounts.is_empty() {
            panic!("Requires two existing accounts.");
        }

        let mut last_height = node.status().unwrap().chain_height;

        let interval = Duration::new(interval, 0);
        let task = Interval::new_interval(interval)
            .for_each(move |_| {
                println!("Checking...");
                let cur_height = node.status().unwrap().node_height; // want to make sure node is current
                if cur_height > last_height {
                    println!("New height: {}", cur_height);
                    let mut rng = rand::thread_rng();
                    let amt: u64 = rng.gen_range(10_000, 100_000);

                    println!("Paying: {}", amt);
                    node.pay(&accounts[0].address, &accounts[1].address, amt)
                        .unwrap();

                    node.pay(&accounts[1].address, &accounts[0].address, amt)
                        .unwrap();
                    last_height = cur_height;
                }
                Ok(())
            })
            .map_err(|e| println!("interval errored; err={:?}", e));

        tokio::run(task);
    }

    pub fn multiping(&self, interval: u64) {
        let min_accts = 2;

        let node1 = helium::Node::new("localhost", 4002);
        let node2 = helium::Node::new("localhost", 4003);
        let node3 = helium::Node::new("localhost", 4004);
        let node4 = helium::Node::new("localhost", 4005);
        let node5 = helium::Node::new("localhost", 4006);
        let nodes: Vec<helium::Node> = vec![node1, node2, node3, node4, node5];
        let mut last_height = nodes[0].status().unwrap().chain_height;

        // Make sure we have enough accounts
        for n in nodes {
            let accts = n.list_accounts().unwrap();
            let i = min_accts - accts.len();

            if i > 0 {
                for _ in 0..i {
                    let a = n.create_account().unwrap();
                    println!("Created account: {}", a.address);
                }
            }
        }

        // Loop on interval and make payments
        let interval = Duration::new(interval, 0);
        let task = Interval::new_interval(interval)
            .for_each(move |_| {
                let node1 = helium::Node::new("localhost", 4002);

                println!("Checking...");
                let cur_height = node1.status().unwrap().node_height; // want to make sure node is current
                if cur_height > last_height {
                    println!("New height: {}", cur_height);

                    let node2 = helium::Node::new("localhost", 4003);
                    let node3 = helium::Node::new("localhost", 4004);
                    let node4 = helium::Node::new("localhost", 4005);
                    let node5 = helium::Node::new("localhost", 4006);
                    let nodes: Vec<&helium::Node> = vec![&node1, &node2, &node3, &node4, &node5];

                    let mut all_accts: Vec<(helium::Account, &helium::Node)> = Vec::new();
                    for n in nodes {
                        for a in n.list_accounts().unwrap() {
                            println!("Account: {}\t\tBal: {}", a.address, a.balance);
                            all_accts.push((a, n));
                        }
                    }

                    for (i, (a, n)) in all_accts.iter().enumerate() {
                        if a.balance > 1 {
                            let to_acct = if all_accts.len() > i + 1 {
                                &all_accts[i + 1].0
                            } else {
                                &all_accts[0].0
                            };

                            let amt = a.balance / 2;

                            n.pay(&a.address, &to_acct.address, amt).unwrap();
                            println!("Paid {} from {} to {}", amt, &a.address, &to_acct.address);
                        }
                    }
                    last_height = cur_height;
                }

                Ok(())
            })
            .map_err(|e| println!("interval errored; err={:?}", e));

        tokio::run(task);
    }

    pub fn create_and_multiply(&self, interval: u64) {
        let node = helium::Node::new("localhost", 4001);

        let mut last_height = node.status().unwrap().chain_height;

        let interval = Duration::new(interval, 0);
        let task = Interval::new_interval(interval)
            .for_each(move |_| {
                println!("Checking...");
                let cur_height = node.status().unwrap().node_height; // want to make sure node is current
                if cur_height > last_height {
                    println!("New height: {}", cur_height);
                    node.list_accounts()
                        .unwrap()
                        .iter()
                        .filter(|&a| a.balance > 1000)
                        .for_each(|a| {
                            let amt = a.balance / 2;
                            let acct = node.create_account().unwrap();
                            node.pay(&a.address, &acct.address, amt).unwrap();
                            println!("Paid {} to {}", amt, acct.address);
                        });
                    last_height = cur_height;
                }
                Ok(())
            })
            .map_err(|e| println!("interval errored; err={:?}", e));

        tokio::run(task);
    }
}
