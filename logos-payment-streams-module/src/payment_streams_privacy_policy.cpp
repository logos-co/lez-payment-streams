#include "payment_streams_privacy_policy.h"

namespace payment_streams_privacy {

QString depositSignerMismatchMessage() {
    return QStringLiteral("deposit signer must equal VaultConfig.owner for PseudonymousFunder vaults");
}

bool resolutionsContainPrivate(const QStringList& resolutions) {
    return resolutions.contains(QStringLiteral("private"));
}

VaultSubmitDecision decideVaultSubmitPath(uint8_t privacyTier,
                                          bool anyPrivateSlot,
                                          bool enforceDepositSignerEqualsOwner,
                                          const QString& signerHexLower,
                                          const QString& vaultOwnerHexLower) {
    VaultSubmitDecision decision;
    if (enforceDepositSignerEqualsOwner && signerHexLower != vaultOwnerHexLower) {
        decision.ok = false;
        decision.error = depositSignerMismatchMessage();
        return decision;
    }
    if (anyPrivateSlot || privacyTier == kTierPseudonymousFunder) {
        decision.path = VaultSubmitPath::Private;
        return decision;
    }
    decision.path = VaultSubmitPath::Public;
    return decision;
}

QString providerBase58ForPeer(const QJsonObject& mappings, const QString& peerId) {
    return mappings.value(peerId.trimmed()).toString().trimmed();
}

void setProviderBase58ForPeer(QJsonObject* mappings,
                              const QString& peerId,
                              const QString& accountIdBase58) {
    if (mappings == nullptr) {
        return;
    }
    mappings->insert(peerId.trimmed(), accountIdBase58.trimmed());
}

QString providerIdHexFromMappedBase58(const QString& base58,
                                      const std::function<QString(const QString&)>& base58ToHex,
                                      QString* errorOut) {
    const QString trimmed = base58.trimmed();
    if (trimmed.isEmpty()) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("provider account base58 empty");
        }
        return {};
    }
    if (!base58ToHex) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("provider base58 decoder missing");
        }
        return {};
    }
    const QString hex = base58ToHex(trimmed).trimmed().toLower();
    if (hex.size() != 64) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("provider account_id_from_base58 failed");
        }
        return {};
    }
    return hex;
}

bool providerIdHexMatchesStreamProvider(const QString& mappedProviderIdHex,
                                        const QString& streamProviderIdHex) {
    return !mappedProviderIdHex.isEmpty() &&
           mappedProviderIdHex.toLower() == streamProviderIdHex.trimmed().toLower();
}

}  // namespace payment_streams_privacy
