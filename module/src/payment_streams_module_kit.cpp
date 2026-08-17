#include "payment_streams_module_kit.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QJsonValue>

#include <cstring>

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

quint64 chainTimestampToFoldSeconds(quint64 ts) {
    // Matches lez_payment_streams_core::chain_timestamp_to_fold_seconds (>= 1e12 ms).
    if (ts >= kMsEpochThreshold) {
        return ts / 1000;
    }
    return ts;
}

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

QString makeErrorJson(const QString& message) {
    return makeErrorJson(message, QJsonObject());
}

QString makeErrorJson(const QString& message, const QJsonObject& extra) {
    QJsonObject obj;
    obj.insert(QStringLiteral("status"), QStringLiteral("error"));
    obj.insert(QStringLiteral("message"), message);
    for (auto it = extra.begin(); it != extra.end(); ++it) {
        if (it.key() == QStringLiteral("status") || it.key() == QStringLiteral("message")) {
            continue;
        }
        obj.insert(it.key(), it.value());
    }
    return QJsonDocument(obj).toJson(QJsonDocument::Compact);
}

QString makeOkJson(const QJsonObject& payload) {
    QJsonObject obj;
    obj.insert(QStringLiteral("status"), QStringLiteral("ok"));
    for (auto it = payload.begin(); it != payload.end(); ++it) {
        obj.insert(it.key(), it.value());
    }
    return QJsonDocument(obj).toJson(QJsonDocument::Compact);
}

QString makePlainError(const QString& message) {
    return makeErrorJson(message);
}

QString makeEligibilityError(const QString& code, const QString& message) {
    QJsonObject obj;
    obj.insert(QStringLiteral("status"), QStringLiteral("error"));
    obj.insert(QStringLiteral("code"), code);
    obj.insert(QStringLiteral("message"), message);
    return QJsonDocument(obj).toJson(QJsonDocument::Compact);
}

QString makeVerifyEligibilityError(const QString& eligibility, const QString& message) {
    QJsonObject obj;
    obj.insert(QStringLiteral("status"), QStringLiteral("error"));
    obj.insert(QStringLiteral("eligibility"), eligibility);
    obj.insert(QStringLiteral("message"), message);
    return QJsonDocument(obj).toJson(QJsonDocument::Compact);
}

bool parseWalletAccountJson(const QString& accountJson,
                            QByteArray* dataOut,
                            QString* errorOut,
                            QString* balanceHexOut) {
    QJsonParseError parseError{};
    const QJsonDocument doc = QJsonDocument::fromJson(accountJson.toUtf8(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !doc.isObject()) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("wallet account JSON parse failed: %1").arg(parseError.errorString());
        }
        return false;
    }
    const QJsonObject obj = doc.object();
    const QString dataHex = obj.value(QStringLiteral("data")).toString().trimmed();
    if (dataOut != nullptr) {
        if (dataHex.isEmpty()) {
            if (errorOut != nullptr) {
                *errorOut = QStringLiteral("wallet account data field is empty");
            }
            return false;
        }
        QByteArray data = QByteArray::fromHex(dataHex.toLatin1());
        if (data.isEmpty() && !dataHex.isEmpty()) {
            if (errorOut != nullptr) {
                *errorOut = QStringLiteral("wallet account data is not valid hex");
            }
            return false;
        }
        *dataOut = data;
    }
    if (balanceHexOut != nullptr) {
        *balanceHexOut = obj.value(QStringLiteral("balance")).toString().trimmed();
    }
    return true;
}

bool hex32FromQString(const QString& hexIn, uint8_t out[32]) {
    const QByteArray hex = hexIn.trimmed().toLatin1();
    if (hex.size() != 64) {
        return false;
    }
    const QByteArray bytes = QByteArray::fromHex(hex);
    if (bytes.size() != 32) {
        return false;
    }
    std::memcpy(out, bytes.constData(), 32);
    return true;
}

quint64 variantToU64(const QVariant& value, bool* okOut) {
    bool ok = false;
    const quint64 parsed = value.toULongLong(&ok);
    if (okOut != nullptr) {
        *okOut = ok;
    }
    return parsed;
}

QString resolveRepoRelativePath(const QString& path) {
    if (QDir::isAbsolutePath(path)) {
        return path;
    }
    const QByteArray repo = qgetenv("REPO");
    if (!repo.isEmpty()) {
        return QDir(QString::fromUtf8(repo)).filePath(path);
    }
    return path;
}

bool findRepoFile(const QString& relativePath, QString* absoluteOut) {
    QDir dir(QDir::currentPath());
    for (int depth = 0; depth < 10; ++depth) {
        const QString candidate = dir.filePath(relativePath);
        if (QFile::exists(candidate)) {
            if (absoluteOut != nullptr) {
                *absoluteOut = candidate;
            }
            return true;
        }
        if (!dir.cdUp()) {
            break;
        }
    }
    const QByteArray repo = qgetenv("REPO");
    if (!repo.isEmpty()) {
        const QString candidate = QDir(QString::fromUtf8(repo)).filePath(relativePath);
        if (QFile::exists(candidate)) {
            if (absoluteOut != nullptr) {
                *absoluteOut = candidate;
            }
            return true;
        }
    }
    return false;
}

QString fixtureManifestPath() {
    const QByteArray env = qgetenv("FIXTURE_MANIFEST");
    if (!env.isEmpty()) {
        return resolveRepoRelativePath(QString::fromUtf8(env));
    }
    QString found;
    if (findRepoFile(QStringLiteral("verify/fixtures/localnet.json"), &found)) {
        return found;
    }
    return QStringLiteral("verify/fixtures/localnet.json");
}

QString walletHomeFromEnv() {
    static const char* kKeys[] = {"WALLET_HOME", "LEE_WALLET_HOME_DIR", "NSSA_WALLET_HOME_DIR"};
    for (const char* key : kKeys) {
        const QByteArray value = qgetenv(key);
        if (value.isEmpty()) {
            continue;
        }
        const QString home = QString::fromUtf8(value).trimmed();
        if (!home.isEmpty()) {
            return QDir::cleanPath(home);
        }
    }
    return {};
}

bool resolveWalletHomePaths(const QString& home, WalletHomePaths* out, QString* errorOut) {
    const QString trimmed = home.trimmed();
    if (trimmed.isEmpty()) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("WALLET_HOME is unset. Launch with make basecamp-ui-run.");
        }
        return false;
    }
    const QDir dir(trimmed);
    WalletHomePaths paths;
    paths.home = dir.absolutePath();
    paths.config = dir.filePath(QStringLiteral("wallet_config.json"));
    paths.storage = dir.filePath(QStringLiteral("storage.json"));
    paths.statistics = dir.filePath(QStringLiteral("statistics.json"));
    if (!QFile::exists(paths.config)) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("wallet_config.json missing at %1").arg(paths.config);
        }
        return false;
    }
    if (!QFile::exists(paths.storage)) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("storage.json missing at %1").arg(paths.storage);
        }
        return false;
    }
    if (out != nullptr) {
        *out = paths;
    }
    return true;
}

bool ensureWalletStatisticsFile(const QString& statisticsPath, QString* errorOut) {
    if (statisticsPath.trimmed().isEmpty()) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("wallet statistics path is empty");
        }
        return false;
    }
    if (QFile::exists(statisticsPath)) {
        return true;
    }
    QDir parent = QFileInfo(statisticsPath).absoluteDir();
    if (!parent.exists() && !parent.mkpath(QStringLiteral("."))) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("cannot create wallet statistics directory %1").arg(parent.absolutePath());
        }
        return false;
    }
    QFile file(statisticsPath);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("cannot create wallet statistics file %1").arg(statisticsPath);
        }
        return false;
    }
    file.write("{}\n");
    return true;
}

bool loadFixtureManifest(QJsonObject* out, QString* errorOut) {
    const QString path = fixtureManifestPath();
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("cannot open fixture manifest: %1").arg(path);
        }
        return false;
    }
    QJsonParseError parseError{};
    const QJsonDocument doc = QJsonDocument::fromJson(file.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !doc.isObject()) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("fixture manifest JSON parse failed");
        }
        return false;
    }
    if (out != nullptr) {
        *out = doc.object();
    }
    return true;
}

bool ffiBufferTwoPhase(const std::function<uint32_t(uint8_t*, size_t, size_t*)>& call,
                       QByteArray* out,
                       QString* errorOut,
                       const QString& sizeFail,
                       const QString& writeFail) {
    size_t required = 0;
    const auto sizing = call(nullptr, 0, &required);
    if (sizing != kFfiSuccess) {
        if (errorOut != nullptr) {
            const QString msg = sizeFail.isEmpty()
                                    ? QStringLiteral("FFI sizing failed (status %1)")
                                    : sizeFail;
            *errorOut = msg.arg(static_cast<uint>(sizing));
        }
        return false;
    }
    out->resize(static_cast<int>(required));
    const auto write = call(reinterpret_cast<uint8_t*>(out->data()), required, &required);
    if (write != kFfiSuccess) {
        if (errorOut != nullptr) {
            *errorOut = writeFail.isEmpty() ? QStringLiteral("FFI encode failed") : writeFail.arg(static_cast<uint>(write));
        }
        return false;
    }
    out->resize(static_cast<int>(required));
    return true;
}

bool isAccountIdHex64(const QString& trimmed) {
    if (trimmed.size() != kAccountIdHexLen) {
        return false;
    }
    for (QChar ch : trimmed) {
        if (!ch.isDigit() && (ch.toLower() < QLatin1Char('a') || ch.toLower() > QLatin1Char('f'))) {
            return false;
        }
    }
    return true;
}

QString accountIdHexFromField(const QString& field,
                              const std::function<QString(const QString&)>& base58ToHex,
                              QString* errorOut) {
    const QString trimmed = field.trimmed();
    if (isAccountIdHex64(trimmed)) {
        return trimmed.toLower();
    }
    const QString hex = base58ToHex ? base58ToHex(trimmed).trimmed().toLower() : QString();
    if (hex.size() != kAccountIdHexLen) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("invalid account id (expect 64-hex or base58)");
        }
        return {};
    }
    return hex;
}

bool accountIdBytesFromField(const QString& field,
                             uint8_t out[32],
                             const std::function<QString(const QString&)>& base58ToHex,
                             QString* errorOut) {
    const QString hex = accountIdHexFromField(field, base58ToHex, errorOut);
    if (hex.size() != kAccountIdHexLen) {
        return false;
    }
    return hex32FromQString(hex, out);
}

}  // namespace payment_streams_kit
