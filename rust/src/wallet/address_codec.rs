//! Chain-flavor address codec.
//!
//! One place where wallet-facing address strings switch between the Zcash
//! namespace (default build) and the Wcash namespace (`--features wcash`).
//! Receiver payloads are identical in both namespaces; only the textual
//! encoding (HRPs, Base58 version bytes, F4Jumble padding domain) differs.
//!
//! RPC-facing transparent address strings (the lightwalletd
//! `GetAddressUtxos` path in `sync_engine`) intentionally do NOT go through
//! this module yet; see WCASH-PORT.md phase 2.

use transparent::address::TransparentAddress;
use zcash_keys::address::UnifiedAddress;

use super::network::WalletNetwork;

#[cfg(feature = "wcash")]
use zcash_protocol::consensus::Parameters as _;

/// Encodes a wallet-derived Unified Address for display/storage.
#[cfg(not(feature = "wcash"))]
pub(crate) fn encode_unified_address(
    ua: &UnifiedAddress,
    network: WalletNetwork,
) -> Result<String, String> {
    Ok(ua.encode(&network))
}

/// Encodes a wallet-derived Unified Address in the Wcash namespace.
#[cfg(feature = "wcash")]
pub(crate) fn encode_unified_address(
    ua: &UnifiedAddress,
    network: WalletNetwork,
) -> Result<String, String> {
    super::wcash_address::encode_wallet_unified_address(ua, network.network_type())
        .map_err(|e| format!("Failed to encode Wcash address: {e}"))
}

/// Encodes a wallet-derived transparent receiver for display/storage.
#[cfg(not(feature = "wcash"))]
pub(crate) fn encode_transparent_address(
    address: &TransparentAddress,
    network: WalletNetwork,
) -> String {
    zcash_keys::encoding::encode_transparent_address_p(&network, address)
}

/// Encodes a wallet-derived transparent receiver in the Wcash namespace.
#[cfg(feature = "wcash")]
pub(crate) fn encode_transparent_address(
    address: &TransparentAddress,
    network: WalletNetwork,
) -> String {
    super::wcash_address::encode_wallet_transparent_address(address, network.network_type())
}

/// Parses a recipient string into the `ZcashAddress` container consumed by
/// `TransactionRequest` and the librustzcash send stack.
#[cfg(not(feature = "wcash"))]
pub(crate) fn parse_recipient(
    to_address: &str,
    _network: WalletNetwork,
) -> Result<zcash_address::ZcashAddress, String> {
    to_address
        .parse()
        .map_err(|e| format!("Bad address: {e}"))
}

/// Parses a Wcash recipient string, fail-closed: Zcash textual addresses and
/// other Wcash networks are rejected, then the receiver payload is re-wrapped
/// as the `ZcashAddress` container the librustzcash send stack consumes.
#[cfg(feature = "wcash")]
pub(crate) fn parse_recipient(
    to_address: &str,
    network: WalletNetwork,
) -> Result<zcash_address::ZcashAddress, String> {
    let parsed = super::wcash_address::WcashAddress::try_from_encoded(to_address)
        .map_err(|e| format!("Bad address: {e}"))?;
    if parsed.network() != network.network_type() {
        return Err("Bad address: address is for a different Wcash network".to_string());
    }
    let address = parsed
        .convert::<zcash_keys::address::Address>()
        .map_err(|e| format!("Bad address: {e}"))?;
    Ok(address.to_zcash_address(&network))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regtest_unified_address() -> UnifiedAddress {
        use zcash_address::unified::Encoding as _;
        let encoded = "uregtest1pszqlgxaf5w8mu2yd9uygg8cswp0ec4f7eejqnqc35tztw4tk0sxnt3pym2f3s2872cy2ruuc5n8y9cen5q6ngzlmzu8ztrjesv8zm9j";
        let (_, container) = zcash_address::unified::Address::decode(encoded).unwrap();
        UnifiedAddress::try_from(container).unwrap()
    }

    #[test]
    #[cfg(not(feature = "wcash"))]
    fn default_flavor_round_trips_zcash_namespace() {
        let ua = regtest_unified_address();
        let encoded = encode_unified_address(&ua, WalletNetwork::Regtest).unwrap();
        assert!(encoded.starts_with("uregtest1"));
        assert!(parse_recipient(&encoded, WalletNetwork::Regtest).is_ok());
    }

    #[test]
    #[cfg(feature = "wcash")]
    fn wcash_flavor_round_trips_wcash_namespace_and_rejects_zcash() {
        let ua = regtest_unified_address();
        let encoded = encode_unified_address(&ua, WalletNetwork::Regtest).unwrap();
        assert!(encoded.starts_with("wuregtest1"));

        let recipient = parse_recipient(&encoded, WalletNetwork::Regtest).unwrap();
        assert_eq!(recipient.encode(), ua.encode(&WalletNetwork::Regtest));

        // The Zcash encoding of the very same receiver is rejected.
        assert!(parse_recipient(&ua.encode(&WalletNetwork::Regtest), WalletNetwork::Regtest)
            .is_err());

        // Transparent receivers use the Wcash Base58 namespace.
        let taddr = TransparentAddress::PublicKeyHash([7; 20]);
        let encoded = encode_transparent_address(&taddr, WalletNetwork::Regtest);
        assert!(encoded.starts_with("WR"));
    }
}
