#pragma once

#include <QJsonObject>
#include <QString>
#include <QStringList>

#include <cstdint>
#include <functional>

// Submit-path policy for vault-touching operations (D37.9).
// Any private account slot forces private submit. PseudonymousFunding vaults
// must never take the transparent submit path. Deposit may additionally
// require submitter == VaultConfig.owner.
//
// Also hosts N5 peer-mapping encoding helpers for D37.12 unit smoke (Store
// dual-host E2E remains Step 38).

namespace payment_streams_privacy {

constexpr uint8_t kTierPublic = 0;
constexpr uint8_t kTierPseudonymousFunding = 1;

enum class VaultSubmitPath : uint8_t {
    Public = 0,
    Private = 1,
};

// Account-plan layouts used by vault-touching chainAction ops.
enum class VaultIxLayout : uint8_t {
    InitOrDeposit3 = 0,
    StreamOwner5 = 1,
    StreamProvider6 = 2,
    Withdraw4 = 3,
};

// True for owner, provider, or authority slots. PDA and clock slots are never
// private-key accounts and must not be probed with get_private_account_keys.
bool slotMayHoldPrivateKey(VaultIxLayout layout, int index);

// PF layout fallback: owner identity slots stay private even if keychain probe
// races.
bool pfOwnerSlotByLayout(VaultIxLayout layout, int index);

struct VaultSubmitDecision {
    bool ok = true;
    VaultSubmitPath path = VaultSubmitPath::Public;
    QString error;
};

// Slot-based submit selection:
// 1) optional deposit submitter == owner check
// 2) anyPrivateSlot → Private
// 3) PseudonymousFunding → Private (never public)
// 4) else Public
VaultSubmitDecision decideVaultSubmitPath(uint8_t privacyTier,
                                          bool anyPrivateSlot,
                                          bool enforceDepositSubmitterEqualsOwner,
                                          const QString& submitterHexLower,
                                          const QString& vaultOwnerHexLower);

QString depositOwnerMismatchMessage();

bool resolutionsContainPrivate(const QStringList& resolutions);

// Host-local PeerId → provider base58 mapping (N5 / D37.12).
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
