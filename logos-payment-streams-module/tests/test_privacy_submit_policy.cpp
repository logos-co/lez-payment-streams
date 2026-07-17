#include <logos_test.h>

#include "payment_streams_privacy_policy.h"

#include <QJsonObject>

using payment_streams_privacy::decideVaultSubmitPath;
using payment_streams_privacy::depositSignerMismatchMessage;
using payment_streams_privacy::kTierPseudonymousFunder;
using payment_streams_privacy::kTierPublic;
using payment_streams_privacy::providerBase58ForPeer;
using payment_streams_privacy::providerIdHexFromMappedBase58;
using payment_streams_privacy::providerIdHexMatchesStreamProvider;
using payment_streams_privacy::resolutionsContainPrivate;
using payment_streams_privacy::setProviderBase58ForPeer;
using payment_streams_privacy::VaultSubmitPath;

static const QString kOwnerHex = QStringLiteral(
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
static const QString kOtherHex = QStringLiteral(
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
static const QString kPeer = QStringLiteral("16Uiu2HAmProviderPeerForEncodingSmoke");
static const QString kPrivateProviderB58 = QStringLiteral("PrivProvAccountBase58EncodedValue");
static const QString kProviderHex = QStringLiteral(
    "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");

LOGOS_TEST(pseudonymous_funder_routes_to_private_submit) {
    const auto d = decideVaultSubmitPath(kTierPseudonymousFunder, false, false, kOwnerHex, kOwnerHex);
    LOGOS_ASSERT_TRUE(d.ok);
    LOGOS_ASSERT_EQ(static_cast<int>(d.path), static_cast<int>(VaultSubmitPath::Private));
}

LOGOS_TEST(public_tier_routes_to_public_submit) {
    const auto d = decideVaultSubmitPath(kTierPublic, false, false, kOwnerHex, kOwnerHex);
    LOGOS_ASSERT_TRUE(d.ok);
    LOGOS_ASSERT_EQ(static_cast<int>(d.path), static_cast<int>(VaultSubmitPath::Public));
}

LOGOS_TEST(any_private_slot_on_public_vault_routes_to_private_submit) {
    // D37.9: private provider claim on a Public vault.
    const auto d = decideVaultSubmitPath(kTierPublic, true, false, kOtherHex, kOwnerHex);
    LOGOS_ASSERT_TRUE(d.ok);
    LOGOS_ASSERT_EQ(static_cast<int>(d.path), static_cast<int>(VaultSubmitPath::Private));
}

LOGOS_TEST(public_slots_without_private_stay_on_public_submit) {
    const auto d = decideVaultSubmitPath(kTierPublic, false, false, kOtherHex, kOwnerHex);
    LOGOS_ASSERT_TRUE(d.ok);
    LOGOS_ASSERT_EQ(static_cast<int>(d.path), static_cast<int>(VaultSubmitPath::Public));
}

LOGOS_TEST(pseudonymous_funder_never_selects_public_submit) {
    const auto matched = decideVaultSubmitPath(kTierPseudonymousFunder, true, true, kOwnerHex, kOwnerHex);
    LOGOS_ASSERT_TRUE(matched.ok);
    LOGOS_ASSERT_TRUE(matched.path != VaultSubmitPath::Public);

    const auto noPrivateSlots =
        decideVaultSubmitPath(kTierPseudonymousFunder, false, false, kOtherHex, kOwnerHex);
    LOGOS_ASSERT_TRUE(noPrivateSlots.ok);
    LOGOS_ASSERT_TRUE(noPrivateSlots.path != VaultSubmitPath::Public);
}

LOGOS_TEST(deposit_signer_mismatch_is_rejected) {
    const auto d = decideVaultSubmitPath(kTierPseudonymousFunder, true, true, kOtherHex, kOwnerHex);
    LOGOS_ASSERT_FALSE(d.ok);
    LOGOS_ASSERT_EQ(d.error, depositSignerMismatchMessage());
}

LOGOS_TEST(deposit_signer_match_allows_private_submit) {
    const auto d = decideVaultSubmitPath(kTierPseudonymousFunder, true, true, kOwnerHex, kOwnerHex);
    LOGOS_ASSERT_TRUE(d.ok);
    LOGOS_ASSERT_EQ(static_cast<int>(d.path), static_cast<int>(VaultSubmitPath::Private));
}

LOGOS_TEST(public_deposit_signer_mismatch_is_rejected) {
    const auto d = decideVaultSubmitPath(kTierPublic, false, true, kOtherHex, kOwnerHex);
    LOGOS_ASSERT_FALSE(d.ok);
    LOGOS_ASSERT_EQ(d.error, depositSignerMismatchMessage());
}

LOGOS_TEST(resolutions_contain_private_helper) {
    LOGOS_ASSERT_TRUE(resolutionsContainPrivate(
        QStringList{QStringLiteral("public_no_sign"), QStringLiteral("private")}));
    LOGOS_ASSERT_FALSE(resolutionsContainPrivate(
        QStringList{QStringLiteral("public_sign"), QStringLiteral("public_no_sign")}));
}

LOGOS_TEST(register_provider_mapping_stores_private_provider_base58) {
    QJsonObject mappings;
    setProviderBase58ForPeer(&mappings, kPeer, kPrivateProviderB58);
    LOGOS_ASSERT_EQ(providerBase58ForPeer(mappings, kPeer), kPrivateProviderB58);
}

LOGOS_TEST(mapped_private_provider_decodes_to_stream_provider_hex) {
    QJsonObject mappings;
    setProviderBase58ForPeer(&mappings, kPeer, kPrivateProviderB58);
    const QString base58 = providerBase58ForPeer(mappings, kPeer);
    QString err;
    const QString mappedHex = providerIdHexFromMappedBase58(
        base58,
        [](const QString& b58) {
            if (b58 == kPrivateProviderB58) {
                return kProviderHex;
            }
            return QString();
        },
        &err);
    LOGOS_ASSERT_TRUE(err.isEmpty());
    LOGOS_ASSERT_EQ(mappedHex, kProviderHex);
    LOGOS_ASSERT_TRUE(providerIdHexMatchesStreamProvider(mappedHex, kProviderHex));
}

LOGOS_TEST(mapped_provider_hex_mismatch_is_detected) {
    const QString otherHex = QStringLiteral(
        "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");
    LOGOS_ASSERT_FALSE(providerIdHexMatchesStreamProvider(kProviderHex, otherHex));
}

LOGOS_TEST(empty_mapped_base58_fails_hex_decode) {
    QString err;
    const QString hex = providerIdHexFromMappedBase58(
        QString(),
        [](const QString&) { return kProviderHex; },
        &err);
    LOGOS_ASSERT_TRUE(hex.isEmpty());
    LOGOS_ASSERT_FALSE(err.isEmpty());
}
