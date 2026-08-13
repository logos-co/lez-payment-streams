#pragma once

#include <QJsonObject>

// Shared module helpers. Named namespace so one definition is linked across
// translation units (D50.3).

namespace payment_streams_kit {

// Merge disk-only provider_acceptances rows and peer_mappings keys into memory.
// Memory wins on conflict. Acceptance identity is (vault_id, provider_id_hex).
// peer_mappings merge by peer-id object key.
QJsonObject mergePersistedDiskKeys(const QJsonObject& memory, const QJsonObject& disk);

}  // namespace payment_streams_kit
