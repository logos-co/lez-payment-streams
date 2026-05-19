#pragma once

#include "interface.h"

/**
 * Qt plugin interface for payment_streams_module (legacy PluginInterface).
 * Additional Q_INVOKABLE surface arrives in later integration steps.
 */
class IPaymentStreamsModule : public PluginInterface
{
public:
  virtual ~IPaymentStreamsModule() = default;
};

#define IPaymentStreamsModule_iid "org.logos.IPaymentStreamsModule"
Q_DECLARE_INTERFACE(IPaymentStreamsModule, IPaymentStreamsModule_iid)
