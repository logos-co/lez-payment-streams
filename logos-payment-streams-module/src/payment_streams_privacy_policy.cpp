#include "payment_streams_privacy_policy.h"

namespace payment_streams_privacy {

QString depositSignerMismatchMessage() {
    return QStringLiteral("deposit signer must equal VaultConfig.owner for PseudonymousFunder vaults");
}

VaultSubmitDecision decideVaultSubmitPath(uint8_t privacyTier,
                                          bool enforceDepositSignerEqualsOwner,
                                          const QString& signerHexLower,
                                          const QString& vaultOwnerHexLower) {
    VaultSubmitDecision decision;
    if (enforceDepositSignerEqualsOwner && signerHexLower != vaultOwnerHexLower) {
        decision.ok = false;
        decision.error = depositSignerMismatchMessage();
        return decision;
    }
    if (privacyTier == kTierPseudonymousFunder) {
        decision.path = VaultSubmitPath::Private;
        return decision;
    }
    decision.path = VaultSubmitPath::Public;
    return decision;
}

}  // namespace payment_streams_privacy
