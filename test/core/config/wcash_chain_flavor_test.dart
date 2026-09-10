@Tags(['wcash'])
library;

import 'package:flutter_test/flutter_test.dart';
import 'package:zcash_wallet/src/core/config/network_config.dart';
import 'package:zcash_wallet/src/features/address_book/models/address_book_contact.dart';
import 'package:zcash_wallet/src/features/address_book/models/address_format_validator.dart';

// Wcash-lane assertions: this file is compiled and run only by
// `fvm flutter test --tags wcash --run-skipped --dart-define=VIZOR_CHAIN=wcash`.
// Fixtures are the deterministic addresses from the Rust wcash regtest E2E
// (rust/tests/wcash_regtest_sync.rs) and wolf's public regtest mining fixture.
const _wcashRegtestUa =
    'wuregtest1de8nt40l5457cv5s565m6kn543wxvh06kf46xnfeg6wv9q824emvluvkuuqn8c7u40c0vup4avryvqp76f8pqakawmluqzaug53829az';
const _wcashRegtestTransparent = 'WR64VqQpZRujxYnAJmqGK4d4fbqQZRZHazG';
const _zcashRegtestUa =
    'uregtest1pszqlgxaf5w8mu2yd9uygg8cswp0ec4f7eejqnqc35tztw4tk0sxnt3pym2f3s2872cy2ruuc5n8y9cen5q6ngzlmzu8ztrjesv8zm9j';

void main() {
  test('wcash lane runs with the wcash chain define', () {
    // The analogue of test/mobile_lane_sanity_test.dart: fail loudly by name
    // when the lane command forgot --dart-define=VIZOR_CHAIN=wcash.
    expect(
      kWcashChain,
      isTrue,
      reason:
          'run this lane with fvm flutter test --tags wcash --run-skipped '
          '--dart-define=VIZOR_CHAIN=wcash',
    );
  });

  test('wcash networks use Wcash tickers and namespaces', () {
    expect(ZcashNetwork.mainnet.currencyTicker, 'WEC');
    expect(ZcashNetwork.testnet.currencyTicker, 'TWC');
    expect(ZcashNetwork.regtest.currencyTicker, 'TWC');

    expect(ZcashNetwork.regtest.uaPrefix, 'wuregtest1');
    expect(ZcashNetwork.testnet.uaPrefix, 'wutest1');
    expect(ZcashNetwork.regtest.tAddrPrefixes, ['WR', 'WS']);
    expect(ZcashNetwork.regtest.texPrefix, 'wtexregtest1');
    expect(ZcashNetwork.regtest.supportsSaplingRecipients, isFalse);
    expect(ZcashNetwork.testnet.saplingActivationHeight, 1);
  });

  test('wcash builds reject mainnet and default to the testnet profile', () {
    expect(normalizeZcashNetworkName('main'), 'test');
    expect(normalizeZcashNetworkName('garbage'), 'test');
    expect(normalizeZcashNetworkName('regtest'), 'regtest');
    expect(kZcashDefaultNetworkName, isNot('main'));
  });

  test('wcash secure stores never share a namespace with Zcash stores', () {
    for (final network in ['main', 'test', 'regtest']) {
      final service = secureStoreServiceForNetwork(network);
      expect(service, startsWith('com.keplr.vizor.wcash.'));
    }
  });

  test('address validation accepts Wcash and rejects Zcash encodings', () {
    AddressFormatFinding? check(String value) => addressFormatCheck(
      AddressBookNetwork.zcash,
      value,
      zcashNetwork: ZcashNetwork.regtest,
    );

    expect(check(_wcashRegtestUa), isNull);
    expect(check(_wcashRegtestTransparent), isNull);
    expect(check(_zcashRegtestUa), isNotNull);
    // Wcash reserves but never accepts Sapling namespaces.
    expect(check('wregtestsapling1qqqqqqqqqqqqq'), isNotNull);
  });
}
