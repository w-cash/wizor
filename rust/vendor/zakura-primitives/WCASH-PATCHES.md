# Wcash downstream patches

This directory vendors `zakura-primitives` 1.0.0 from crates.io (the Zakura
fork of upstream `zcash_primitives` 0.30.1 that the Vizor wallet builds on).
The upstream license files, changelog, original manifest, and README are
retained beside the source.

Upstream provenance:

- crates.io package: `zakura-primitives` 1.0.0
- crates.io archive SHA-256:
  `5a5c71f57ec127e0429928795354ec3971fc58151603657a4a6f1ffd9f4e0d67`

The downstream changes mirror wolf's `vendor/zcash_primitives` patch
(w-cash/wolf, see its `WCASH-PATCHES.md`): they teach the transaction
implementation about `BranchId::WcashTestnetV1` and `BranchId::WcashRegtestV1`
from the adjacent vendored `zcash_protocol` crate:

- both Wcash branches use NU6.3 / Ironwood bundle and circuit semantics;
- only transaction version 6 is valid for the Wcash branches, so every
  post-genesis transaction embeds its chain-specific branch ID on the wire;
- transaction hashes and signature hashes use the Wcash branch ID and
  therefore differ from otherwise identical Zcash NU6.3 transactions.

Patched files (all hunks are additive `BranchId::WcashTestnetV1 |
BranchId::WcashRegtestV1` arms next to the existing `Nu6_3` arms):

- `src/transaction/mod.rs` — `TxVersion::suggested_for_branch`,
  `TxVersion::valid_in_branch`, and the proptest version strategy;
- `src/transaction/builder.rs` — the `ironwood_branch` gate in
  `BuildConfig::ironwood_builder`.

Wolf's `Builder::new_with_branch_id` checked explicit-domain constructor is
intentionally NOT mirrored: Vizor constructs transactions only through
`zcash_client_backend`, which uses `Builder::new` and therefore selects the
branch from the wallet's consensus parameters
(`WalletNetwork::branch_id_for_upgrade` in `src/wallet/network.rs`).

Standard Zcash branch mappings and transaction-version rules are unchanged.
