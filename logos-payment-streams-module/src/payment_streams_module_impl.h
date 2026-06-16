#pragma once

#include <logos_module_context.h>

#include <QString>
#include <QVariant>

class PaymentStreamsModuleImpl : public LogosModuleContext {
public:
    PaymentStreamsModuleImpl() = default;
    ~PaymentStreamsModuleImpl() override = default;

    QString accountIdHexFromBase58(const QVariant& accountIdBase58);
    QString readVaultConfigDecoded(const QVariant& vaultConfigAccountIdBase58);
    QString readVaultHoldingDecoded(const QVariant& vaultHoldingAccountIdBase58);
    QString readStreamConfigDecoded(const QVariant& streamConfigAccountIdBase58);
    QString readClockDecoded(const QVariant& clockAccountIdBase58);
    QString readClock10Decoded();
};
