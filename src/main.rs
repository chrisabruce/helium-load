use helium;

use rand::prelude::*;
use std::time::Duration;
use tokio::prelude::*;
use tokio::timer::Interval;

use clap::{App, Arg};

const POLL_INTERVAL: u64 = 10;
const PAY_INTERVAL: u64 = 60;

fn main() {
    let matches = App::new("Helium Load")
        .version("1.0")
        .author("Chris Bruce <chris@helium.com>")
        .about("Provides various options for load testing helium blockchain.")
        .arg(
            Arg::with_name("formula")
                .short("f")
                .long("formula")
                .value_name("FORMULA")
                .help("Which load formula to run: ping | pong | multiply")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("trigger")
            .short("t")
            .long("trigger")
            .value_name("TRIGGER")
            .help("Select a trigger type, either on new block or on timed interval. (block | interval)")
            .takes_value(true),
        )
        .arg(
            Arg::with_name("interval")
            .short("i")
            .long("interval")
            .value_name("INTERVAL")
            .help("Sets the poll interval for checking block height or triggering the formula.")
            .takes_value(true),
        )
        .get_matches();

    let formula = matches.value_of("formula").unwrap_or("ping");
    println!("Value for formula: {}", formula);

    let trigger = matches.value_of("trigger").unwrap_or("block");
    println!("Value for trigger: {}", trigger);

    match formula {
        "multiply" => create_and_multiply(),
        "pong" => pong(),
        _ => ping(),
    };
}

fn pong() {
    let node = helium::Node::new("localhost", 4001);
    let accounts = node.list_accounts().unwrap();

    print!("Found {} account(s).\n", accounts.len());
    if accounts.len() < 1 {
        panic!("Requires two existing accounts.");
    }

    let interval = Duration::new(PAY_INTERVAL, 0);
    let task = Interval::new_interval(interval)
        .for_each(move |_| {
            let mut rng = rand::thread_rng();
            let amt: u64 = rng.gen_range(10_000, 100_000);

            print!("Paying: {}\n", amt);
            node.pay(&accounts[0].address, &accounts[1].address, amt)
                .unwrap();

            node.pay(&accounts[1].address, &accounts[0].address, amt)
                .unwrap();

            Ok(())
        })
        .map_err(|e| print!("interval errored; err={:?}\n", e));

    tokio::run(task);
}

fn ping() {
    let node = helium::Node::new("localhost", 4001);
    let accounts = node.list_accounts().unwrap();

    print!("Found {} account(s).\n", accounts.len());
    if accounts.len() < 1 {
        panic!("Requires two existing accounts.");
    }

    let mut last_height = node.status().unwrap().chain_height;

    let interval = Duration::new(POLL_INTERVAL, 0);
    let task = Interval::new_interval(interval)
        .for_each(move |_| {
            println!("Checking...");
            let cur_height = node.status().unwrap().node_height; // want to make sure node is current
            if cur_height > last_height {
                println!("New height: {}", cur_height);
                let mut rng = rand::thread_rng();
                let amt: u64 = rng.gen_range(10_000, 100_000);

                print!("Paying: {}\n", amt);
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

fn create_and_multiply() {
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
