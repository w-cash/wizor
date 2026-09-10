# Wcash port

Adapting Vizor (a Zcash wallet: Flutter + Rust via `flutter_rust_bridge`,
`librustzcash`/Zakura crates, lightwalletd gRPC sync) to the Wcash chain
([w-cash/wolf](https://github.com/w-cash/wolf) — a Zebra fork: AuxPoW chain
with Zcash NU6.3 / Ironwood protocol semantics under chain-specific
transaction domains).

Local wolf checkout used as reference: `/Users/vladyslav/rust/blockchain/wolf`.

## Why this is tractable

- Wcash **is** NU6.3/Ironwood semantics under different ZIP-200 domains:
  `BranchId::WcashTestnetV1` (`0xb3cfd27e`) and `BranchId::WcashRegtestV1`
  (`0xc3a6678a`). Same pools (transparent + Ironwood; Sprout/Sapling/legacy
  Orchard rejected), same zatoshi precision, V6-only transactions.
- Vizor's crypto stack (Zakura forks) and wolf's patched upstream crates share
  the same `zcash_primitives 0.30.1` / `zcash_protocol 0.10.x` lineage; wolf's
  consensus patches are small and additive, so they mirror cleanly onto the
  Zakura forks.
- wolf's `zebrad` serves the lightwalletd `CompactTxStreamer` gRPC natively
  (`zebra-rpc/src/lightwalletd/`), so Vizor's sync engine can point at a wcash
  node without a separate lightwalletd.
- wolf's `wcash-wallet` crate is the reference implementation for the wallet
  boundary: network params, branch IDs, Wcash address namespace, fail-closed
  RPC.

## Build flavor

The port is a **build-time flavor**, not extra networks in one binary
(precedent: the existing `ironwood_masquerade` cfg):

- Rust: cargo feature `wcash` in `rust/Cargo.toml`.
- `WalletNetwork::Test` → Wcash engineering Testnet,
  `WalletNetwork::Regtest` → Wcash local Regtest, `"main"` is rejected
  (Wcash mainnet is disabled upstream).
- Wcash networks activate the cumulative shielded upgrade set at height 1 and
  select the Wcash branch IDs for NU6.3
  (`rust/src/wallet/network.rs`).

## Phase 0 — consensus core (DONE on this branch)

- `rust/vendor/zcash_protocol` — wolf's patched upstream 0.10.5 (adds the two
  Wcash `BranchId`s + `Parameters::branch_id_for_upgrade`), copied verbatim.
- `rust/vendor/zakura-primitives` — `zakura-primitives` 1.0.0 with wolf's
  primitives patch mirrored (V6-only Wcash domains; see `WCASH-PATCHES.md`
  inside).
- `rust/vendor/zakura-pczt` — `zakura-pczt` 0.1.0-rc2 with wolf's pczt patch
  mirrored (Wcash branches are V6 PCZT domains).
- `[patch.crates-io]` entries in `rust/Cargo.toml` wire the vendored crates
  into the whole graph (including inside `zakura-client-backend` /
  `zakura-client-sqlite`).
- Feature `wcash` + Wcash `Parameters` impl and tests in
  `rust/src/wallet/network.rs`.

Effect: with `--features wcash`, every transaction Vizor builds or parses on
Test/Regtest uses the Wcash signature/txid domain (selected generically via
`BranchId::for_height` inside `zcash_client_backend`'s builder), and Zcash
NU6.3 transactions are cross-rejected.

## Phase 1 — Wcash address namespace (codec DONE, sweep TODO)

Wcash rejects Zcash textual addresses; its own encodings use e.g. `wu…` /
`wutest…` / `wuregtest…` HRPs for Unified Addresses and Wcash-specific
transparent Base58 prefixes.

- DONE: `rust/src/wallet/wcash_address.rs` — port of
  `zebra-chain/src/primitives/wcash_address.rs` (gated on
  `#[cfg(feature = "wcash")]`, golden vectors included), plus boundary
  helpers mirroring `wolf/wcash-wallet/src/address.rs`:
  `encode_wallet_unified_address`, `encode_wallet_transparent_address`,
  `decode_wallet_address` (fail-closed: rejects the Zcash namespace and
  cross-network Wcash addresses).
- DONE: `rust/src/wallet/address_codec.rs` — the chain-flavor codec facade
  (`encode_unified_address`, `encode_transparent_address`,
  `parse_recipient`), and the call-site sweep through it: every UA/transparent
  display+derive encode in `keys.rs`, `sync/transactions.rs` (including the
  `TransactionsInvolvingAddress` enhancement request string), recipient
  parsing in `send.rs` (`build_send_request`, `build_send_max_proposal`), and
  a Wcash variant of `validate_address` in `sync/mod.rs`. Under the feature,
  `shielded_address_request()` is Orchard/Ironwood-only (Wcash UAs reject
  Sapling receivers).
- Still Zcash-namespace by design/for now:
  - UFVK/UIVK encodings (`uview…`) — wolf defines no Wcash namespace for
    viewing keys (`keys.rs:654/1043`);
  - librustzcash-internal transparent address strings on the
    `GetAddressUtxos` RPC path (`sync_engine/mod.rs`, transparent receive
    cache) — needs the phase-2 transport-boundary rewrite that
    `wcash-wallet`'s `WcashNamespaceService` implements.
- TODO (Dart): QR display and `bech32.dart` address validation must accept
  the Wcash prefixes in the wcash flavor; payment URIs are parsed on the
  Dart side and need the same treatment.
- Test lanes: the default lane (`cargo test --lib`) stays the full-suite
  regression gate. The wcash lane currently runs the targeted filters
  (`cargo test --features wcash --lib wcash` and `… address_codec`); most
  legacy tests assert Zcash encodings/`Main` and are not yet wcash-aware.

## Phase 2 — sync/RPC against a wcash node (in progress)

- DONE: transparent RPC namespace at the source instead of a transport
  rewriter. Unlike `wcash-wallet` (whose sync lives inside
  `zcash_client_backend` and needs `WcashNamespaceService` on the wire),
  Vizor's sync engine is custom, so every RPC-facing address string is owned
  by us and now goes through `address_codec`:
  `transparent_address_for_query` re-encodes DB-cached Zcash strings
  (`reencode_cached_transparent_address`), the non-external receiver list and
  the enhancement `TransactionsInvolvingAddress` filter encode typed
  receivers directly. UTXO replies are consumed via txid/script only — no
  response rewriting needed.
- DONE: E2E harness against a real wcash node:
  - `rust/tests/wcash-regtest-node.toml` — loopback wcash-zebrad regtest
    profile (RPC 58232, lightwalletd gRPC 58234, `internal_miner = true`
    paying every coinbase privately to the test wallet's Ironwood receiver).
  - `rust/tests/wcash_regtest_sync.rs` — `#[ignore]`d tests: derive the
    deterministic miner UA; sync against the node and assert the Ironwood
    coinbase balance and history are detected. The node build recipe is in
    the test header (wolf: `--features wcash-consensus,internal-miner`).
- macOS note: if C++ deps (rocksdb, zcash_script) fail with
  `'algorithm' file not found`, the CommandLineTools libc++ headers are
  missing; build with
  `CXXFLAGS="-isystem $(xcrun --show-sdk-path)/usr/include/c++/v1"`
  (proper fix: reinstall CLT).
- VERIFIED (2026-09-10, local run): against a wcash-zebrad regtest node with
  the internal miner (genesis `70bf0bab…`),
  `wcash_regtest_sync_detects_ironwood_coinbase` synced to tip and detected
  16 private Ironwood coinbases (100 TWC total, Sapling/transparent 0), and
  `wcash_regtest_send_between_wallets` completed a full transfer round trip:
  propose (fee 10000 zat) → sign in the Wcash sighash domain → broadcast
  accepted by the node → mined into the active chain (verified via
  `getrawtransaction … in_active_chain: true`) → recipient wallet detected
  the 1 TWC Ironwood note; sender history shows the outbound transfer.
- TODO: optional node hardening from `AttestedWcashClient` (verify server
  identity/genesis before trusting a node); wire the endpoint into the Dart
  settings once phase 3 starts.

## Phase 3 — Dart flavor: product surface (core DONE)

The flavor knob is `--dart-define=VIZOR_CHAIN=wcash` (`kWcashChain` in
`lib/src/core/config/network_config.dart`), paired with the `VIZOR_CHAIN`
process environment variable that Cargokit reads to add `--features wcash`
to the Rust build (`rust_builder/cargokit/build_tool/lib/src/builder.dart`,
same pattern as the existing `VIZOR_RUST_TOOLCHAIN` hook). App bootstrap
calls the new sync FRB API `is_wcash_build()` and refuses to run a
mixed-flavor binary (`lib/app.dart`).

- DONE `network_config.dart`: WEC/TWC tickers, `wu…`/`wutest…`/W* address
  prefixes (`tAddrPrefixes` now carries both Base58 prefixes per network),
  Sapling declared unsupported (`supportsSaplingRecipients`), localhost
  lightwalletd defaults (`127.0.0.1:58234`, matching
  `rust/tests/wcash-regtest-node.toml` — Wcash has no public infra),
  `saplingActivationHeight = 1`, `"main"` normalizes to `test` (mainnet is
  disabled upstream), and wcash secure stores live under
  `com.keplr.vizor.wcash.<network>.secure_store` so they can never collide
  with Zcash wallets.
- DONE address book validation flows from the config (accepts Wcash UAs,
  W-prefix transparent, rejects Zcash encodings and all Sapling recipients).
- DONE hardcoded `'ZEC'` unit labels in send/pay/receive amount fields now
  use `kZcashDefaultCurrencyTicker` (donation/voting/gift-card literals left:
  those features are Zcash-only and gated off below).
- DONE feature gates under `kWcashChain`: coinholder voting
  (`votingHomeEntryVisibleProvider` + desktop sidebar item), Keystone
  onboarding entries (desktop welcome + mobile method selection; the device
  firmware signs Zcash-domain sighashes), payment links
  (`VizorPaymentLink.supportsNetwork`). Swap is already mainnet-only, so it
  is off by construction.
- Test lanes: `wcash` tag in `dart_test.yaml` mirroring the mobile lane;
  `test/core/config/wcash_chain_flavor_test.dart` runs via
  `fvm flutter test --tags wcash --run-skipped --dart-define=VIZOR_CHAIN=wcash`.
- TODO: mobile voting deep entries beyond the home card, Ironwood-migration
  surfaces (dormant on wcash — no legacy Orchard balance can exist — but not
  explicitly gated), explorer default (still CipherScan; no Wcash explorer
  exists), branding (name/icons/deep-link host), donation flow.
- Verification ceiling on this machine: `fvm flutter analyze` + full Dart
  suite + the wcash test lane. No Xcode / Android SDK is installed, so an
  actual app build (and therefore the Cargokit `VIZOR_CHAIN` hook) has not
  been exercised end-to-end; run
  `export VIZOR_CHAIN=wcash && fvm flutter run --dart-define=VIZOR_CHAIN=wcash --dart-define=ZCASH_DEFAULT_NETWORK=regtest`
  on a machine with the platform toolchains.

## Open questions

- **Seed domain separation.** `wcash-wallet` derives its seed through a
  Wcash-domain KDF so a Wcash wallet can never share keys with a Zcash wallet.
  Vizor uses plain BIP-39; under `NetworkType::Test` the coin type is the
  shared testnet one, so the same mnemonic yields the same keys as a Zcash
  testnet wallet. Decide whether to keep BIP-39 UX or add domain separation
  before any real-value network exists.
- **Vendored-crate maintenance.** Three vendored crates must track their
  upstreams (Zakura releases + wolf patch revisions). Wolf freezes its
  Testnet profile, so churn should be low.
- **`configure_regtest_nu6_3_activation_height`** is a no-op under the wcash
  feature (Wcash regtest activates NU6.3 at height 1); the Zcash regtest E2E
  machinery that uses it doesn't apply to the flavor.

## Verification so far

- `cargo check` / targeted tests: see branch history.
- Consensus-domain unit tests: `rust/src/wallet/network.rs::wcash_tests`
  (`cargo test --features wcash wcash_tests`).
