#![allow(dead_code)]

use std::path::Path;

use bip39::Mnemonic;
use bls::SecretKey;
use bytes::Bytes;
use curv::elliptic::curves::ECScalar as _;
use libp2p::Multiaddr;
use rand::{Rng, RngCore as _};
use sn_peers_acquisition::parse_peer_addr;
use sn_transfers::{get_faucet_data_dir, HotWallet, MainSecretKey};

const MNEMONIC_FILENAME: &str = "account_secret";
const ACCOUNT_ROOT_XORNAME_DERIVATION: &str = "m/1/0";
const ACCOUNT_WALLET_DERIVATION: &str = "m/2/0";
const DEFAULT_WALLET_DERIVIATION_PASSPHRASE: &str = "default";

/// When launching a testnet locally, we can use the faucet wallet.
pub fn load_hot_wallet_from_faucet() -> HotWallet {
    let root_dir = get_faucet_data_dir();
    load_account_wallet_or_create_with_mnemonic(&root_dir, None)
        .expect("faucet wallet should be available for tests")
}

pub fn gen_random_data(len: usize) -> Bytes {
    let mut data = vec![0u8; len];
    rand::thread_rng().fill(&mut data[..]);
    Bytes::from(data)
}

/// Enable logging for tests. E.g. use `RUST_LOG=autonomi` to see logs.
pub fn enable_logging() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
}

/// Parse the `SAFE_PEERS` env var into a list of Multiaddrs.
///
/// An empty `Vec` will be returned if the env var is not set.
pub fn peers_from_env() -> Result<Vec<Multiaddr>, libp2p::multiaddr::Error> {
    let Ok(peers_str) = std::env::var("SAFE_PEERS") else {
        return Ok(vec![]);
    };

    peers_str.split(',').map(parse_peer_addr).collect()
}
/// Load a account from disk, with wallet, or create a new one using the mnemonic system
fn load_account_wallet_or_create_with_mnemonic(
    root_dir: &Path,
    derivation_passphrase: Option<&str>,
) -> Result<HotWallet, Box<dyn std::error::Error>> {
    let wallet = HotWallet::load_from(root_dir);

    match wallet {
        Ok(wallet) => Ok(wallet),
        Err(error) => {
            tracing::warn!("Issue loading wallet, creating a new one: {error}");

            let mnemonic = load_or_create_mnemonic(root_dir)?;
            let wallet =
                secret_key_from_mnemonic(mnemonic, derivation_passphrase.map(|v| v.to_owned()))?;

            Ok(HotWallet::create_from_key(root_dir, wallet, None)?)
        }
    }
}

fn load_or_create_mnemonic(root_dir: &Path) -> Result<Mnemonic, Box<dyn std::error::Error>> {
    match read_mnemonic_from_disk(root_dir) {
        Ok(mnemonic) => {
            tracing::info!("Using existing mnemonic from {root_dir:?}");
            Ok(mnemonic)
        }
        Err(error) => {
            tracing::warn!("No existing mnemonic found in {root_dir:?}, creating new one. Error was: {error:?}");
            let mnemonic = random_eip2333_mnemonic()?;
            write_mnemonic_to_disk(root_dir, &mnemonic)?;
            Ok(mnemonic)
        }
    }
}

fn secret_key_from_mnemonic(
    mnemonic: Mnemonic,
    derivation_passphrase: Option<String>,
) -> Result<MainSecretKey, Box<dyn std::error::Error>> {
    let passphrase =
        derivation_passphrase.unwrap_or(DEFAULT_WALLET_DERIVIATION_PASSPHRASE.to_owned());
    account_wallet_secret_key(mnemonic, &passphrase)
}

fn create_faucet_account_and_wallet() -> HotWallet {
    let root_dir = get_faucet_data_dir();

    println!("Loading faucet wallet... {root_dir:#?}");
    load_account_wallet_or_create_with_mnemonic(&root_dir, None)
        .expect("Faucet wallet shall be created successfully.")
}

pub fn write_mnemonic_to_disk(
    files_dir: &Path,
    mnemonic: &bip39::Mnemonic,
) -> Result<(), Box<dyn std::error::Error>> {
    let filename = files_dir.join(MNEMONIC_FILENAME);
    let content = mnemonic.to_string();
    std::fs::write(filename, content)?;
    Ok(())
}

pub(super) fn read_mnemonic_from_disk(
    files_dir: &Path,
) -> Result<bip39::Mnemonic, Box<dyn std::error::Error>> {
    let filename = files_dir.join(MNEMONIC_FILENAME);
    let content = std::fs::read_to_string(filename)?;
    let mnemonic = bip39::Mnemonic::parse_normalized(&content)?;
    Ok(mnemonic)
}

fn random_eip2333_mnemonic() -> Result<bip39::Mnemonic, Box<dyn std::error::Error>> {
    let mut entropy = [1u8; 32];
    let rng = &mut rand::rngs::OsRng;
    rng.fill_bytes(&mut entropy);
    let mnemonic = bip39::Mnemonic::from_entropy(&entropy)?;
    Ok(mnemonic)
}

/// Derive a wallet secret key from the mnemonic for the account.
fn account_wallet_secret_key(
    mnemonic: bip39::Mnemonic,
    passphrase: &str,
) -> Result<MainSecretKey, Box<dyn std::error::Error>> {
    let seed = mnemonic.to_seed(passphrase);

    let root_sk = eip2333::derive_master_sk(&seed)?;
    let derived_key = eip2333::derive_child_sk(root_sk, ACCOUNT_WALLET_DERIVATION);
    let key_bytes = derived_key.serialize();
    let sk = SecretKey::from_bytes(key_bytes.into())?;
    Ok(MainSecretKey::new(sk))
}
