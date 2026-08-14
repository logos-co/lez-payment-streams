#include <logos_test.h>

#include "payment_streams_module_kit.h"

#include <QJsonArray>
#include <QJsonObject>

using payment_streams_kit::mergePersistedDiskKeys;

static QJsonObject acceptanceRow(qint64 vaultId,
                                 const QString& providerIdHex,
                                 const QString& sessionHex) {
    QJsonObject row;
    row.insert(QStringLiteral("vault_id"), vaultId);
    row.insert(QStringLiteral("provider_id_hex"), providerIdHex);
    row.insert(QStringLiteral("session_public_key_hex"), sessionHex);
    return row;
}

LOGOS_TEST(disk_only_acceptance_survives_merge) {
    QJsonObject memory;
    memory.insert(QStringLiteral("provider_acceptances"), QJsonArray());
    memory.insert(QStringLiteral("peer_mappings"), QJsonObject());

    QJsonArray diskAcc;
    diskAcc.append(acceptanceRow(
        0,
        QStringLiteral("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        QStringLiteral("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")));
    QJsonObject disk;
    disk.insert(QStringLiteral("provider_acceptances"), diskAcc);

    const QJsonObject merged = mergePersistedDiskKeys(memory, disk);
    const QJsonArray acc = merged.value(QStringLiteral("provider_acceptances")).toArray();
    LOGOS_ASSERT_EQ(acc.size(), 1);
    LOGOS_ASSERT_EQ(
        acc.at(0).toObject().value(QStringLiteral("session_public_key_hex")).toString(),
        QStringLiteral("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"));
}

LOGOS_TEST(memory_acceptance_wins_on_same_key) {
    const QString provider =
        QStringLiteral("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    QJsonArray memoryAcc;
    memoryAcc.append(acceptanceRow(0, provider, QStringLiteral("memory-session")));
    QJsonObject memory;
    memory.insert(QStringLiteral("provider_acceptances"), memoryAcc);

    QJsonArray diskAcc;
    diskAcc.append(acceptanceRow(0, provider.toUpper(), QStringLiteral("disk-session")));
    QJsonObject disk;
    disk.insert(QStringLiteral("provider_acceptances"), diskAcc);

    const QJsonObject merged = mergePersistedDiskKeys(memory, disk);
    const QJsonArray acc = merged.value(QStringLiteral("provider_acceptances")).toArray();
    LOGOS_ASSERT_EQ(acc.size(), 1);
    LOGOS_ASSERT_EQ(acc.at(0).toObject().value(QStringLiteral("session_public_key_hex")).toString(),
                    QStringLiteral("memory-session"));
}

LOGOS_TEST(disk_only_peer_mapping_survives_merge) {
    QJsonObject memory;
    memory.insert(QStringLiteral("peer_mappings"), QJsonObject());

    QJsonObject diskMappings;
    diskMappings.insert(QStringLiteral("16Uiu2HAmPeer"), QStringLiteral("ProviderBase58"));
    QJsonObject disk;
    disk.insert(QStringLiteral("peer_mappings"), diskMappings);

    const QJsonObject merged = mergePersistedDiskKeys(memory, disk);
    const QJsonObject mappings = merged.value(QStringLiteral("peer_mappings")).toObject();
    LOGOS_ASSERT_EQ(mappings.value(QStringLiteral("16Uiu2HAmPeer")).toString(),
                    QStringLiteral("ProviderBase58"));
}

LOGOS_TEST(memory_peer_mapping_wins_on_same_key) {
    QJsonObject memoryMappings;
    memoryMappings.insert(QStringLiteral("16Uiu2HAmPeer"), QStringLiteral("MemoryBase58"));
    QJsonObject memory;
    memory.insert(QStringLiteral("peer_mappings"), memoryMappings);

    QJsonObject diskMappings;
    diskMappings.insert(QStringLiteral("16Uiu2HAmPeer"), QStringLiteral("DiskBase58"));
    QJsonObject disk;
    disk.insert(QStringLiteral("peer_mappings"), diskMappings);

    const QJsonObject merged = mergePersistedDiskKeys(memory, disk);
    const QJsonObject mappings = merged.value(QStringLiteral("peer_mappings")).toObject();
    LOGOS_ASSERT_EQ(mappings.value(QStringLiteral("16Uiu2HAmPeer")).toString(),
                    QStringLiteral("MemoryBase58"));
}
