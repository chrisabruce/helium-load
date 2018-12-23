use helium;

use std::time::Duration;
use tokio::prelude::*;
use tokio::timer::Interval;

const POLL_INTERVAL: u64 = 10;

fn main() {
    let node = helium::Node::new("localhost", 4001);

    let mut last_height = node.status().unwrap().chain_height;

    let interval = Duration::new(POLL_INTERVAL, 0);
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
