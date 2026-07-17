#pragma once

#include <QJsonObject>
#include <QString>
#include <QStringList>

#include <cstdint>
#include <functional>

// Submit-path policy for vault-touching operations (D37.9).
// Any private account slot forces private submit. PseudonymousFunder vaults
// must never take the transparent submit path. Deposit may additionally
// require signer == VaultConfig.owner.
//
// Also hosts N5 peer-mapping encoding helpers for D37.12 unit smoke (Store
// dual-host E2E remains Step 38).

namespace payment_streams_privacy {

constexpr uint8_t kTierPublic = 0;
constexpr uint8_t kTierPseudonymousFunder = 1;

enum class VaultSubmitPath : uint8_t {
    Public = 0,
    Private = 1,
};

struct VaultSubmitDecision {
    bool ok = true;
    VaultSubmitPath path = VaultSubmitPath::Public;
    QString error;
};

// Slot-based submit selection:
// 1) optional deposit signer == owner check
// 2) anyPrivateSlot → Private
// 3) PseudonymousFunder → Private (never public)
// 4) else Public
VaultSubmitDecision decideVaultSubmitPath(uint8_t privacyTier,
                                          bool anyPrivateSlot,
                                          bool enforceDepositSignerEqualsOwner,
                                          const QString& signerHexLower,
                                          const QString& vaultOwnerHexLower);

QString depositSignerMismatchMessage();

bool resolutionsContainPrivate(const QStringList& resolutions);

// Host-local PeerId → payee base58 mapping (N5 / D37.12).
QString providerBase58ForPeer(const QJsonObject& mappings, const QString& peerId);
void setProviderBase58ForPeer(QJsonObject* mappings,
                              const QString& peerId,
                              const QString& accountIdBase58);
QString providerIdHexFromMappedBase58(const QString& base58,
                                      const std::function<QString(const QString&)>& base58ToHex,
                                      QString* errorOut);
bool providerIdHexMatchesStreamProvider(const QString& mappedProviderIdHex,
                                        const QString& streamProviderIdHex);

}  // namespace payment_streams_privacy
