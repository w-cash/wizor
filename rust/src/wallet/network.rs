#[cfg(test)]
use std::cell::Cell;
#[cfg(not(test))]
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(feature = "wcash")]
use zcash_protocol::consensus::BranchId;
#[cfg(not(feature = "wcash"))]
use zcash_protocol::consensus::Network;
use zcash_protocol::consensus::{BlockHeight, NetworkType, NetworkUpgrade, Parameters};

// Keep ordinary regtest builds Orchard-only unless an Ironwood E2E explicitly
// configures a controlled activation height.
const DEFAULT_REGTEST_NU6_3_ACTIVATION_HEIGHT: u32 = u32::MAX;
#[cfg(not(test))]
static REGTEST_NU6_3_ACTIVATION_HEIGHT: AtomicU32 =
    AtomicU32::new(DEFAULT_REGTEST_NU6_3_ACTIVATION_HEIGHT);

#[cfg(test)]
thread_local! {
    static REGTEST_NU6_3_ACTIVATION_HEIGHT: Cell<u32> =
        const { Cell::new(DEFAULT_REGTEST_NU6_3_ACTIVATION_HEIGHT) };
}

pub fn configure_regtest_nu6_3_activation_height(height: u32) -> Result<(), String> {
    if height < 2 {
        return Err("Regtest NU6.3 activation height must be at least 2".to_string());
    }
    #[cfg(not(test))]
    REGTEST_NU6_3_ACTIVATION_HEIGHT.store(height, Ordering::SeqCst);
    #[cfg(test)]
    REGTEST_NU6_3_ACTIVATION_HEIGHT.set(height);
    Ok(())
}

// Unused in wcash builds: Wcash regtest activates NU6.3 at height 1.
#[cfg_attr(feature = "wcash", allow(dead_code))]
fn regtest_nu6_3_activation_height() -> BlockHeight {
    #[cfg(not(test))]
    let height = REGTEST_NU6_3_ACTIVATION_HEIGHT.load(Ordering::SeqCst);
    #[cfg(test)]
    let height = REGTEST_NU6_3_ACTIVATION_HEIGHT.get();
    BlockHeight::from_u32(height)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WalletNetwork {
    Main,
    Test,
    Regtest,
}

impl WalletNetwork {
    pub fn from_str(network: &str) -> Option<Self> {
        match network {
            // Wcash mainnet is disabled upstream, so a wcash build refuses to
            // select it: "main" falls through to `None`.
            #[cfg(not(feature = "wcash"))]
            "main" => Some(Self::Main),
            "test" => Some(Self::Test),
            "regtest" => Some(Self::Regtest),
            _ => None,
        }
    }
}

#[cfg(ironwood_masquerade)]
fn ironwood_masquerade_activation_height(nu: NetworkUpgrade) -> Option<BlockHeight> {
    let height = match nu {
        NetworkUpgrade::Overwinter
        | NetworkUpgrade::Sapling
        | NetworkUpgrade::Blossom
        | NetworkUpgrade::Heartwood
        | NetworkUpgrade::Canopy => 1,
        NetworkUpgrade::Nu5 => 2,
        NetworkUpgrade::Nu6 => 3,
        NetworkUpgrade::Nu6_1 => 4,
        NetworkUpgrade::Nu6_2 => 5,
        NetworkUpgrade::Nu6_3 => 5000,
    };
    Some(BlockHeight::from_u32(height))
}

/// Wcash launch networks activate the cumulative shielded upgrade set at
/// height 1, mirroring zebra-chain's `new_wcash_testnet` / `new_wcash_regtest`
/// (`ConfiguredActivationHeights { nu6_3: Some(1), .. }` is cumulative there).
#[cfg(feature = "wcash")]
fn wcash_activation_height(nu: NetworkUpgrade) -> Option<BlockHeight> {
    match nu {
        NetworkUpgrade::Overwinter
        | NetworkUpgrade::Sapling
        | NetworkUpgrade::Blossom
        | NetworkUpgrade::Heartwood
        | NetworkUpgrade::Canopy
        | NetworkUpgrade::Nu5
        | NetworkUpgrade::Nu6
        | NetworkUpgrade::Nu6_1
        | NetworkUpgrade::Nu6_2
        | NetworkUpgrade::Nu6_3 => Some(BlockHeight::from_u32(1)),
    }
}

impl Parameters for WalletNetwork {
    fn network_type(&self) -> NetworkType {
        match self {
            Self::Main => NetworkType::Main,
            Self::Test => NetworkType::Test,
            Self::Regtest => NetworkType::Regtest,
        }
    }

    #[cfg(feature = "wcash")]
    fn activation_height(&self, nu: NetworkUpgrade) -> Option<BlockHeight> {
        match self {
            // Wcash mainnet is disabled upstream: no upgrade is ever active,
            // so a `Main` wallet fails closed instead of using Zcash domains.
            Self::Main => None,
            Self::Test | Self::Regtest => wcash_activation_height(nu),
        }
    }

    #[cfg(not(feature = "wcash"))]
    fn activation_height(&self, nu: NetworkUpgrade) -> Option<BlockHeight> {
        match self {
            #[cfg(ironwood_masquerade)]
            Self::Main => ironwood_masquerade_activation_height(nu),
            #[cfg(not(ironwood_masquerade))]
            Self::Main => Network::MainNetwork.activation_height(nu),
            Self::Test => Network::TestNetwork.activation_height(nu),
            Self::Regtest => match nu {
                NetworkUpgrade::Overwinter
                | NetworkUpgrade::Sapling
                | NetworkUpgrade::Blossom
                | NetworkUpgrade::Heartwood
                | NetworkUpgrade::Canopy
                | NetworkUpgrade::Nu5
                | NetworkUpgrade::Nu6
                | NetworkUpgrade::Nu6_1
                | NetworkUpgrade::Nu6_2 => Some(BlockHeight::from_u32(1)),
                NetworkUpgrade::Nu6_3 => Some(regtest_nu6_3_activation_height()),
            },
        }
    }

    /// Wcash's NU6.3 uses chain-specific signature and transaction-hash
    /// domains that are distinct from Zcash NU6.3 while keeping its protocol
    /// semantics (see wolf's `vendor/zcash_protocol/WCASH-PATCHES.md`).
    #[cfg(feature = "wcash")]
    fn branch_id_for_upgrade(&self, nu: NetworkUpgrade) -> BranchId {
        if nu == NetworkUpgrade::Nu6_3 {
            match self {
                // Unreachable in practice: `Main` has no activation heights
                // under wcash, so no branch is ever selected for it.
                Self::Main => nu.branch_id(),
                Self::Test => BranchId::WcashTestnetV1,
                Self::Regtest => BranchId::WcashRegtestV1,
            }
        } else {
            nu.branch_id()
        }
    }
}

#[cfg(all(test, feature = "wcash"))]
mod wcash_tests {
    use super::*;

    #[test]
    fn wcash_networks_expose_cumulative_shielded_activations_at_launch() {
        let launch = Some(BlockHeight::from_u32(1));
        for network in [WalletNetwork::Test, WalletNetwork::Regtest] {
            assert_eq!(network.activation_height(NetworkUpgrade::Sapling), launch);
            assert_eq!(network.activation_height(NetworkUpgrade::Nu5), launch);
            assert_eq!(network.activation_height(NetworkUpgrade::Nu6_3), launch);
        }
    }

    #[test]
    fn wcash_networks_use_disjoint_wcash_transaction_domains() {
        assert_eq!(
            WalletNetwork::Test.branch_id_for_upgrade(NetworkUpgrade::Nu6_3),
            BranchId::WcashTestnetV1
        );
        assert_eq!(
            WalletNetwork::Regtest.branch_id_for_upgrade(NetworkUpgrade::Nu6_3),
            BranchId::WcashRegtestV1
        );
        // Post-genesis heights select the Wcash domain, not Zcash NU6.3.
        assert_eq!(
            BranchId::for_height(&WalletNetwork::Test, BlockHeight::from_u32(1)),
            BranchId::WcashTestnetV1
        );
        assert_eq!(
            BranchId::for_height(&WalletNetwork::Regtest, BlockHeight::from_u32(100)),
            BranchId::WcashRegtestV1
        );
    }

    #[test]
    fn wcash_build_rejects_mainnet() {
        assert_eq!(WalletNetwork::from_str("main"), None);
        assert_eq!(
            WalletNetwork::Main.activation_height(NetworkUpgrade::Nu6_3),
            None
        );
        assert!(matches!(
            WalletNetwork::from_str("test"),
            Some(WalletNetwork::Test)
        ));
        assert!(matches!(
            WalletNetwork::from_str("regtest"),
            Some(WalletNetwork::Regtest)
        ));
    }
}

#[cfg(all(test, ironwood_masquerade))]
mod tests {
    use super::*;

    #[test]
    fn masquerade_main_keeps_mainnet_identity_with_test_chain_activation_heights() {
        let network = WalletNetwork::Main;

        assert_eq!(network.network_type(), NetworkType::Main);
        assert_eq!(
            network.activation_height(NetworkUpgrade::Nu6_3),
            Some(BlockHeight::from_u32(5000))
        );
    }
}
