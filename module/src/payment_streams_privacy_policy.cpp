#include "payment_streams_privacy_policy.h"

namespace payment_streams_privacy {

QString depositOwnerMismatchMessage() {
    return QStringLiteral("deposit submitter must equal VaultConfig.owner for PseudonymousFunding vaults");
}

bool resolutionsContainPrivate(const QStringList& resolutions) {
    return resolutions.contains(QStringLiteral("private"));
}

bool pfOwnerSlotByLayout(VaultIxLayout layout, int index) {
    switch (layout) {
    case VaultIxLayout::InitOrDeposit3:
        return index == 2;
    case VaultIxLayout::StreamOwner5:
    case VaultIxLayout::StreamProvider6:
        return index == 3;
    }
    return false;
}

bool slotMayHoldPrivateKey(VaultIxLayout layout, int index) {
    switch (layout) {
    case VaultIxLayout::InitOrDeposit3:
        return index == 2;
    case VaultIxLayout::StreamOwner5:
        return index == 3;
    case VaultIxLayout::StreamProvider6:
        return index == 3 || index == 4;
    }
    return false;
}

VaultSubmitDecision decideVaultSubmitPath(uint8_t privacyTier,
                                          bool anyPrivateSlot,
                                          bool enforceDepositSubmitterEqualsOwner,
                                          const QString& submitterHexLower,
                                          const QString& vaultOwnerHexLower) {
    VaultSubmitDecision decision;
    if (enforceDepositSubmitterEqualsOwner && submitterHexLower != vaultOwnerHexLower) {
        decision.ok = false;
        decision.error = depositOwnerMismatchMessage();
        return decision;
    }
    if (anyPrivateSlot || privacyTier == kTierPseudonymousFunding) {
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
