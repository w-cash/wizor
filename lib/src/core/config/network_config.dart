/// Chain flavor selector. `--dart-define=VIZOR_CHAIN=wcash` builds the app
/// for the Wcash chain (w-cash/wolf) instead of Zcash. The Rust layer must be
/// compiled with the matching `wcash` cargo feature (Cargokit reads the
/// `VIZOR_CHAIN` process environment variable); app bootstrap asserts both
/// sides agree. See WCASH-PORT.md.
const kVizorChainEnvKey = 'VIZOR_CHAIN';
const kVizorChainRaw = String.fromEnvironment(
  kVizorChainEnvKey,
  defaultValue: 'zcash',
);
const bool kWcashChain = kVizorChainRaw == 'wcash';

enum ZcashNetwork {
  mainnet,
  testnet,
  regtest;

  String get name => switch (this) {
    mainnet => 'main',
    testnet => 'test',
    regtest => 'regtest',
  };

  int get coinType => switch (this) {
    mainnet => 133,
    testnet => 1,
    regtest => 1,
  };

  /// The primary P2PKH prefix. Wcash uses its own Base58 namespace
  /// (`W1…`/`WT…`/`WR…`) so a Zcash t-addr can never be pasted into a Wcash
  /// wallet by accident.
  String get tAddrPrefix => kWcashChain
      ? switch (this) {
          mainnet => 'W1',
          testnet => 'WT',
          regtest => 'WR',
        }
      : switch (this) {
          mainnet => 't1',
          testnet => 'tm',
          regtest => 'tm',
        };

  /// All transparent Base58Check prefixes (P2PKH + P2SH) for this network.
  List<String> get tAddrPrefixes => kWcashChain
      ? switch (this) {
          mainnet => const ['W1', 'W3'],
          testnet => const ['WT', 'WU'],
          regtest => const ['WR', 'WS'],
        }
      : switch (this) {
          mainnet => const ['t1', 't3'],
          testnet || regtest => const ['tm', 't2'],
        };

  /// Wcash permanently reserves its Sapling namespaces but never activates
  /// the pool, so these prefixes are display/rejection metadata only there.
  String get saplingPrefix => kWcashChain
      ? switch (this) {
          mainnet => 'ws',
          testnet => 'wtestsapling',
          regtest => 'wregtestsapling',
        }
      : switch (this) {
          mainnet => 'zs',
          testnet => 'ztestsapling',
          regtest => 'zregtestsapling',
        };

  /// Whether this chain can receive to Sapling addresses at all.
  bool get supportsSaplingRecipients => !kWcashChain;

  String get uaPrefix => kWcashChain
      ? switch (this) {
          mainnet => 'wu1',
          testnet => 'wutest1',
          regtest => 'wuregtest1',
        }
      : switch (this) {
          mainnet => 'u1',
          testnet => 'utest1',
          regtest => 'uregtest1',
        };

  String get texPrefix => kWcashChain
      ? switch (this) {
          mainnet => 'wtex1',
          testnet => 'wtextest1',
          regtest => 'wtexregtest1',
        }
      : switch (this) {
          mainnet => 'tex1',
          testnet => 'textest1',
          regtest => 'texregtest1',
        };

  int get defaultPort => kWcashChain
      ? 58234
      : switch (this) {
          mainnet => 9067,
          testnet => 18232,
          regtest => 9067,
        };

  /// Wcash has no public node infrastructure yet (the engineering Testnet is
  /// not deployed), so both testing networks default to a local node's
  /// lightwalletd gRPC listener (`rust/tests/wcash-regtest-node.toml`).
  String get lightwalletdHost => kWcashChain
      ? '127.0.0.1'
      : switch (this) {
          mainnet => 'us.zec.stardust.rest',
          testnet => 'lightwalletd.testnet.electriccoin.co',
          regtest => '127.0.0.1',
        };

  int get lightwalletdPort => kWcashChain
      ? 58234
      : switch (this) {
          mainnet => 443,
          testnet => 9067,
          regtest => 9067,
        };

  String get currencyTicker => kWcashChain
      ? switch (this) {
          mainnet => 'WEC',
          testnet => 'TWC',
          regtest => 'TWC',
        }
      : switch (this) {
          mainnet => 'ZEC',
          testnet => 'TAZ',
          regtest => 'TAZ',
        };

  String get lightwalletdUrl => switch (this) {
    _ when kWcashChain => 'http://$lightwalletdHost:$lightwalletdPort',
    regtest => 'http://$lightwalletdHost:$lightwalletdPort',
    _ => 'https://$lightwalletdHost:$lightwalletdPort',
  };

  /// Wcash launch networks activate the cumulative shielded upgrade set at
  /// height 1.
  int get saplingActivationHeight => kWcashChain
      ? 1
      : switch (this) {
          mainnet => 419200,
          testnet => 280000,
          regtest => 1,
        };
}

const kZcashDefaultNetworkEnvKey = 'ZCASH_DEFAULT_NETWORK';
const kZcashDefaultNetworkRaw = String.fromEnvironment(
  kZcashDefaultNetworkEnvKey,
  defaultValue: 'main',
);

const kZcashRegtestIronwoodActivationHeightEnvKey =
    'ZCASH_REGTEST_IRONWOOD_ACTIVATION_HEIGHT';
const kZcashRegtestIronwoodActivationHeight = int.fromEnvironment(
  kZcashRegtestIronwoodActivationHeightEnvKey,
  defaultValue: 0xFFFFFFFF,
);

const kZcashFastTestnetMigrationEnvKey = 'ZCASH_FAST_TESTNET_MIGRATION';
const kZcashFastTestnetMigration = bool.fromEnvironment(
  kZcashFastTestnetMigrationEnvKey,
  defaultValue: false,
);

/// Opt-in test build for Adam's private Ironwood chain that presents itself as
/// mainnet so normal-mode Keystone devices can use mainnet 133'/u1 derivation.
const kZcashIronwoodMasqueradeEnvKey = 'ZCASH_IRONWOOD_MASQUERADE';
const kZcashIronwoodMasquerade = bool.fromEnvironment(
  kZcashIronwoodMasqueradeEnvKey,
  defaultValue: false,
);

final String kZcashDefaultNetworkName = kZcashIronwoodMasquerade
    ? 'main'
    : normalizeZcashNetworkName(kZcashDefaultNetworkRaw);

final String kZcashDefaultCurrencyTicker = zcashNetworkFromName(
  kZcashDefaultNetworkName,
).currencyTicker;

String normalizeZcashNetworkName(String networkName) {
  return switch (networkName.trim()) {
    'test' => 'test',
    'regtest' => 'regtest',
    // Wcash mainnet is disabled upstream (the Rust layer rejects "main"), so
    // a wcash build falls back to the engineering testnet.
    _ => kWcashChain ? 'test' : 'main',
  };
}

String resolveStoredOrDefaultZcashNetworkName(String? storedNetworkName) {
  if (kZcashIronwoodMasquerade) return 'main';
  final stored = storedNetworkName?.trim();
  if (stored == null || stored.isEmpty) return kZcashDefaultNetworkName;
  return normalizeZcashNetworkName(stored);
}

ZcashNetwork zcashNetworkFromName(String networkName) {
  return switch (normalizeZcashNetworkName(networkName)) {
    'test' => ZcashNetwork.testnet,
    'regtest' => ZcashNetwork.regtest,
    _ => ZcashNetwork.mainnet,
  };
}

String secureStoreServiceForNetwork(String networkName) {
  final network = normalizeZcashNetworkName(networkName);
  // Wcash stores live in their own namespace so a wcash build can never read
  // or clobber a Zcash wallet's secure-store entries (and vice versa).
  if (kWcashChain) {
    return 'com.keplr.vizor.wcash.$network.secure_store';
  }
  if (kZcashIronwoodMasquerade && network == 'main') {
    return 'com.keplr.vizor.ironwood.secure_store';
  }
  return network == 'main'
      ? 'com.keplr.vizor.secure_store'
      : 'com.keplr.vizor.$network.secure_store';
}
