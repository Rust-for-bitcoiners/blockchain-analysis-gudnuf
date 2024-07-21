use std::error::Error;
#[allow(unused_imports, unused_variables)]
use std::{env, path::PathBuf, str::FromStr, time};

use bitcoincore_rpc::{
    bitcoin::{block, Network},
    Auth, Client, Error as BitcoinRpcError, RpcApi,
};
use chrono::{Duration, Utc};
#[macro_use]
extern crate lazy_static;

type Result<T> = std::result::Result<T, BitcoinRpcError>;

trait LoadCredentials {
    fn from_env() -> Self;
}

struct RpcCredentials {
    rpc_url: String,
    rpc_user: String,
    rpc_password: String,
}

impl LoadCredentials for RpcCredentials {
    fn from_env() -> Self {
        dotenv::dotenv().ok();

        let rpc_url: String = env::var("BITCOIN_RPC_URL").expect("BITCOIN_RPC_URL must be set");
        let rpc_user: String = env::var("BITCOIN_RPC_USER").expect("BITCOIN_RPC_USER must be set");
        let rpc_password: String =
            env::var("BITCOIN_RPC_PASSWORD").expect("BITCOIN_RPC_PASSWORD must be set");

        RpcCredentials {
            rpc_url,
            rpc_user,
            rpc_password,
        }
    }
}

struct RpcCookieCredentials {
    url: String,
    pathbuf: PathBuf,
}

impl LoadCredentials for RpcCookieCredentials {
    fn from_env() -> Self {
        dotenv::dotenv().ok();

        let cookie_path = env::var("COOKIE_FILE").expect("Cookie file not set");
        let url = env::var("BITCOIN_RPC_URL").expect("BITCOIN_RPC_URL not set");

        RpcCookieCredentials {
            pathbuf: PathBuf::from_str(&cookie_path).expect("Invalid cookie path"),
            url,
        }
    }
}

lazy_static! {
    static ref RPC_CLIENT: Client = {
        // const TIMEOUT_UTXO_SET_SCANS: time::Duration = time::Duration::from_secs(60 * 8); // 8 minutes
        // let RpcCredentials {
        //     rpc_url,
        //     rpc_user,
        //     rpc_password,
        // } = RpcCredentials::from_env();
        // let custom_timeout_transport = jsonrpc::simple_http::Builder::new()
        //     .url(&rpc_url)
        //     .expect("invalid rpc url")
        //     .auth(rpc_user, Some(rpc_password))
        //     .timeout(TIMEOUT_UTXO_SET_SCANS)
        //     .build();
        // let custom_timeout_rpc_client =
        //     jsonrpc::client::Client::with_transport(custom_timeout_transport);
        // Client::from_jsonrpc(custom_timeout_rpc_client)


        let creds = RpcCookieCredentials::from_env();
        match Client::new(&creds.url, Auth::CookieFile(creds.pathbuf)) {
            Ok(client) => client,
            Err(err) => {
                eprintln!("Error connecting to client: {:?}", err);
                panic!()
            }
        }
    };
}

fn get_block_by_height(block_height: u64) -> Result<block::Block> {
    let rpc = &*RPC_CLIENT;
    let block = rpc.get_block_hash(block_height)?;
    let block = rpc.get_block(&block)?;
    Ok(block)
}

fn get_block_time(block_height: u64) -> Result<Duration> {
    let block = get_block_by_height(block_height)?;
    Ok(Duration::seconds(block.header.time as i64))
}

/**
 * Attempts to find average block time of recent blocks
 */
fn avg_time_to_mine(block_height: u64) -> Result<Duration> {
    let num_blocks_in_epoch = block_height % 2016;

    let first_block_in_epoch = block_height - num_blocks_in_epoch;

    let total_diff = get_block_time(block_height)? - get_block_time(first_block_in_epoch)?;

    let avg_diff = total_diff.num_seconds() as u64 / num_blocks_in_epoch;

    Ok(Duration::seconds(avg_diff as i64))
}

pub fn time_to_mine(block_height: u64) -> Result<Duration> {
    Ok(get_block_time(block_height)? - get_block_time(block_height - 1)?)
}

/**
 * Attempts to use average time to mine a block to guess when the next block will be mined
 */
pub fn guess_time_to_mine_next_block() -> Result<Duration> {
    let rpc = &*RPC_CLIENT;
    let tip = rpc.get_block_count()?;
    let avg_time = avg_time_to_mine(tip)?;

    let now = Utc::now().timestamp();

    let time_to_mine =
        avg_time - Duration::seconds(now - get_block_time(tip)?.num_seconds() as i64);
    Ok(time_to_mine)
}

pub fn number_of_transactions(block_height: u64) -> Result<u16> {
    let block = get_block_by_height(block_height)?;
    Ok(block.txdata.len() as u16)
}

pub fn get_chain() -> Result<Network> {
    let rpc = &*RPC_CLIENT;
    let chain = rpc.get_blockchain_info()?;
    Ok(chain.chain)
}

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bitcoin-rpc-cli")]
#[command(version = "0.1.0")]
#[command(about = "A CLI for the Bitcoin RPC API", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Get the current chain")]
    Chain,
    #[command(about = "Get the it took to mine a block")]
    TimeToMine {
        #[arg(required = true, help = "(numeric, required) The height index")]
        block_height: u64,
    },
    #[command(about = "Get the number of transactions in a block")]
    NumberOfTransactions {
        #[arg(required = true, help = "(numeric, required) The height index")]
        block_height: u64,
    },
    #[command(about = "Guess how long until next block is mined")]
    NextBlock,
}

// QUESTION: is this the best way to make error handling happen in a single place?
// if command returns an error then return it to whatever called `call_command`
fn call_command(command: Commands) -> std::result::Result<(), Box<dyn Error>> {
    match command {
        Commands::Chain => {
            let chain = get_chain()?;
            println!("{}", chain);
        }
        Commands::TimeToMine { block_height } => {
            let time = time_to_mine(block_height)?;
            println!("{}s, {}min", time.num_seconds(), time.num_minutes());
        }
        Commands::NumberOfTransactions { block_height } => {
            let num = number_of_transactions(block_height)?;
            println!("{} transactions", num);
        }
        Commands::NextBlock => {
            println!("Next block will be mined in: ");
            let time = guess_time_to_mine_next_block()?;
            println!(
                "{}s, {}min, {}days",
                time.num_seconds(),
                time.num_minutes(),
                time.num_days()
            );
        }
    };
    Ok(())
}

fn main() {
    let cli: Cli = Cli::parse();

    if let Err(e) = match cli.command {
        Some(cmd) => call_command(cmd),
        None => {
            eprintln!("No command provided");
            Ok(())
        }
    } {
        eprintln!("Error: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_chain() {
        // QUESTION: how to test just that this returns instance of Network? Or is that redundant because of type safety. What would be a better test if so?
        let chain = get_chain().unwrap();
        match chain {
            Network::Bitcoin | Network::Regtest | Network::Signet | Network::Testnet => {
                println!("{}", chain);
            }
            _ => panic!("Unexpected network type"),
        }
    }

    #[test]
    fn test_time_to_mine() {
        let time = time_to_mine(24).unwrap();
        println!("{:?}", time);
        assert_eq!(time.num_seconds(), 666);
    }

    #[test]
    fn test_num_transactions() {
        let num = number_of_transactions(300_000).unwrap();
        println!("{}", num);
        assert_eq!(num, 237);
    }
}
