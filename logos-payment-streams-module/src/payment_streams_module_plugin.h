#pragma once

#include <QtCore/QObject>
#include <QtCore/QString>

#include "i_payment_streams_module.h"
#include "logos_api.h"

/**
 * Logos Core module shell for payment_streams_module (Step 6c).
 * Links lez_payment_streams_ffi and probes cross-module plumbing to lez_wallet_module.
 */
class PaymentStreamsModulePlugin : public QObject, public IPaymentStreamsModule
{
  Q_OBJECT
  Q_PLUGIN_METADATA(IID IPaymentStreamsModule_iid FILE "metadata.json")
  Q_INTERFACES(IPaymentStreamsModule PluginInterface)

public:
  PaymentStreamsModulePlugin();
  ~PaymentStreamsModulePlugin() override;

  QString name() const override;
  QString version() const override;
  Q_INVOKABLE void initLogos(LogosAPI* logosApiInstance);

private:
  LogosAPI* m_logosApi = nullptr;
};
