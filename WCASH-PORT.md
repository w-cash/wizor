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
- TODO: sweep Vizor call sites to route through the boundary helpers under
  the feature. Production sites found so far: `keys.rs` (`ua.encode(&network)`
  at 271/378/467/506/534, `ufvk.encode` 654/1043, `address.encode` 1075),
  `sync/transactions.rs:216`, recipient parsing in `sync/mod.rs:509`
  (`ZcashAddress::try_from_encoded`). UFVK/UIVK encodings (`uview…`) keep the
  Zcash namespace for now — wolf does not define a Wcash namespace for them.
- TODO (Dart): QR display and `bech32.dart` address validation must accept
  the Wcash prefixes in the wcash flavor.

## Phase 2 — sync/RPC against a wcash node (TODO)

- Endpoint config: point the gRPC channel at a wcash `zebrad` with the
  lightwalletd interface enabled (no public Testnet infra exists — regtest via
  wolf's docker profiles, or a self-hosted node).
- Mirror `wcash-wallet`'s `WcashNamespaceService` if/when the transparent
  `GetAddressUtxos` path is used: it rewrites synchronizer-generated
  transparent address strings between the Zcash and Wcash namespaces at the
  transport boundary.
- Optional hardening from `AttestedWcashClient`: verify server identity /
  genesis before trusting a node.
- E2E: replace `scripts/regtest/` (zcashd+lightwalletd docker) with a wcash
  regtest stack (`wolf/docker`, `wcash-local.toml`), then run the existing
  regtest suites.

## Phase 3 — Dart flavor: product surface (TODO)

- Tickers/copy: ZEC → WEC (mainnet) / TWC (testing networks), 8 decimal
  places unchanged. `network_config.dart`, formatting, explorer config.
- Default network: `test` (Wcash testnet); hide/reject mainnet selection.
- Disable Zcash-only features for the flavor: swap (ZEC swap deposits),
  voting, payment links/gift cards (`link.vizor.cash`), Ironwood *migration*
  UI (Wcash has no legacy Orchard pool to migrate from — the chain launches
  Ironwood-native).
- Keystone hardware flow stays disabled for wcash: the device firmware
  computes Zcash-domain sighashes; Wcash-domain signing requires a firmware
  update. (`zakura-pczt` is patched so software PCZT paths still work.)
- Branding (name, icons, deep-link host) — product decision, not started.
- Build plumbing: Cargokit must pass `--features wcash` when the Dart flavor
  is selected (pair with a `VIZOR_CHAIN=wcash` dart-define, mirroring the
  `VIZOR_FORM_FACTOR` pattern).

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
