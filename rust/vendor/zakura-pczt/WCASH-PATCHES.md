# Wcash downstream patches

This directory vendors `zakura-pczt` 0.1.0-rc2 from crates.io (the Zakura fork
of upstream `pczt` 0.9.3 that the Vizor wallet builds on) because the crate
exhaustively matches the closed `zcash_protocol::BranchId` enum. The upstream
license files, changelog, original manifest, and README are retained beside
the source.

Upstream provenance:

- crates.io package: `zakura-pczt` 0.1.0-rc2

The minimal downstream patch mirrors wolf's `vendor/pczt` patch (w-cash/wolf,
see its `WCASH-PATCHES.md`): it treats `BranchId::WcashTestnetV1` and the
distinct `BranchId::WcashRegtestV1` as V6 transaction domains and permits V6
deferred-anchor updates for them. It does not change any Zcash branch behavior
or PCZT encoding.

Patched files:

- `src/roles/creator/mod.rs` — the Wcash branches map to
  `(V6_TX_VERSION, V6_VERSION_GROUP_ID)`;
- `src/roles/updater/mod.rs` — deferred-anchor updates accept the Wcash
  branches.
