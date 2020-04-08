mod loader;

use dotenv::dotenv;
use loader::Load;
use std::env;

use clap::{App, Arg};

fn main() {
    dotenv().ok();

    let matches = App::new("Helium Load")
        .version("1.0")
        .author("Chris Bruce <chris@helium.com>")
        .about("Provides various options for load testing helium blockchain.")
        .arg(
            Arg::with_name("formula")
                .short("f")
                .long("formula")
                .value_name("FORMULA")
                .help("Which load formula to run: ping | pong | multiply | multiping")
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
            .help("Sets the poll interval in seconds for checking block height or triggering the formula.")
            .takes_value(true),
        )
        .arg(
            Arg::with_name("accounts")
            .short("a")
            .long("accounts")
            .value_name("NUM_ACCOUNTS")
            .help("Sets the number of accounts to use for the specified formula.  Defaults to 2.")
            .takes_value(true),
        )
        .get_matches();

    let formula = matches.value_of("formula").unwrap_or("ping");
    println!("Value for formula: {}", formula);

    let trigger = matches.value_of("trigger").unwrap_or("block");
    println!("Value for trigger: {}", trigger);

    let poll_interval = matches
        .value_of("interval")
        .unwrap_or("10")
        .parse::<u64>()
        .unwrap();
    println!("Value for polling interval: {}", poll_interval);

    let account_num = matches
        .value_of("accounts")
        .unwrap_or("10")
        .parse::<u64>()
        .unwrap();
    println!("Value for account number: {}", account_num);

    let api_url = env::var("API_URL").expect("Missing API_URL env var.");
    let password = env::var("PASSWORD").expect("Missing PASSWORD env var.");
    let load = Load::new(&api_url, &password);

    let _ = match formula {
        _ => load.start_pong(poll_interval),
    };
}
