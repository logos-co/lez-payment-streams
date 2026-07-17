#include <logos_test.h>

#include "payment_streams_privacy_policy.h"

using payment_streams_privacy::decideVaultSubmitPath;
using payment_streams_privacy::depositSignerMismatchMessage;
using payment_streams_privacy::kTierPseudonymousFunder;
using payment_streams_privacy::kTierPublic;
using payment_streams_privacy::VaultSubmitPath;

static const QString kOwnerHex = QStringLiteral(
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
static const QString kOtherHex = QStringLiteral(
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");

LOGOS_TEST(pseudonymous_funder_routes_to_private_submit) {
    const auto d = decideVaultSubmitPath(kTierPseudonymousFunder, false, kOwnerHex, kOwnerHex);
    LOGOS_ASSERT_TRUE(d.ok);
    LOGOS_ASSERT_EQ(static_cast<int>(d.path), static_cast<int>(VaultSubmitPath::Private));
}

LOGOS_TEST(public_tier_routes_to_public_submit) {
    const auto d = decideVaultSubmitPath(kTierPublic, false, kOwnerHex, kOwnerHex);
    LOGOS_ASSERT_TRUE(d.ok);
    LOGOS_ASSERT_EQ(static_cast<int>(d.path), static_cast<int>(VaultSubmitPath::Public));
}

LOGOS_TEST(pseudonymous_funder_never_selects_public_submit) {
    const auto matched = decideVaultSubmitPath(kTierPseudonymousFunder, true, kOwnerHex, kOwnerHex);
    LOGOS_ASSERT_TRUE(matched.ok);
    LOGOS_ASSERT_TRUE(matched.path != VaultSubmitPath::Public);

    const auto noEnforce = decideVaultSubmitPath(kTierPseudonymousFunder, false, kOtherHex, kOwnerHex);
    LOGOS_ASSERT_TRUE(noEnforce.ok);
    LOGOS_ASSERT_TRUE(noEnforce.path != VaultSubmitPath::Public);
}

LOGOS_TEST(deposit_signer_mismatch_is_rejected) {
    const auto d = decideVaultSubmitPath(kTierPseudonymousFunder, true, kOtherHex, kOwnerHex);
    LOGOS_ASSERT_FALSE(d.ok);
    LOGOS_ASSERT_EQ(d.error, depositSignerMismatchMessage());
}

LOGOS_TEST(deposit_signer_match_allows_private_submit) {
    const auto d = decideVaultSubmitPath(kTierPseudonymousFunder, true, kOwnerHex, kOwnerHex);
    LOGOS_ASSERT_TRUE(d.ok);
    LOGOS_ASSERT_EQ(static_cast<int>(d.path), static_cast<int>(VaultSubmitPath::Private));
}

LOGOS_TEST(public_deposit_signer_mismatch_is_rejected) {
    const auto d = decideVaultSubmitPath(kTierPublic, true, kOtherHex, kOwnerHex);
    LOGOS_ASSERT_FALSE(d.ok);
    LOGOS_ASSERT_EQ(d.error, depositSignerMismatchMessage());
}
