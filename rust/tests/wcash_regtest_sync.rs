//! End-to-end check of the Wcash port against a local wcash-zebrad regtest
//! node (w-cash/wolf).
//!
//! Node recipe (see WCASH-PORT.md phase 2):
//!
//! 1. In the wolf checkout, build the Wcash consensus binary with the
//!    internal miner:
//!    `cargo build --locked --release -p zebrad --bin zebrad \
//!       --no-default-features --features wcash-consensus,internal-miner \
//!       --target-dir target/wcash-profile-builds/wcash`
//! 2. Print this wallet's deterministic miner address:
//!    `cargo test --features wcash --test wcash_regtest_sync \
//!       -- --ignored --nocapture wcash_regtest_derive_miner_address`
//! 3. Start the node with a loopback config whose `[rpc]` block sets
//!    `lightwalletd_listen_addr` (default here: 127.0.0.1:58234) and whose
//!    `[mining]` block pays `internal_miner = true` blocks to the printed
//!    `wuregtest1…` address (private Ironwood coinbase).
//! 4. Run the sync test:
//!    `cargo test --features wcash --test wcash_regtest_sync \
//!       -- --ignored --nocapture wcash_regtest_sync_detects_ironwood_coinbase`
//!
//! `WCASH_E2E_LIGHTWALLETD_URL` overrides the gRPC endpoint.

#![cfg(feature = "wcash")]

use std::path::Path;
use std::time::{Duration, Instant};

use rust_lib_zcash_wallet::api::{sync as sync_api, wallet as wallet_api};

const WCASH_REGTEST_NETWORK: &str = "regtest";

/// Public test-only mnemonic (BIP-39 vector). Never carries value.
const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon \
     abandon abandon abandon about";

/// Second public BIP-39 test vector, used as the send recipient.
const RECIPIENT_MNEMONIC: &str =
    "legal winner thank year wave sausage worth useful legal winner thank yellow";

fn lightwalletd_url() -> String {
    std::env::var("WCASH_E2E_LIGHTWALLETD_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:58234".to_string())
}

fn path_str(path: &Path) -> String {
    path.to_str().expect("temp db path is valid UTF-8").into()
}

fn import_wallet_with(db_path: &Path, mnemonic: &str, name: &str) -> wallet_api::WalletImportResult {
    wallet_api::import_wallet(
        mnemonic.into(),
        String::new(),
        Some(1),
        WCASH_REGTEST_NETWORK.into(),
        path_str(db_path),
        Some(name.into()),
    )
    .expect("import_wallet")
}

fn import_test_wallet(db_path: &Path) -> wallet_api::WalletImportResult {
    import_wallet_with(db_path, TEST_MNEMONIC, "Wcash Regtest")
}

fn wait_for_height(url: &str, min_height: u64, timeout: Duration) -> u64 {
    let deadline = Instant::now() + timeout;
    loop {
        match wallet_api::get_latest_block_height(url.to_string(), WCASH_REGTEST_NETWORK.into()) {
            Ok(height) if height >= min_height => return height,
            Ok(height) => println!("waiting for height {min_height}, tip={height}"),
            Err(e) => println!("waiting for node at {url}: {e}"),
        }
        assert!(
            Instant::now() < deadline,
            "node at {url} did not reach height {min_height} in time"
        );
        std::thread::sleep(Duration::from_secs(2));
    }
}

fn sync_wallet(db_path: &Path, url: &str) {
    sync_api::run_full_sync_blocking(
        path_str(db_path),
        url.to_string(),
        WCASH_REGTEST_NETWORK.into(),
        1,
    )
    .expect("run_full_sync_blocking");
}

fn balance(db_path: &Path, account_uuid: &str) -> sync_api::WalletBalance {
    sync_api::get_balance(
        path_str(db_path),
        WCASH_REGTEST_NETWORK.into(),
        account_uuid.to_string(),
    )
    .expect("get_balance")
}

/// Prints the deterministic address the node's `[mining] miner_address`
/// should pay. Offline: needs no running node.
#[test]
#[ignore = "prints the miner address for the wcash regtest node config"]
fn wcash_regtest_derive_miner_address() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let wallet = import_test_wallet(&tempdir.path().join("wallet.db"));

    println!("miner_address = \"{}\"", wallet.unified_address);
    assert!(
        wallet.unified_address.starts_with("wuregtest1"),
        "wcash flavor must derive a Wcash regtest UA, got {}",
        wallet.unified_address
    );
}

/// Syncs against the mining node and asserts that Wcash-domain V6 Ironwood
/// coinbase notes paid to this wallet are detected.
#[test]
#[ignore = "requires a local wcash-zebrad regtest node with the internal miner"]
fn wcash_regtest_sync_detects_ironwood_coinbase() {
    let url = lightwalletd_url();
    let tip = wait_for_height(&url, 3, Duration::from_secs(120));
    println!("chain tip at start: {tip}");

    let tempdir = tempfile::tempdir().expect("tempdir");
    let db_path = tempdir.path().join("zcash_wallet.db");
    let wallet = import_test_wallet(&db_path);
    println!("wallet ua={}", wallet.unified_address);

    sync_wallet(&db_path, &url);

    let balance = balance(&db_path, &wallet.account_uuid);
    println!(
        "post-sync balance total={} spendable={} ironwood={} orchard={} sapling={} transparent={}",
        balance.total,
        balance.spendable,
        balance.ironwood,
        balance.orchard,
        balance.sapling,
        balance.transparent
    );

    // Every mined block pays 6.25 TWC (625_000_000 zatoshi) to this wallet's
    // Ironwood receiver. At least one coinbase must have been scanned.
    assert!(
        balance.total >= 625_000_000,
        "expected at least one Ironwood coinbase (6.25 TWC), got total={}",
        balance.total
    );
    assert_eq!(balance.sapling, 0, "Wcash has no Sapling pool");

    let history = sync_api::get_transaction_history(
        path_str(&db_path),
        WCASH_REGTEST_NETWORK.into(),
        None,
        wallet.account_uuid.clone(),
    )
    .expect("get_transaction_history");
    assert!(
        history.iter().any(|tx| tx.account_balance_delta > 0),
        "expected a positive coinbase receive in transaction history"
    );
}

/// Full send round trip over the node: the mined wallet proposes and
/// broadcasts a Wcash-domain V6 Ironwood transfer, the internal miner mines
/// it, and the recipient wallet detects the note. Consensus acceptance of the
/// broadcast is the definitive check of the Wcash signature-domain port.
#[test]
#[ignore = "requires a local wcash-zebrad regtest node with the internal miner"]
fn wcash_regtest_send_between_wallets() {
    let url = lightwalletd_url();
    // Sender funds mature for spending after the untrusted confirmation depth.
    wait_for_height(&url, 8, Duration::from_secs(240));

    let sender_dir = tempfile::tempdir().expect("tempdir");
    let sender_db = sender_dir.path().join("zcash_wallet.db");
    let sender = import_test_wallet(&sender_db);

    let recipient_dir = tempfile::tempdir().expect("tempdir");
    let recipient_db = recipient_dir.path().join("zcash_wallet.db");
    let recipient = import_wallet_with(&recipient_db, RECIPIENT_MNEMONIC, "Wcash Recipient");
    println!("recipient ua={}", recipient.unified_address);
    assert!(recipient.unified_address.starts_with("wuregtest1"));

    sync_wallet(&sender_db, &url);
    let sender_before = balance(&sender_db, &sender.account_uuid);
    assert!(
        sender_before.spendable >= 200_000_000,
        "sender needs at least 2 TWC spendable, got {}",
        sender_before.spendable
    );

    let send_flow_id = "wcash-regtest-send";
    let proposal = sync_api::propose_send(
        path_str(&sender_db),
        WCASH_REGTEST_NETWORK.into(),
        sender.account_uuid.clone(),
        send_flow_id.into(),
        recipient.unified_address.clone(),
        100_000_000,
        Some("wcash e2e".into()),
    )
    .expect("propose_send");
    assert!(
        !proposal.needs_sapling_params,
        "an Ironwood-only Wcash transfer must not require Sapling params"
    );
    println!(
        "proposal id={} fee={}",
        proposal.proposal_id, proposal.fee_zatoshi
    );

    let broadcast_tip =
        wallet_api::get_latest_block_height(url.clone(), WCASH_REGTEST_NETWORK.into())
            .expect("tip before broadcast");
    let result = sync_api::execute_proposal(
        path_str(&sender_db),
        url.clone(),
        proposal.proposal_id,
        send_flow_id.into(),
        TEST_MNEMONIC.as_bytes().to_vec(),
        None,
        None,
    )
    .expect("execute_proposal: the node must accept the Wcash-domain V6 transaction");
    println!("broadcast txids={:?}", result.txids);
    assert!(!result.txids.is_empty());

    // Wait for the transfer to be mined; the internal miner may seal a
    // template built before the broadcast, so keep re-syncing the recipient
    // until the note lands (bounded).
    wait_for_height(&url, broadcast_tip + 2, Duration::from_secs(240));
    let deadline = Instant::now() + Duration::from_secs(180);
    let recipient_after = loop {
        sync_wallet(&recipient_db, &url);
        let recipient_after = balance(&recipient_db, &recipient.account_uuid);
        println!(
            "recipient balance total={} ironwood={}",
            recipient_after.total, recipient_after.ironwood
        );
        if recipient_after.total >= 100_000_000 {
            break recipient_after;
        }
        assert!(
            Instant::now() < deadline,
            "recipient should have received 1 TWC, got total={}",
            recipient_after.total
        );
        std::thread::sleep(Duration::from_secs(3));
    };
    assert!(recipient_after.ironwood >= 100_000_000);
    sync_wallet(&sender_db, &url);

    let sender_history = sync_api::get_transaction_history(
        path_str(&sender_db),
        WCASH_REGTEST_NETWORK.into(),
        None,
        sender.account_uuid.clone(),
    )
    .expect("get_transaction_history");
    assert!(
        sender_history.iter().any(|tx| tx.account_balance_delta < 0),
        "sender history should record the outbound transfer"
    );
}

/// Probes the public Wcash engineering Testnet lightwalletd front through the
/// wallet's own gRPC stack (tonic + TLS + webpki roots).
#[test]
#[ignore = "requires internet access to wallet-testnet.wcashexplorer.com"]
fn wcash_public_testnet_tip_is_reachable() {
    let url = std::env::var("WCASH_E2E_TESTNET_URL")
        .unwrap_or_else(|_| "https://wallet-testnet.wcashexplorer.com:443".to_string());
    let height = wallet_api::get_latest_block_height(url.clone(), "test".into())
        .expect("the public Wcash testnet lightwalletd must answer GetLatestBlock");
    println!("public wcash testnet tip via {url}: {height}");
    assert!(height > 0, "testnet tip should be past genesis");
}
