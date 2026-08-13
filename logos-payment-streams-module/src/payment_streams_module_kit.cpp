#include "payment_streams_module_kit.h"

#include <QJsonArray>
#include <QJsonValue>

namespace payment_streams_kit {

namespace {

QString acceptanceProviderIdHex(const QJsonObject& row) {
    return row.value(QStringLiteral("provider_id_hex")).toString().trimmed().toLower();
}

quint64 acceptanceVaultId(const QJsonObject& row) {
    return static_cast<quint64>(row.value(QStringLiteral("vault_id")).toInteger());
}

bool memoryHasAcceptance(const QJsonArray& memory, const QJsonObject& diskRow) {
    const quint64 vaultId = acceptanceVaultId(diskRow);
    const QString providerIdHex = acceptanceProviderIdHex(diskRow);
    for (const QJsonValue& v : memory) {
        const QJsonObject row = v.toObject();
        if (acceptanceVaultId(row) == vaultId && acceptanceProviderIdHex(row) == providerIdHex) {
            return true;
        }
    }
    return false;
}

QJsonArray mergeProviderAcceptances(const QJsonArray& memory, const QJsonArray& disk) {
    QJsonArray out = memory;
    for (const QJsonValue& v : disk) {
        const QJsonObject row = v.toObject();
        if (!memoryHasAcceptance(memory, row)) {
            out.append(row);
        }
    }
    return out;
}

QJsonObject mergePeerMappings(const QJsonObject& memory, const QJsonObject& disk) {
    QJsonObject out = memory;
    for (auto it = disk.begin(); it != disk.end(); ++it) {
        if (!out.contains(it.key())) {
            out.insert(it.key(), it.value());
        }
    }
    return out;
}

}  // namespace

QJsonObject mergePersistedDiskKeys(const QJsonObject& memory, const QJsonObject& disk) {
    QJsonObject out = memory;
    const QJsonArray memoryAcceptances = memory.value(QStringLiteral("provider_acceptances")).toArray();
    const QJsonArray diskAcceptances = disk.value(QStringLiteral("provider_acceptances")).toArray();
    out.insert(QStringLiteral("provider_acceptances"),
               mergeProviderAcceptances(memoryAcceptances, diskAcceptances));

    const QJsonObject memoryMappings = memory.value(QStringLiteral("peer_mappings")).toObject();
    const QJsonObject diskMappings = disk.value(QStringLiteral("peer_mappings")).toObject();
    out.insert(QStringLiteral("peer_mappings"), mergePeerMappings(memoryMappings, diskMappings));
    return out;
}

}  // namespace payment_streams_kit
