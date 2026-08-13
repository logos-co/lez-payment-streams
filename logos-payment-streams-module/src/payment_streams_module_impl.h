#pragma once

#include <logos_module_context.h>

#include <QString>
#include <QVariant>

class PaymentStreamsModuleImpl : public LogosModuleContext {
public:
    PaymentStreamsModuleImpl() = default;
    ~PaymentStreamsModuleImpl() override = default;

    void onContextReady() override;

    QString readVaultConfigDecoded(const QVariant& vaultConfigAccountIdBase58);
    QString readVaultHoldingDecoded(const QVariant& vaultHoldingAccountIdBase58);
    QString readStreamConfigDecoded(const QVariant& streamConfigAccountIdBase58);
    QString readClockDecoded(const QVariant& clockAccountIdBase58);
    QString readClock10Decoded();

    QString chainAction(const QVariant& operation, const QVariant& paramsJson);

    QString registerProviderMapping(const QVariant& providerPeerId, const QVariant& providerAccountIdBase58);
    QString prepareEligibilityProofWithStreamProposalForStoreQuery(const QVariant& canonicalRequestHex, const QVariant& providerPeerId);
    QString prepareEligibilityProofWithStreamProofForStoreQuery(const QVariant& canonicalRequestHex, const QVariant& providerPeerId, const QVariant& streamId);
    QString verifyEligibilityForStoreQuery(const QVariant& proofBytes, const QVariant& canonicalRequestBytes, const QVariant& userPeerId);
    QString listMyStreams(const QVariant& vaultId);
    QString rediscoverStreams(const QVariant& vaultId);

private:
    QString accountIdHexFromBase58(const QVariant& accountIdBase58);
    QString initializeVault(const QVariant& ownerAccountIdBase58,
                            const QVariant& vaultId,
                            const QVariant& privacyTier,
                            const QVariant& tokenId);
    QString deposit(const QVariant& ownerAccountIdBase58,
                    const QVariant& vaultId,
                    const QVariant& amountLo,
                    const QVariant& amountHi);
    QString withdraw(const QVariant& ownerAccountIdBase58,
                     const QVariant& vaultId,
                     const QVariant& amountLo,
                     const QVariant& amountHi,
                     const QVariant& withdrawToAccountIdBase58);
    QString createStream(const QVariant& ownerAccountIdBase58,
                         const QVariant& vaultId,
                         const QVariant& streamId,
                         const QVariant& providerAccountIdBase58,
                         const QVariant& rateTokensPerSecond,
                         const QVariant& allocationLo,
                         const QVariant& allocationHi);
    QString pauseStream(const QVariant& ownerAccountIdBase58,
                        const QVariant& vaultId,
                        const QVariant& streamId);
    QString resumeStream(const QVariant& ownerAccountIdBase58,
                         const QVariant& vaultId,
                         const QVariant& streamId);
    QString topUpStream(const QVariant& ownerAccountIdBase58,
                        const QVariant& vaultId,
                        const QVariant& streamId,
                        const QVariant& increaseLo,
                        const QVariant& increaseHi);
    QString closeStream(const QVariant& ownerAccountIdBase58,
                        const QVariant& vaultId,
                        const QVariant& streamId,
                        const QVariant& providerAccountIdBase58);
    QString claim(const QVariant& ownerAccountIdBase58,
                  const QVariant& providerAccountIdBase58,
                  const QVariant& vaultId,
                  const QVariant& streamId);
    QString getVaultStatus(const QVariant& ownerAccountIdBase58,
                           const QVariant& vaultId,
                           const QVariant& streamId);
    QString getStreamStatus(const QVariant& ownerAccountIdBase58,
                            const QVariant& vaultId,
                            const QVariant& streamId);
};
