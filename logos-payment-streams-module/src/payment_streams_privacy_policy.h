#pragma once

#include <QString>

#include <cstdint>

// Submit-path policy for vault-touching operations.
// PseudonymousFunder vaults must never take the transparent submit path;
// deposit may additionally require signer == VaultConfig.owner.

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

// Returns Private for PseudonymousFunder, Public otherwise.
// When enforceDepositSignerEqualsOwner is true, signerHexLower must equal
// vaultOwnerHexLower or the decision is ok=false with a stable error string.
VaultSubmitDecision decideVaultSubmitPath(uint8_t privacyTier,
                                          bool enforceDepositSignerEqualsOwner,
                                          const QString& signerHexLower,
                                          const QString& vaultOwnerHexLower);

QString depositSignerMismatchMessage();

}  // namespace payment_streams_privacy
