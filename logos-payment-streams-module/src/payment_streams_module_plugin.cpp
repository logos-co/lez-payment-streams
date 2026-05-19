#include "payment_streams_module_plugin.h"

#include <QtCore/QDebug>
#include <QtCore/QVariant>

#include <cpp/logos_api_client.h>

PaymentStreamsModulePlugin::PaymentStreamsModulePlugin() = default;

PaymentStreamsModulePlugin::~PaymentStreamsModulePlugin() = default;

QString PaymentStreamsModulePlugin::name() const
{
  return QStringLiteral("payment_streams_module");
}

QString PaymentStreamsModulePlugin::version() const
{
  return QStringLiteral("0.1.0");
}

void PaymentStreamsModulePlugin::initLogos(LogosAPI* logosApiInstance)
{
  m_logosApi = logosApiInstance;

  if (!m_logosApi)
  {
    qWarning() << "payment_streams_module: initLogos called with null LogosAPI";
    return;
  }

  LogosAPIClient* walletClient = m_logosApi->getClient(QStringLiteral("lez_wallet_module"));
  if (!walletClient)
  {
    qWarning() << "payment_streams_module: no LogosAPIClient for lez_wallet_module";
    return;
  }

  const QVariant probe = walletClient->invokeRemoteMethod(
      QStringLiteral("lez_wallet_module"), QStringLiteral("list_accounts"));
  qDebug() << "payment_streams_module: wallet list_accounts probe finished:" << probe;
}
