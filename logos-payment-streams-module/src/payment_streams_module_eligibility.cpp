#include "payment_streams_module_impl.h"
#include "payment_streams_module_inventory.h"

#include <QDir>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QSaveFile>
#include <QVariant>

#include <logos_api.h>
#include <logos_api_client.h>
#include <logos_sdk.h>

#include "payment_streams_ffi_bridge.h"

#include <cstring>
#include <functional>
#include <algorithm>

namespace {

constexpr uint32_t kFfiSuccess = 0u;
constexpr uint8_t kStreamStateActive = 0u;
constexpr uint8_t kStreamStatePaused = 1u;
constexpr uint8_t kStreamStateClosed = 2u;
constexpr quint64 kDemoMinRate = 1;
constexpr quint64 kDemoMinAllocation = 1;
constexpr quint64 kDemoMaxDeadlineDelay = 3600;
constexpr quint64 kDemoVaultProofMaxResponseBytes = 65536;
constexpr char kServiceId[] = "/vac/waku/store-query/3.0.0";
constexpr quint64 kDemoRate = 10;
constexpr uint32_t kMaxServiceIdLen = 128;

QString makeVerifyOk() {
    QJsonObject obj;
    obj.insert(QStringLiteral("status"), QStringLiteral("ok"));
    obj.insert(QStringLiteral("eligibility"), QStringLiteral("OK"));
    return QJsonDocument(obj).toJson(QJsonDocument::Compact);
}

QString makeVerifyEligibilityError(const QString& eligibility, const QString& message) {
    QJsonObject obj;
    obj.insert(QStringLiteral("status"), QStringLiteral("error"));
    obj.insert(QStringLiteral("eligibility"), eligibility);
    obj.insert(QStringLiteral("message"), message);
    return QJsonDocument(obj).toJson(QJsonDocument::Compact);
}

// Demo bypass: accept proofs for depleted (fully accrued) streams. Honors only truthy values
// so the always-exported default "0" means OFF; prepare and verify must agree on this.
bool allowDepletedStreamProof() {
    const QByteArray raw = qgetenv("PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF").trimmed().toLower();
    return raw == "1" || raw == "true" || raw == "yes" || raw == "on";
}

quint64 foldClockForPolicy(quint64 rawTs) {
    if (rawTs > 1'000'000'000'000ULL) {
        return rawTs / 1000;
    }
    return rawTs;
}

void fillDemoProviderPolicy(PsFfiStreamProviderPolicy* policy) {
    std::memset(policy, 0, sizeof(*policy));
    policy->min_rate = kDemoMinRate;
    policy->min_allocation_lo = kDemoMinAllocation;
    policy->max_create_stream_deadline_delay = kDemoMaxDeadlineDelay;
    policy->vault_proof_max_response_bytes = kDemoVaultProofMaxResponseBytes;
}

bool serviceIdMatchesParams(const PsFfiStreamParams& params) {
    const uint32_t len = params.service_id_len;
    if (len == 0 || len > kMaxServiceIdLen) {
        return false;
    }
    return std::memcmp(params.service_id_bytes, kServiceId, len) == 0 &&
           std::strlen(kServiceId) == len;
}

QString verdictForPolicyReject(uint32_t reason) {
    if (reason == 7u) {
        return QStringLiteral("STREAM_NOT_ACTIVE");
    }
    return QStringLiteral("PARAMS_REJECTED");
}

QString verdictForFfiStatus(uint32_t status) {
    if (status == 3u) {
        return QStringLiteral("PARAMS_REJECTED");
    }
    return QStringLiteral("PROOF_INVALID");
}

constexpr quint64 kDemoAllocationNewStream = 15;
constexpr quint64 kDemoAllocationFreshVault = 80;
constexpr quint64 kDemoDeadlineOffset = 600;

QString makeOkJson(const QJsonObject& payload) {
    QJsonObject obj;
    obj.insert(QStringLiteral("status"), QStringLiteral("ok"));
    for (auto it = payload.begin(); it != payload.end(); ++it) {
        obj.insert(it.key(), it.value());
    }
    return QJsonDocument(obj).toJson(QJsonDocument::Compact);
}

QString makeEligibilityError(const QString& code, const QString& message) {
    QJsonObject obj;
    obj.insert(QStringLiteral("status"), QStringLiteral("error"));
    obj.insert(QStringLiteral("code"), code);
    obj.insert(QStringLiteral("message"), message);
    return QJsonDocument(obj).toJson(QJsonDocument::Compact);
}

QString makePlainError(const QString& message) {
    QJsonObject obj;
    obj.insert(QStringLiteral("status"), QStringLiteral("error"));
    obj.insert(QStringLiteral("message"), message);
    return QJsonDocument(obj).toJson(QJsonDocument::Compact);
}

struct PersistedState {
    QString dir;
    QJsonObject root;
    bool dirty = false;
};

PersistedState& state() {
    static PersistedState s;
    return s;
}

QString stateFilePath() {
    const QString dir = state().dir;
    if (dir.isEmpty()) {
        return {};
    }
    return QDir(dir).filePath(QStringLiteral("payment_streams_state.json"));
}

void ensureStateSchema() {
    PersistedState& s = state();
    if (!s.root.contains(QStringLiteral("schema_version"))) {
        s.root.insert(QStringLiteral("schema_version"), 1);
    }
    if (!s.root.contains(QStringLiteral("peer_mappings"))) {
        s.root.insert(QStringLiteral("peer_mappings"), QJsonObject());
    }
    if (!s.root.contains(QStringLiteral("negotiations"))) {
        s.root.insert(QStringLiteral("negotiations"), QJsonArray());
    }
    if (!s.root.contains(QStringLiteral("inventory"))) {
        s.root.insert(QStringLiteral("inventory"), QJsonArray());
    }
    if (!s.root.contains(QStringLiteral("provider_acceptances"))) {
        s.root.insert(QStringLiteral("provider_acceptances"), QJsonArray());
    }
    const int version = s.root.value(QStringLiteral("schema_version")).toInt(1);
    if (version < 2) {
        s.root.insert(QStringLiteral("schema_version"), 2);
    }
}

void loadStateFromDisk() {
    ensureStateSchema();
    const QString path = stateFilePath();
    if (path.isEmpty()) {
        return;
    }
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        return;
    }
    QJsonParseError err{};
    const QJsonDocument doc = QJsonDocument::fromJson(file.readAll(), &err);
    if (err.error != QJsonParseError::NoError || !doc.isObject()) {
        return;
    }
    state().root = doc.object();
    ensureStateSchema();
}

bool saveStateToDisk() {
    PersistedState& s = state();
    const QString path = stateFilePath();
    if (path.isEmpty()) {
        return false;
    }
    QDir().mkpath(s.dir);
    QSaveFile file(path);
    if (!file.open(QIODevice::WriteOnly)) {
        return false;
    }
    file.write(QJsonDocument(s.root).toJson(QJsonDocument::Compact));
    if (!file.commit()) {
        return false;
    }
    s.dirty = false;
    return true;
}

void persistIfDirty() {
    if (state().dirty) {
        saveStateToDisk();
    }
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
    if (findRepoFile(QStringLiteral("fixtures/localnet.json"), &found)) {
        return found;
    }
    return QStringLiteral("fixtures/localnet.json");
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

void seedInventoryFromFixtureIfEmpty() {
    ensureStateSchema();
    QJsonArray inventory = state().root.value(QStringLiteral("inventory")).toArray();
    if (!inventory.isEmpty()) {
        return;
    }
    QJsonObject manifest;
    QString err;
    if (!loadFixtureManifest(&manifest, &err)) {
        return;
    }
    const qint64 vaultId = manifest.value(QStringLiteral("vault_id")).toInteger(0);
    const qint64 streamId = manifest.value(QStringLiteral("stream_id")).toInteger(-1);
    if (streamId < 0) {
        return;
    }
    QJsonObject row;
    row.insert(QStringLiteral("vault_id"), vaultId);
    row.insert(QStringLiteral("stream_id"), streamId);
    inventory.append(row);
    state().root.insert(QStringLiteral("inventory"), inventory);
    state().dirty = true;
    persistIfDirty();
}

QString walletAccountIdHexFromBase58(LogosExecutionZone& wallet, const QString& accountIdBase58) {
    const QString trimmed = accountIdBase58.trimmed();
    if (trimmed.isEmpty()) {
        return {};
    }
    return QString::fromStdString(wallet.account_id_from_base58(trimmed.toStdString()));
}

// Dynamic-dispatch fallback for wallet methods added by repo-local Qt
// patches (sign_public_payload, send_generic_public_transaction_json).
// These are Qt-only on the wallet side and are NOT in the codegen-emitted
// lp typed API (logos_execution_zone_api.h), so they cannot go through the
// LogosExecutionZone lp wrapper. Routed through the Qt LogosAPIClient
// (modules().api) instead. See Step 30 patched-method handling.
LogosAPIClient* walletQtClientOrNull(LogosAPI* api) {
    if (api == nullptr) {
        return nullptr;
    }
    return api->getClient(QStringLiteral("logos_execution_zone"));
}

QString invokeWalletQtString(LogosAPIClient* client, const char* method, const QVariant& a1, const QVariant& a2 = {}) {
    if (client == nullptr) {
        return {};
    }
    const QString methodName = QString::fromUtf8(method);
    QVariant result;
    if (a2.isValid() && !a2.isNull()) {
        result = client->invokeRemoteMethod(QStringLiteral("logos_execution_zone"), methodName, a1, a2);
    } else if (a1.isValid() && !a1.isNull()) {
        result = client->invokeRemoteMethod(QStringLiteral("logos_execution_zone"), methodName, a1);
    } else {
        result = client->invokeRemoteMethod(QStringLiteral("logos_execution_zone"), methodName);
    }
    if (!result.isValid()) {
        return {};
    }
    return result.toString();
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

QString bytes32ToHexLower(const uint8_t* bytes) {
    return QString::fromLatin1(QByteArray(reinterpret_cast<const char*>(bytes), 32).toHex());
}

bool parseWalletAccountJson(const QString& accountJson, QByteArray* dataOut, QString* balanceHexOut) {
    QJsonParseError parseError{};
    const QJsonDocument doc = QJsonDocument::fromJson(accountJson.toUtf8(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !doc.isObject()) {
        return false;
    }
    const QJsonObject obj = doc.object();
    const QString dataHex = obj.value(QStringLiteral("data")).toString().trimmed();
    if (dataOut != nullptr && !dataHex.isEmpty()) {
        *dataOut = QByteArray::fromHex(dataHex.toLatin1());
    }
    if (balanceHexOut != nullptr) {
        *balanceHexOut = obj.value(QStringLiteral("balance")).toString().trimmed();
    }
    return true;
}

QByteArray accountDataBytesFromHex(LogosExecutionZone& wallet, const QString& accountHex, QString* errorOut) {
    const QString accountJson = QString::fromStdString(wallet.get_account_public(accountHex.toStdString()));
    if (accountJson.isEmpty()) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("get_account_public failed");
        }
        return {};
    }
    QByteArray data;
    if (!parseWalletAccountJson(accountJson, &data, nullptr) || data.isEmpty()) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("account data missing");
        }
        return {};
    }
    return data;
}

bool programIdBytes(uint8_t out[32], QString* errorOut) {
    QJsonObject manifest;
    if (!loadFixtureManifest(&manifest, errorOut)) {
        return false;
    }
    const QString hex = manifest.value(QStringLiteral("program_id_hex")).toString().trimmed();
    if (hex.size() != 64) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("fixture program_id_hex invalid");
        }
        return false;
    }
    return hex32FromQString(hex, out);
}

bool ownerBytesFromBase58(LogosExecutionZone& wallet, const QString& base58, uint8_t out[32], QString* errorOut) {
    const QString hex = walletAccountIdHexFromBase58(wallet, base58);
    if (hex.size() != 64) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("account_id_from_base58 failed");
        }
        return false;
    }
    return hex32FromQString(hex, out);
}

bool clockBytes(uint8_t out[32], QString* errorOut) {
    if (ps_ffi_fixed_clock_10_account_id(out) != kFfiSuccess) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("clock account id FFI failed");
        }
        return false;
    }
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

bool ffiBufferTwoPhase(const std::function<uint32_t(uint8_t*, size_t, size_t*)>& call,
                       QByteArray* out,
                       QString* errorOut) {
    size_t required = 0;
    const auto sizing = call(nullptr, 0, &required);
    if (sizing != kFfiSuccess) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("FFI sizing failed (status %1)").arg(static_cast<uint>(sizing));
        }
        return false;
    }
    out->resize(static_cast<int>(required));
    if (call(reinterpret_cast<uint8_t*>(out->data()), required, &required) != kFfiSuccess) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("FFI encode failed");
        }
        return false;
    }
    out->resize(static_cast<int>(required));
    return true;
}

QString providerBase58ForPeer(const QString& peerId) {
    const QJsonObject mappings = state().root.value(QStringLiteral("peer_mappings")).toObject();
    return mappings.value(peerId.trimmed()).toString().trimmed();
}

QString providerIdHexForPeer(LogosExecutionZone& wallet, const QString& peerId, QString* errorOut) {
    const QString base58 = providerBase58ForPeer(peerId);
    if (base58.isEmpty()) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("provider peer not registered");
        }
        return {};
    }
    const QString hex = walletAccountIdHexFromBase58(wallet, base58);
    if (hex.size() != 64) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("provider account_id_from_base58 failed");
        }
        return {};
    }
    return hex.toLower();
}

QJsonArray negotiations() {
    return state().root.value(QStringLiteral("negotiations")).toArray();
}

void setNegotiations(QJsonArray arr) {
    state().root.insert(QStringLiteral("negotiations"), arr);
    state().dirty = true;
}

int findNegotiationIndex(quint64 vaultId, const QString& providerIdHex) {
    const QJsonArray arr = negotiations();
    for (int i = 0; i < arr.size(); ++i) {
        const QJsonObject row = arr.at(i).toObject();
        if (static_cast<quint64>(row.value(QStringLiteral("vault_id")).toInteger()) == vaultId &&
            row.value(QStringLiteral("provider_id_hex")).toString().toLower() == providerIdHex.toLower()) {
            return i;
        }
    }
    return -1;
}

void persistSessionForActiveStream(quint64 vaultId,
                                   const QString& providerIdHex,
                                   quint64 streamId,
                                   const uint8_t sessionSecret[32],
                                   const uint8_t sessionPublic[32]) {
    QJsonObject row;
    row.insert(QStringLiteral("vault_id"), static_cast<qint64>(vaultId));
    row.insert(QStringLiteral("provider_id_hex"), providerIdHex);
    row.insert(QStringLiteral("stream_id"), static_cast<qint64>(streamId));
    row.insert(QStringLiteral("status"), QStringLiteral("established"));
    row.insert(QStringLiteral("create_stream_deadline"), 0);
    row.insert(QStringLiteral("session_private_key_hex"),
               QString::fromLatin1(QByteArray(reinterpret_cast<const char*>(sessionSecret), 32).toHex()));
    row.insert(QStringLiteral("session_public_key_hex"),
               QString::fromLatin1(QByteArray(reinterpret_cast<const char*>(sessionPublic), 32).toHex()));
    QJsonArray arr = negotiations();
    const int existing = findNegotiationIndex(vaultId, providerIdHex);
    if (existing >= 0) {
        arr.replace(existing, row);
    } else {
        arr.append(row);
    }
    setNegotiations(arr);
    state().dirty = true;
}

bool sessionKeysForVaultProvider(quint64 vaultId,
                                 const QString& providerIdHex,
                                 uint8_t outSecret[32],
                                 uint8_t outPublic[32]) {
    const QJsonArray arr = negotiations();
    for (const QJsonValue& v : arr) {
        const QJsonObject row = v.toObject();
        if (static_cast<quint64>(row.value(QStringLiteral("vault_id")).toInteger()) != vaultId) {
            continue;
        }
        if (row.value(QStringLiteral("provider_id_hex")).toString().toLower() != providerIdHex.toLower()) {
            continue;
        }
        const QByteArray sec =
            QByteArray::fromHex(row.value(QStringLiteral("session_private_key_hex")).toString().toLatin1());
        const QByteArray pub =
            QByteArray::fromHex(row.value(QStringLiteral("session_public_key_hex")).toString().toLatin1());
        if (sec.size() == 32 && pub.size() == 32) {
            std::memcpy(outSecret, sec.constData(), 32);
            std::memcpy(outPublic, pub.constData(), 32);
            return true;
        }
    }
    return false;
}

void removeNegotiationAt(int index) {
    QJsonArray arr = negotiations();
    if (index < 0 || index >= arr.size()) {
        return;
    }
    arr.removeAt(index);
    setNegotiations(arr);
}

bool inventoryContains(quint64 vaultId, quint64 streamId) {
    const QJsonArray arr = state().root.value(QStringLiteral("inventory")).toArray();
    for (const QJsonValue& v : arr) {
        const QJsonObject row = v.toObject();
        if (static_cast<quint64>(row.value(QStringLiteral("vault_id")).toInteger()) == vaultId &&
            static_cast<quint64>(row.value(QStringLiteral("stream_id")).toInteger()) == streamId) {
            return true;
        }
    }
    return false;
}

void addInventory(quint64 vaultId, quint64 streamId) {
    if (inventoryContains(vaultId, streamId)) {
        return;
    }
    QJsonArray arr = state().root.value(QStringLiteral("inventory")).toArray();
    QJsonObject row;
    row.insert(QStringLiteral("vault_id"), static_cast<qint64>(vaultId));
    row.insert(QStringLiteral("stream_id"), static_cast<qint64>(streamId));
    arr.append(row);
    state().root.insert(QStringLiteral("inventory"), arr);
    state().dirty = true;
}

QList<quint64> inventoryStreamIdsForVault(quint64 vaultId) {
    QList<quint64> ids;
    const QJsonArray arr = state().root.value(QStringLiteral("inventory")).toArray();
    for (const QJsonValue& v : arr) {
        const QJsonObject row = v.toObject();
        if (static_cast<quint64>(row.value(QStringLiteral("vault_id")).toInteger()) == vaultId) {
            ids.append(static_cast<quint64>(row.value(QStringLiteral("stream_id")).toInteger()));
        }
    }
    return ids;
}

bool readClock10Timestamp(LogosExecutionZone& wallet, quint64* outTs, QString* errorOut) {
    uint8_t clock[32]{};
    if (!clockBytes(clock, errorOut)) {
        return false;
    }
    const QString clockHex = bytes32ToHexLower(clock);
    const QByteArray data = accountDataBytesFromHex(wallet, clockHex, errorOut);
    if (data.isEmpty()) {
        return false;
    }
    PsFfiDecodedClock decoded{};
    if (ps_ffi_decode_clock(reinterpret_cast<const uint8_t*>(data.constData()),
                            static_cast<size_t>(data.size()),
                            &decoded) != 0u) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("clock decode failed");
        }
        return false;
    }
    *outTs = foldClockForPolicy(decoded.timestamp);
    return true;
}

void evictExpiredNegotiations(LogosExecutionZone& wallet, bool* evictedOut = nullptr) {
    if (evictedOut != nullptr) {
        *evictedOut = false;
    }
    quint64 now = 0;
    QString err;
    if (!readClock10Timestamp(wallet, &now, &err)) {
        return;
    }
    QJsonArray arr = negotiations();
    QJsonArray kept;
    for (const QJsonValue& v : arr) {
        const QJsonObject row = v.toObject();
        const QString status = row.value(QStringLiteral("status")).toString();
        const qint64 deadline = row.value(QStringLiteral("create_stream_deadline")).toInteger();
        if (status == QLatin1String("pending") && deadline > 0 && static_cast<quint64>(deadline) <= now) {
            if (evictedOut != nullptr) {
                *evictedOut = true;
            }
            continue;
        }
        kept.append(row);
    }
    if (kept.size() != arr.size()) {
        setNegotiations(kept);
        persistIfDirty();
    }
}

struct ChainStreamView {
    bool found = false;
    quint64 streamId = 0;
    PsFfiDecodedStreamConfig decoded{};
    PsFfiStreamFoldAtTime fold{};
    quint64 asOf = 0;
};

bool readStreamAtId(LogosExecutionZone& wallet,
                    const uint8_t programId[32],
                    const uint8_t owner[32],
                    quint64 vaultId,
                    quint64 streamId,
                    ChainStreamView* out,
                    QString* errorOut) {
    uint8_t vaultCfg[32]{};
    uint8_t streamCfg[32]{};
    uint8_t vaultHolding[32]{};
    if (ps_ffi_derive_vault_account_ids(programId, owner, vaultId, vaultCfg, vaultHolding) != kFfiSuccess ||
        ps_ffi_derive_stream_config_account_id(programId, vaultCfg, streamId, streamCfg) != kFfiSuccess) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("derive stream PDA failed");
        }
        return false;
    }
    const QString streamHex = bytes32ToHexLower(streamCfg);
    const QByteArray streamData = accountDataBytesFromHex(wallet, streamHex, errorOut);
    if (streamData.isEmpty()) {
        return true;
    }
    PsFfiDecodedStreamConfig decoded{};
    if (ps_ffi_decode_stream_config(reinterpret_cast<const uint8_t*>(streamData.constData()),
                                    static_cast<size_t>(streamData.size()),
                                    &decoded) != 0u) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("stream config decode failed");
        }
        return false;
    }
    quint64 asOf = 0;
    if (!readClock10Timestamp(wallet, &asOf, errorOut)) {
        return false;
    }
    PsFfiStreamFoldAtTime fold{};
    uint32_t guestError = 0;
    if (ps_ffi_fold_stream_at(&decoded, asOf, &fold, &guestError) != kFfiSuccess) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("stream fold failed");
        }
        return false;
    }
    if (out != nullptr) {
        out->found = true;
        out->streamId = streamId;
        out->decoded = decoded;
        out->fold = fold;
        out->asOf = asOf;
    }
    return true;
}

bool providerMatchesStream(const PsFfiDecodedStreamConfig& decoded, const uint8_t provider[32]) {
    return std::memcmp(decoded.provider, provider, 32) == 0;
}

QString eligibilityErrorForStreamState(const ChainStreamView& view) {
    if (view.decoded.stream_state == kStreamStateClosed) {
        return makeEligibilityError(QStringLiteral("STREAM_CLOSED"),
                                    QStringLiteral("stream is closed on chain"));
    }
    if (view.decoded.stream_state == kStreamStatePaused) {
        return makeEligibilityError(QStringLiteral("STREAM_PAUSED"),
                                    QStringLiteral("stream is paused on chain"));
    }
    if (view.fold.unaccrued_lo == 0 && view.fold.unaccrued_hi == 0) {
        if (allowDepletedStreamProof()) {
            return {};
        }
        return makeEligibilityError(QStringLiteral("STREAM_DEPLETED"),
                                    QStringLiteral("stream allocation fully accrued"));
    }
    if (view.decoded.stream_state != kStreamStateActive) {
        return makeEligibilityError(QStringLiteral("STREAM_NOT_CONFIRMED"),
                                    QStringLiteral("stream not active on chain"));
    }
    return {};
}

bool ownerPublicKeyHex(LogosExecutionZone& wallet, const QString& ownerHex, QString* outHex, QString* errorOut) {
    const QString json = QString::fromStdString(wallet.get_public_account_key(ownerHex.toStdString()));
    if (json.isEmpty()) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("get_public_account_key failed");
        }
        return false;
    }
    const QString trimmed = json.trimmed().toLower();
    if (trimmed.size() == 64 && trimmed.indexOf(QLatin1Char('{')) < 0) {
        if (outHex != nullptr) {
            *outHex = trimmed;
        }
        return true;
    }
    QJsonParseError parseError{};
    const QJsonDocument doc = QJsonDocument::fromJson(json.toUtf8(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !doc.isObject()) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("get_public_account_key response invalid");
        }
        return false;
    }
    const QJsonObject obj = doc.object();
    const QString keyHex = obj.value(QStringLiteral("result")).toString().trimmed().toLower();
    if (keyHex.size() != 64) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("owner public key hex invalid");
        }
        return false;
    }
    if (outHex != nullptr) {
        *outHex = keyHex;
    }
    return true;
}

bool ownerVaultProofPublicKeyHex(LogosExecutionZone& wallet,
                                 const QString& ownerHex,
                                 uint8_t vaultPrivacyTier,
                                 QString* outHex,
                                 bool* usePrivateOwnerKeyOut,
                                 QString* errorOut) {
    if (usePrivateOwnerKeyOut != nullptr) {
        *usePrivateOwnerKeyOut = vaultPrivacyTier == 1;
    }
    if (vaultPrivacyTier == 1) {
        const QString keysJson = QString::fromStdString(wallet.get_private_account_keys(ownerHex.toStdString()));
        if (keysJson.isEmpty()) {
            if (errorOut != nullptr) {
                *errorOut = QStringLiteral("get_private_account_keys failed");
            }
            return false;
        }
        QJsonParseError parseError{};
        const QJsonDocument doc = QJsonDocument::fromJson(keysJson.toUtf8(), &parseError);
        if (parseError.error != QJsonParseError::NoError || !doc.isObject()) {
            if (errorOut != nullptr) {
                *errorOut = QStringLiteral("get_private_account_keys response invalid");
            }
            return false;
        }
        const QString npkHex =
            doc.object().value(QStringLiteral("nullifier_public_key")).toString().trimmed().toLower();
        if (npkHex.size() != 64) {
            if (errorOut != nullptr) {
                *errorOut = QStringLiteral("owner NPK hex invalid");
            }
            return false;
        }
        if (outHex != nullptr) {
            *outHex = npkHex;
        }
        return true;
    }
    return ownerPublicKeyHex(wallet, ownerHex, outHex, errorOut);
}

bool signVaultOwnerDigest(LogosAPI* api,
                          const QString& ownerAccountHex,
                          const uint8_t digest[32],
                          uint8_t outSig[64],
                          bool usePrivateOwnerKey,
                          QString* errorOut) {
    const QString digestHex =
        QString::fromLatin1(QByteArray(reinterpret_cast<const char*>(digest), 32).toHex());
    LogosAPIClient* qtClient = walletQtClientOrNull(api);
    const char* method = usePrivateOwnerKey ? "sign_private_payload" : "sign_public_payload";
    const QString response = invokeWalletQtString(qtClient, method, ownerAccountHex, digestHex);
    if (response.isEmpty()) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("%1 IPC failed").arg(QString::fromUtf8(method));
        }
        return false;
    }
    QJsonParseError parseError{};
    const QJsonDocument doc = QJsonDocument::fromJson(response.toUtf8(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !doc.isObject()) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("sign_public_payload JSON parse failed");
        }
        return false;
    }
    const QJsonObject obj = doc.object();
    if (obj.value(QStringLiteral("status")).toString() != QLatin1String("ok")) {
        if (errorOut != nullptr) {
            *errorOut = obj.value(QStringLiteral("error")).toString();
        }
        return false;
    }
    const QByteArray sig = QByteArray::fromHex(obj.value(QStringLiteral("result")).toString().toLatin1());
    if (sig.size() != 64) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("sign_public_payload result length invalid");
        }
        return false;
    }
    std::memcpy(outSig, sig.constData(), 64);
    return true;
}

// Only populates the service_id fields. Callers own rate/allocation/deadline: the verify path
// must keep the on-chain values set by paramsFromStreamConfig, while the fresh-vault proposal arm
// sets its own demo terms. (Previously this clobbered rate/allocation, making verify reject
// every stream proof with RateBelowAcceptedParams since chain rate < kDemoRate.)
void fillServiceId(PsFfiStreamParams* params) {
    const size_t len = std::strlen(kServiceId);
    params->service_id_len = static_cast<uint32_t>(len);
    params->_padding = 0;
    std::memset(params->service_id_bytes, 0, sizeof(params->service_id_bytes));
    std::memcpy(params->service_id_bytes, kServiceId, len);
}

quint64 u128LoFromHexBalance(const QString& balanceHex) {
    const QByteArray bytes = QByteArray::fromHex(balanceHex.toLatin1());
    if (bytes.size() < 8) {
        return 0;
    }
    quint64 lo = 0;
    for (int i = 0; i < 8 && i < bytes.size(); ++i) {
        lo |= static_cast<quint64>(static_cast<unsigned char>(bytes[i])) << (8 * i);
    }
    return lo;
}

QJsonArray providerAcceptancesArray() {
    return state().root.value(QStringLiteral("provider_acceptances")).toArray();
}

void setProviderAcceptancesArray(QJsonArray arr) {
    state().root.insert(QStringLiteral("provider_acceptances"), arr);
    state().dirty = true;
}

int findProviderAcceptanceIndex(quint64 vaultId, const QString& providerIdHex) {
    const QJsonArray arr = providerAcceptancesArray();
    for (int i = 0; i < arr.size(); ++i) {
        const QJsonObject row = arr.at(i).toObject();
        if (static_cast<quint64>(row.value(QStringLiteral("vault_id")).toInteger()) == vaultId &&
            row.value(QStringLiteral("provider_id_hex")).toString().toLower() == providerIdHex.toLower()) {
            return i;
        }
    }
    return -1;
}

bool sessionPublicKeyFromNegotiations(quint64 vaultId,
                                      const QString& providerIdHex,
                                      uint8_t outSessionPublic[32]) {
    const QJsonArray arr = negotiations();
    for (const QJsonValue& v : arr) {
        const QJsonObject row = v.toObject();
        if (static_cast<quint64>(row.value(QStringLiteral("vault_id")).toInteger()) != vaultId) {
            continue;
        }
        if (row.value(QStringLiteral("provider_id_hex")).toString().toLower() != providerIdHex.toLower()) {
            continue;
        }
        const QString hex = row.value(QStringLiteral("session_public_key_hex")).toString().trimmed();
        return hex32FromQString(hex, outSessionPublic);
    }
    return false;
}

bool demoProviderIdBytes(LogosExecutionZone& wallet, const QJsonObject& manifest, uint8_t out[32], QString* errorOut) {
    const QString base58 = manifest.value(QStringLiteral("provider_account_id")).toString().trimmed();
    return ownerBytesFromBase58(wallet, base58, out, errorOut);
}

void paramsFromStreamConfig(const PsFfiDecodedStreamConfig& decoded, PsFfiStreamParams* params) {
    std::memset(params, 0, sizeof(*params));
    params->rate = decoded.rate_tokens_per_second;
    params->allocation_lo = decoded.allocation_lo;
    params->allocation_hi = decoded.allocation_hi;
    fillServiceId(params);
    params->create_stream_deadline = 0;
}

void fillAcceptedTermsFromParams(const PsFfiStreamParams& params,
                                 const uint8_t providerId[32],
                                 PsFfiAcceptedStreamTerms* terms) {
    std::memset(terms, 0, sizeof(*terms));
    std::memcpy(&terms->params, &params, sizeof(params));
    std::memcpy(terms->provider_id, providerId, 32);
    fillDemoProviderPolicy(&terms->policy_at_acceptance);
}

}  // namespace

void paymentStreamsModuleOnContextReady(const char* persistenceDirUtf8) {
    if (persistenceDirUtf8 == nullptr || persistenceDirUtf8[0] == '\0') {
        state().dir.clear();
        ensureStateSchema();
        seedInventoryFromFixtureIfEmpty();
        return;
    }
    state().dir = QString::fromUtf8(persistenceDirUtf8);
    loadStateFromDisk();
    seedInventoryFromFixtureIfEmpty();
}

void paymentStreamsModuleRecordStreamInventory(uint64_t vaultId, uint64_t streamId) {
    addInventory(vaultId, streamId);
    persistIfDirty();
}

void PaymentStreamsModuleImpl::onContextReady() {
    paymentStreamsModuleOnContextReady(instancePersistencePath().c_str());
}

QString PaymentStreamsModuleImpl::registerProviderMapping(const QVariant& providerPeerId,
                                                          const QVariant& providerAccountIdBase58) {
    const QString peer = providerPeerId.toString().trimmed();
    const QString base58 = providerAccountIdBase58.toString().trimmed();
    if (peer.isEmpty() || base58.isEmpty()) {
        return makePlainError(QStringLiteral("provider_peer_id and provider account are required"));
    }
    ensureStateSchema();
    QJsonObject mappings = state().root.value(QStringLiteral("peer_mappings")).toObject();
    mappings.insert(peer, base58);
    state().root.insert(QStringLiteral("peer_mappings"), mappings);
    state().dirty = true;
    persistIfDirty();
    return makeOkJson({});
}

QString PaymentStreamsModuleImpl::prepareEligibilityProofWithStreamProposalForStoreQuery(const QVariant& canonicalRequestHex,
                                                                          const QVariant& providerPeerId) {
    LogosExecutionZone& wallet = modules().logos_execution_zone;

    const QString peer = providerPeerId.toString().trimmed();
    QString mapErr;
    const QString providerIdHex = providerIdHexForPeer(wallet, peer, &mapErr);
    if (providerIdHex.isEmpty()) {
        return makeEligibilityError(QStringLiteral("UNKNOWN_PROVIDER"), mapErr);
    }

    const QByteArray n8Wire = QByteArray::fromHex(canonicalRequestHex.toString().trimmed().toLatin1());
    if (n8Wire.isEmpty()) {
        return makePlainError(QStringLiteral("canonical_request_hex must be non-empty even-length hex"));
    }

    QJsonObject manifest;
    QString fixtureErr;
    if (!loadFixtureManifest(&manifest, &fixtureErr)) {
        return makePlainError(fixtureErr);
    }
    const QString ownerBase58 = manifest.value(QStringLiteral("owner_account_id")).toString().trimmed();
    const quint64 vaultId = static_cast<quint64>(manifest.value(QStringLiteral("vault_id")).toInteger(0));

    quint64 now = 0;
    QString clockErr;
    if (!readClock10Timestamp(wallet, &now, &clockErr)) {
        return makeEligibilityError(QStringLiteral("CHAIN_READ_FAILED"), clockErr);
    }

    const int negIdxBeforeEvict = findNegotiationIndex(vaultId, providerIdHex);
    if (negIdxBeforeEvict >= 0) {
        const QJsonObject row = negotiations().at(negIdxBeforeEvict).toObject();
        const QString status = row.value(QStringLiteral("status")).toString();
        const qint64 deadline = row.value(QStringLiteral("create_stream_deadline")).toInteger();
        if (status == QLatin1String("pending") && deadline > 0 && static_cast<quint64>(deadline) <= now) {
            removeNegotiationAt(negIdxBeforeEvict);
            persistIfDirty();
            return makeEligibilityError(QStringLiteral("PROPOSAL_EXPIRED"),
                                        QStringLiteral("pending proposal past create_stream_deadline"));
        }
        if (status == QLatin1String("pending")) {
            return makeEligibilityError(QStringLiteral("PROPOSAL_PENDING"),
                                        QStringLiteral("stream proposal already issued for this provider"));
        }
    }

    bool evicted = false;
    evictExpiredNegotiations(wallet, &evicted);
    if (evicted) {
        return makeEligibilityError(QStringLiteral("PROPOSAL_EXPIRED"),
                                    QStringLiteral("stale pending proposal evicted"));
    }

    uint8_t programId[32]{};
    uint8_t owner[32]{};
    uint8_t provider[32]{};
    if (!programIdBytes(programId, &fixtureErr) || !ownerBytesFromBase58(wallet, ownerBase58, owner, &fixtureErr) ||
        !hex32FromQString(providerIdHex, provider)) {
        return makePlainError(fixtureErr);
    }

    const QString ownerHex = walletAccountIdHexFromBase58(wallet, ownerBase58).toLower();
    uint8_t vaultCfgAccount[32]{};
    uint8_t vaultHoldingAccount[32]{};
    if (ps_ffi_derive_vault_account_ids(programId, owner, vaultId, vaultCfgAccount, vaultHoldingAccount) !=
        kFfiSuccess) {
        return makeEligibilityError(QStringLiteral("CHAIN_READ_FAILED"), QStringLiteral("derive vault accounts failed"));
    }

    const QByteArray vaultCfgData =
        accountDataBytesFromHex(wallet, bytes32ToHexLower(vaultCfgAccount), &fixtureErr);
    if (vaultCfgData.isEmpty()) {
        return makeEligibilityError(QStringLiteral("CHAIN_READ_FAILED"), fixtureErr);
    }
    PsFfiDecodedVaultConfig vaultCfg{};
    if (ps_ffi_decode_vault_config(reinterpret_cast<const uint8_t*>(vaultCfgData.constData()),
                                   static_cast<size_t>(vaultCfgData.size()),
                                   &vaultCfg) != 0u) {
        return makeEligibilityError(QStringLiteral("CHAIN_READ_FAILED"), QStringLiteral("vault config decode failed"));
    }

    const QString holdingJson = QString::fromStdString(wallet.get_account_public(bytes32ToHexLower(vaultHoldingAccount).toStdString()));
    QString balanceHex;
    if (!parseWalletAccountJson(holdingJson, nullptr, &balanceHex)) {
        return makeEligibilityError(QStringLiteral("CHAIN_READ_FAILED"), QStringLiteral("vault holding read failed"));
    }
    const quint64 holdingLo = u128LoFromHexBalance(balanceHex);
    const quint64 totalAllocatedLo = vaultCfg.total_allocated_lo;
    const quint64 unallocated = holdingLo > totalAllocatedLo ? holdingLo - totalAllocatedLo : 0;

    const quint64 proposalAllocation =
        vaultCfg.next_stream_id > 0 ? kDemoAllocationNewStream : kDemoAllocationFreshVault;
    if (unallocated < proposalAllocation) {
        return makeEligibilityError(QStringLiteral("NO_ELIGIBLE_VAULT"),
                                    QStringLiteral("insufficient unallocated vault balance for proposal"));
    }

    if (!readClock10Timestamp(wallet, &now, &fixtureErr)) {
        return makeEligibilityError(QStringLiteral("CHAIN_READ_FAILED"), fixtureErr);
    }

    uint8_t sessionSecret[32]{};
    uint8_t sessionPublic[32]{};
    if (ps_ffi_generate_session_keypair(sessionSecret, sessionPublic) != kFfiSuccess) {
        return makePlainError(QStringLiteral("session key generation failed"));
    }

    QString ownerPubHex;
    bool usePrivateOwnerKey = false;
    if (!ownerVaultProofPublicKeyHex(wallet, ownerHex, vaultCfg.privacy_tier, &ownerPubHex, &usePrivateOwnerKey, &fixtureErr)) {
        return makeEligibilityError(QStringLiteral("WALLET_SIGNING_FAILED"), fixtureErr);
    }

    PsFfiDecodedStreamProposal proposal{};
    std::memset(&proposal, 0, sizeof(proposal));
    proposal.vault_proof.vault_id = vaultId;
    std::memcpy(proposal.vault_proof.provider_id, provider, 32);
    hex32FromQString(ownerPubHex, proposal.vault_proof.owner_public_key);
    fillServiceId(&proposal.params);
    proposal.params.rate = kDemoRate;
    proposal.params.allocation_lo = proposalAllocation;
    const quint64 policyNow = foldClockForPolicy(now);
    proposal.params.create_stream_deadline = policyNow + kDemoDeadlineOffset;
    std::memcpy(proposal.session_public_key, sessionPublic, 32);

    uint8_t ownerDigest[32]{};
    if (ps_ffi_vault_owner_auth_digest_from_decoded_proposal(&proposal, ownerDigest) != kFfiSuccess) {
        return makePlainError(QStringLiteral("vault owner digest FFI failed"));
    }
    if (!signVaultOwnerDigest(modules().api,
                              ownerHex,
                              ownerDigest,
                              proposal.vault_proof.owner_signature,
                              usePrivateOwnerKey,
                              &fixtureErr)) {
        return makeEligibilityError(QStringLiteral("WALLET_SIGNING_FAILED"), fixtureErr);
    }

    QByteArray innerProposal;
    QString encErr;
    if (!ffiBufferTwoPhase(
            [&](uint8_t* ptr, size_t cap, size_t* len) {
                return ps_ffi_serialize_stream_proposal_decoded(&proposal, ptr, cap, len);
            },
            &innerProposal,
            &encErr)) {
        return makePlainError(encErr);
    }

    QByteArray wrapped;
    if (!ffiBufferTwoPhase(
            [&](uint8_t* ptr, size_t cap, size_t* len) {
                return ps_ffi_serialize_eligibility_proof_stream_proposal(
                    reinterpret_cast<const uint8_t*>(innerProposal.constData()),
                    static_cast<size_t>(innerProposal.size()),
                    ptr,
                    cap,
                    len);
            },
            &wrapped,
            &encErr)) {
        return makePlainError(encErr);
    }

    const quint64 streamId = vaultCfg.next_stream_id;
    QJsonObject neg;
    neg.insert(QStringLiteral("vault_id"), static_cast<qint64>(vaultId));
    neg.insert(QStringLiteral("provider_id_hex"), providerIdHex);
    neg.insert(QStringLiteral("stream_id"), static_cast<qint64>(streamId));
    neg.insert(QStringLiteral("status"), QStringLiteral("pending"));
    neg.insert(QStringLiteral("create_stream_deadline"), static_cast<qint64>(proposal.params.create_stream_deadline));
    neg.insert(QStringLiteral("session_private_key_hex"),
               QString::fromLatin1(QByteArray(reinterpret_cast<const char*>(sessionSecret), 32).toHex()));
    neg.insert(QStringLiteral("session_public_key_hex"),
               QString::fromLatin1(QByteArray(reinterpret_cast<const char*>(sessionPublic), 32).toHex()));

    QJsonArray arr = negotiations();
    const int existing = findNegotiationIndex(vaultId, providerIdHex);
    if (existing >= 0) {
        arr.replace(existing, neg);
    } else {
        arr.append(neg);
    }
    setNegotiations(arr);
    persistIfDirty();

    QJsonObject payload;
    payload.insert(QStringLiteral("kind"), QStringLiteral("stream_proposal"));
    payload.insert(QStringLiteral("bytes_hex"), QString::fromLatin1(wrapped.toHex()));
    payload.insert(QStringLiteral("stream_id"), static_cast<qint64>(streamId));
    payload.insert(QStringLiteral("vault_id"), static_cast<qint64>(vaultId));
    return makeOkJson(payload);
}

QString PaymentStreamsModuleImpl::prepareEligibilityProofWithStreamProofForStoreQuery(const QVariant& canonicalRequestHex,
                                                                       const QVariant& providerPeerId,
                                                                       const QVariant& streamIdVariant) {
    LogosExecutionZone& wallet = modules().logos_execution_zone;

    const QString peer = providerPeerId.toString().trimmed();
    QString mapErr;
    const QString providerIdHex = providerIdHexForPeer(wallet, peer, &mapErr);
    if (providerIdHex.isEmpty()) {
        return makeEligibilityError(QStringLiteral("UNKNOWN_PROVIDER"), mapErr);
    }

    const QByteArray n8Wire = QByteArray::fromHex(canonicalRequestHex.toString().trimmed().toLatin1());
    if (n8Wire.isEmpty()) {
        return makePlainError(QStringLiteral("canonical_request_hex must be non-empty even-length hex"));
    }

    bool streamIdOk = false;
    const quint64 streamId = variantToU64(streamIdVariant, &streamIdOk);
    if (!streamIdOk) {
        return makePlainError(QStringLiteral("stream_id must be a non-negative integer"));
    }

    QJsonObject manifest;
    QString fixtureErr;
    if (!loadFixtureManifest(&manifest, &fixtureErr)) {
        return makePlainError(fixtureErr);
    }
    const QString ownerBase58 = manifest.value(QStringLiteral("owner_account_id")).toString().trimmed();
    const quint64 vaultId = static_cast<quint64>(manifest.value(QStringLiteral("vault_id")).toInteger(0));

    uint8_t programId[32]{};
    uint8_t owner[32]{};
    uint8_t provider[32]{};
    if (!programIdBytes(programId, &fixtureErr) || !ownerBytesFromBase58(wallet, ownerBase58, owner, &fixtureErr) ||
        !hex32FromQString(providerIdHex, provider)) {
        return makePlainError(fixtureErr);
    }

    ChainStreamView view;
    if (!readStreamAtId(wallet, programId, owner, vaultId, streamId, &view, &fixtureErr)) {
        return makeEligibilityError(QStringLiteral("CHAIN_READ_FAILED"), fixtureErr);
    }
    if (!view.found) {
        return makeEligibilityError(QStringLiteral("STREAM_NOT_FOUND"),
                                    QStringLiteral("stream not initialized on chain"));
    }
    if (!providerMatchesStream(view.decoded, provider)) {
        return makeEligibilityError(QStringLiteral("UNKNOWN_PROVIDER"),
                                    QStringLiteral("stream provider does not match mapping"));
    }

    const QString streamErr = eligibilityErrorForStreamState(view);
    if (!streamErr.isEmpty()) {
        return streamErr;
    }

    uint8_t sessionSecret[32]{};
    uint8_t sessionPublic[32]{};
    if (!sessionKeysForVaultProvider(vaultId, providerIdHex, sessionSecret, sessionPublic)) {
        if (ps_ffi_generate_session_keypair(sessionSecret, sessionPublic) != kFfiSuccess) {
            return makePlainError(QStringLiteral("session key generation failed"));
        }
        persistSessionForActiveStream(vaultId, providerIdHex, streamId, sessionSecret, sessionPublic);
        persistIfDirty();
    }

    QByteArray innerProof;
    QString encErr;
    if (!ffiBufferTwoPhase(
            [&](uint8_t* ptr, size_t cap, size_t* len) {
                return ps_ffi_serialize_stream_proof_for_n8_wire(
                    streamId,
                    sessionSecret,
                    reinterpret_cast<const uint8_t*>(n8Wire.constData()),
                    static_cast<size_t>(n8Wire.size()),
                    ptr,
                    cap,
                    len);
            },
            &innerProof,
            &encErr)) {
        return makePlainError(QStringLiteral("%1 (n8_wire_bytes=%2)").arg(encErr).arg(n8Wire.size()));
    }

    QByteArray wrapped;
    if (!ffiBufferTwoPhase(
            [&](uint8_t* ptr, size_t cap, size_t* len) {
                return ps_ffi_serialize_eligibility_proof_stream_proof(
                    reinterpret_cast<const uint8_t*>(innerProof.constData()),
                    static_cast<size_t>(innerProof.size()),
                    ptr,
                    cap,
                    len);
            },
            &wrapped,
            &encErr)) {
        return makePlainError(encErr);
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("kind"), QStringLiteral("stream_proof"));
    payload.insert(QStringLiteral("bytes_hex"), QString::fromLatin1(wrapped.toHex()));
    payload.insert(QStringLiteral("stream_id"), static_cast<qint64>(streamId));
    payload.insert(QStringLiteral("vault_id"), static_cast<qint64>(vaultId));
    addInventory(vaultId, streamId);
    persistIfDirty();
    return makeOkJson(payload);
}

QString PaymentStreamsModuleImpl::listMyStreams(const QVariant& vaultId) {
    LogosExecutionZone& wallet = modules().logos_execution_zone;
    bool ok = false;
    const quint64 vid = variantToU64(vaultId, &ok);
    if (!ok) {
        return makePlainError(QStringLiteral("vaultId must be unsigned integer"));
    }

    evictExpiredNegotiations(wallet);

    QJsonObject manifest;
    QString err;
    if (!loadFixtureManifest(&manifest, &err)) {
        return makePlainError(err);
    }
    const QString ownerBase58 = manifest.value(QStringLiteral("owner_account_id")).toString().trimmed();
    uint8_t programId[32]{};
    uint8_t owner[32]{};
    if (!programIdBytes(programId, &err) || !ownerBytesFromBase58(wallet, ownerBase58, owner, &err)) {
        return makePlainError(err);
    }

    QJsonArray streams;
    for (quint64 sid : inventoryStreamIdsForVault(vid)) {
        ChainStreamView view;
        if (!readStreamAtId(wallet, programId, owner, vid, sid, &view, &err)) {
            return makeEligibilityError(QStringLiteral("CHAIN_READ_FAILED"), err);
        }
        QJsonObject row;
        row.insert(QStringLiteral("stream_id"), static_cast<qint64>(sid));
        row.insert(QStringLiteral("vault_id"), static_cast<qint64>(vid));
        if (!view.found) {
            row.insert(QStringLiteral("on_chain"), false);
        } else {
            row.insert(QStringLiteral("on_chain"), true);
            row.insert(QStringLiteral("stream_state"), static_cast<qint64>(view.decoded.stream_state));
            row.insert(QStringLiteral("provider_hex"), bytes32ToHexLower(view.decoded.provider));
            row.insert(QStringLiteral("as_of"), static_cast<qint64>(view.asOf));
            row.insert(QStringLiteral("accrued_lo"), static_cast<qint64>(view.fold.accrued_lo));
            row.insert(QStringLiteral("unaccrued_lo"), static_cast<qint64>(view.fold.unaccrued_lo));
        }
        streams.append(row);
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("streams"), streams);
    return makeOkJson(payload);
}

QString PaymentStreamsModuleImpl::rediscoverStreams(const QVariant& vaultId) {
    LogosExecutionZone& wallet = modules().logos_execution_zone;
    bool ok = false;
    const quint64 vid = variantToU64(vaultId, &ok);
    if (!ok) {
        return makePlainError(QStringLiteral("vaultId must be unsigned integer"));
    }

    QJsonObject manifest;
    QString err;
    if (!loadFixtureManifest(&manifest, &err)) {
        return makePlainError(err);
    }
    const QString ownerBase58 = manifest.value(QStringLiteral("owner_account_id")).toString().trimmed();
    uint8_t programId[32]{};
    uint8_t owner[32]{};
    uint8_t vaultCfg[32]{};
    uint8_t vaultHolding[32]{};
    if (!programIdBytes(programId, &err) || !ownerBytesFromBase58(wallet, ownerBase58, owner, &err)) {
        return makePlainError(err);
    }
    if (ps_ffi_derive_vault_account_ids(programId, owner, vid, vaultCfg, vaultHolding) != kFfiSuccess) {
        return makePlainError(QStringLiteral("derive vault config failed"));
    }
    const QByteArray cfgData = accountDataBytesFromHex(wallet, bytes32ToHexLower(vaultCfg), &err);
    if (cfgData.isEmpty()) {
        return makeEligibilityError(QStringLiteral("CHAIN_READ_FAILED"), err);
    }
    PsFfiDecodedVaultConfig decodedCfg{};
    if (ps_ffi_decode_vault_config(reinterpret_cast<const uint8_t*>(cfgData.constData()),
                                   static_cast<size_t>(cfgData.size()),
                                   &decodedCfg) != 0u) {
        return makePlainError(QStringLiteral("vault config decode failed"));
    }

    quint64 discovered = 0;
    QJsonArray streams;
    for (quint64 sid = 0; sid < decodedCfg.next_stream_id; ++sid) {
        ChainStreamView view;
        if (!readStreamAtId(wallet, programId, owner, vid, sid, &view, &err)) {
            return makeEligibilityError(QStringLiteral("CHAIN_READ_FAILED"), err);
        }
        if (!view.found) {
            break;
        }
        addInventory(vid, sid);
        ++discovered;
        QJsonObject row;
        row.insert(QStringLiteral("stream_id"), static_cast<qint64>(sid));
        row.insert(QStringLiteral("stream_state"), static_cast<qint64>(view.decoded.stream_state));
        streams.append(row);
    }
    persistIfDirty();

    QJsonObject payload;
    payload.insert(QStringLiteral("streams"), streams);
    payload.insert(QStringLiteral("discovered_count"), static_cast<qint64>(discovered));
    return makeOkJson(payload);
}

QString PaymentStreamsModuleImpl::verifyEligibilityForStoreQuery(const QVariant& proofBytes,
                                                                 const QVariant& canonicalRequestBytes,
                                                                 const QVariant& requesterPeerId) {
    const QString peer = requesterPeerId.toString().trimmed();
    if (peer.isEmpty()) {
        return makePlainError(QStringLiteral("requesterPeerId is required"));
    }

    const QByteArray proofWire = QByteArray::fromHex(proofBytes.toString().trimmed().toLatin1());
    const QByteArray n8Wire = QByteArray::fromHex(canonicalRequestBytes.toString().trimmed().toLatin1());
    if (proofWire.isEmpty() || n8Wire.isEmpty()) {
        return makePlainError(QStringLiteral("proofBytes and canonicalRequestBytes must be non-empty even-length hex"));
    }

    LogosExecutionZone& wallet = modules().logos_execution_zone;

    QJsonObject manifest;
    QString fixtureErr;
    if (!loadFixtureManifest(&manifest, &fixtureErr)) {
        return makePlainError(fixtureErr);
    }

    uint8_t demoProvider[32]{};
    if (!demoProviderIdBytes(wallet, manifest, demoProvider, &fixtureErr)) {
        return makePlainError(fixtureErr);
    }
    const QString providerIdHex = bytes32ToHexLower(demoProvider);

    quint64 clockRaw = 0;
    if (!readClock10Timestamp(wallet, &clockRaw, &fixtureErr)) {
        return makePlainError(fixtureErr);
    }
    const quint64 policyNow = foldClockForPolicy(clockRaw);

    uint32_t arm = 0;
    QByteArray inner;
    QString encErr;
    if (!ffiBufferTwoPhase(
            [&](uint8_t* ptr, size_t cap, size_t* len) {
                return ps_ffi_parse_eligibility_proof_bytes(
                    reinterpret_cast<const uint8_t*>(proofWire.constData()),
                    static_cast<size_t>(proofWire.size()),
                    &arm,
                    ptr,
                    cap,
                    len);
            },
            &inner,
            &encErr)) {
        return makeVerifyEligibilityError(QStringLiteral("PROOF_INVALID"), encErr);
    }

    const quint64 vaultId = static_cast<quint64>(manifest.value(QStringLiteral("vault_id")).toInteger(0));
    const QString ownerBase58 = manifest.value(QStringLiteral("owner_account_id")).toString().trimmed();
    uint8_t programId[32]{};
    uint8_t owner[32]{};
    if (!programIdBytes(programId, &fixtureErr) || !ownerBytesFromBase58(wallet, ownerBase58, owner, &fixtureErr)) {
        return makePlainError(fixtureErr);
    }

    if (arm == 0u) {
        PsFfiDecodedStreamProposal proposal{};
        if (ps_ffi_parse_stream_proposal_bytes(reinterpret_cast<const uint8_t*>(inner.constData()),
                                               static_cast<size_t>(inner.size()),
                                               &proposal) != kFfiSuccess) {
            return makeVerifyEligibilityError(QStringLiteral("PROOF_INVALID"),
                                              QStringLiteral("stream proposal decode failed"));
        }
        if (!serviceIdMatchesParams(proposal.params)) {
            return makeVerifyEligibilityError(QStringLiteral("PARAMS_REJECTED"),
                                              QStringLiteral("service_id mismatch"));
        }
        if (std::memcmp(proposal.vault_proof.provider_id, demoProvider, 32) != 0) {
            return makeVerifyEligibilityError(QStringLiteral("PROOF_INVALID"),
                                              QStringLiteral("provider_id mismatch"));
        }
        if (proposal.vault_proof.vault_id != vaultId) {
            return makeVerifyEligibilityError(QStringLiteral("PARAMS_REJECTED"),
                                              QStringLiteral("vault_id mismatch"));
        }

        uint8_t vaultCfgAccount[32]{};
        uint8_t vaultHoldingAccount[32]{};
        if (ps_ffi_derive_vault_account_ids(programId, owner, vaultId, vaultCfgAccount, vaultHoldingAccount) !=
            kFfiSuccess) {
            return makePlainError(QStringLiteral("derive vault accounts failed"));
        }
        const QByteArray vaultCfgData =
            accountDataBytesFromHex(wallet, bytes32ToHexLower(vaultCfgAccount), &fixtureErr);
        if (vaultCfgData.isEmpty()) {
            return makePlainError(fixtureErr);
        }
        PsFfiDecodedVaultConfig vaultCfg{};
        if (ps_ffi_decode_vault_config(reinterpret_cast<const uint8_t*>(vaultCfgData.constData()),
                                       static_cast<size_t>(vaultCfgData.size()),
                                       &vaultCfg) != 0u) {
            return makeVerifyEligibilityError(QStringLiteral("PROOF_INVALID"),
                                              QStringLiteral("vault config decode failed"));
        }

        if (ps_ffi_verify_stream_proposal_vault_proof_bytes(reinterpret_cast<const uint8_t*>(inner.constData()),
                                                            static_cast<size_t>(inner.size()),
                                                            vaultCfg.owner) != kFfiSuccess) {
            return makeVerifyEligibilityError(QStringLiteral("PROOF_INVALID"),
                                              QStringLiteral("vault proof verification failed"));
        }

        const QString holdingJson =
            QString::fromStdString(wallet.get_account_public(bytes32ToHexLower(vaultHoldingAccount).toStdString()));
        QString balanceHex;
        if (!parseWalletAccountJson(holdingJson, nullptr, &balanceHex)) {
            return makePlainError(QStringLiteral("vault holding read failed"));
        }

        PsFfiProposalCheckInputs checkInputs{};
        std::memcpy(&checkInputs.params, &proposal.params, sizeof(proposal.params));
        fillDemoProviderPolicy(&checkInputs.policy);
        checkInputs.vault_holding_balance_lo = u128LoFromHexBalance(balanceHex);
        checkInputs.vault_holding_balance_hi = 0;
        checkInputs.vault_total_allocated_lo = vaultCfg.total_allocated_lo;
        checkInputs.vault_total_allocated_hi = vaultCfg.total_allocated_hi;
        checkInputs.now = policyNow;

        uint32_t rejectReason = 9u;
        if (ps_ffi_proposal_satisfies_policy(&checkInputs, &rejectReason) != kFfiSuccess) {
            return makeVerifyEligibilityError(verdictForPolicyReject(rejectReason),
                                              QStringLiteral("proposal policy check failed"));
        }

        QJsonObject row;
        row.insert(QStringLiteral("vault_id"), static_cast<qint64>(vaultId));
        row.insert(QStringLiteral("provider_id_hex"), providerIdHex);
        row.insert(QStringLiteral("session_public_key_hex"),
                   bytes32ToHexLower(proposal.session_public_key));
        row.insert(QStringLiteral("create_stream_deadline"),
                   static_cast<qint64>(proposal.params.create_stream_deadline));
        row.insert(QStringLiteral("rate"), QString::number(proposal.params.rate));
        row.insert(QStringLiteral("allocation_lo"), QString::number(proposal.params.allocation_lo));
        QJsonObject policyObj;
        policyObj.insert(QStringLiteral("min_rate"), QStringLiteral("1"));
        policyObj.insert(QStringLiteral("min_allocation"), QStringLiteral("1"));
        policyObj.insert(QStringLiteral("max_create_stream_deadline_delay"), QStringLiteral("3600"));
        row.insert(QStringLiteral("policy_at_acceptance"), policyObj);

        QJsonArray arr = providerAcceptancesArray();
        const int idx = findProviderAcceptanceIndex(vaultId, providerIdHex);
        const qint64 existingStream =
            idx >= 0 ? arr.at(idx).toObject().value(QStringLiteral("stream_id")).toInteger(-1) : -1;
        if (existingStream >= 0) {
            row.insert(QStringLiteral("stream_id"), existingStream);
        }
        if (idx >= 0) {
            arr.replace(idx, row);
        } else {
            arr.append(row);
        }
        setProviderAcceptancesArray(arr);
        persistIfDirty();
        return makeVerifyOk();
    }

    if (arm != 1u) {
        return makeVerifyEligibilityError(QStringLiteral("PROOF_INVALID"), QStringLiteral("unknown eligibility arm"));
    }

    PsFfiDecodedStreamProof proof{};
    if (ps_ffi_parse_stream_proof_bytes(reinterpret_cast<const uint8_t*>(inner.constData()),
                                        static_cast<size_t>(inner.size()),
                                        &proof) != kFfiSuccess) {
        return makeVerifyEligibilityError(QStringLiteral("PROOF_INVALID"), QStringLiteral("stream proof decode failed"));
    }

    uint8_t sessionPublic[32]{};
    if (!sessionPublicKeyFromNegotiations(vaultId, providerIdHex, sessionPublic)) {
        const int accIdx = findProviderAcceptanceIndex(vaultId, providerIdHex);
        if (accIdx < 0) {
            return makeVerifyEligibilityError(QStringLiteral("PROOF_INVALID"),
                                              QStringLiteral("session public key unknown"));
        }
        const QString hex =
            providerAcceptancesArray().at(accIdx).toObject().value(QStringLiteral("session_public_key_hex")).toString();
        if (!hex32FromQString(hex, sessionPublic)) {
            return makeVerifyEligibilityError(QStringLiteral("PROOF_INVALID"),
                                              QStringLiteral("session public key invalid"));
        }
    }

    if (ps_ffi_verify_stream_proof_for_n8_wire(reinterpret_cast<const uint8_t*>(inner.constData()),
                                             static_cast<size_t>(inner.size()),
                                             sessionPublic,
                                             reinterpret_cast<const uint8_t*>(n8Wire.constData()),
                                             static_cast<size_t>(n8Wire.size())) != kFfiSuccess) {
        return makeVerifyEligibilityError(QStringLiteral("PROOF_INVALID"),
                                          QStringLiteral("stream proof signature failed"));
    }

    ChainStreamView view;
    if (!readStreamAtId(wallet, programId, owner, vaultId, proof.stream_id, &view, &fixtureErr)) {
        return makePlainError(fixtureErr);
    }
    if (!view.found || !providerMatchesStream(view.decoded, demoProvider)) {
        return makeVerifyEligibilityError(QStringLiteral("STREAM_NOT_ACTIVE"),
                                          QStringLiteral("stream not found or provider mismatch"));
    }
    if (view.decoded.stream_state != kStreamStateActive) {
        return makeVerifyEligibilityError(QStringLiteral("STREAM_NOT_ACTIVE"),
                                          QStringLiteral("stream not active"));
    }
    if (view.fold.unaccrued_lo == 0 && view.fold.unaccrued_hi == 0 && !allowDepletedStreamProof()) {
        return makeVerifyEligibilityError(QStringLiteral("STREAM_NOT_ACTIVE"),
                                          QStringLiteral("stream depleted"));
    }

    PsFfiStreamParams acceptedParams{};
    paramsFromStreamConfig(view.decoded, &acceptedParams);

    PsFfiAcceptedStreamTerms terms{};
    fillAcceptedTermsFromParams(acceptedParams, demoProvider, &terms);

    const int accIdx = findProviderAcceptanceIndex(vaultId, providerIdHex);
    const bool streamIdBound =
        accIdx >= 0 && providerAcceptancesArray().at(accIdx).toObject().contains(QStringLiteral("stream_id"));

    uint32_t rejectReason = 9u;

    if (!streamIdBound) {
        if (ps_ffi_new_stream_satisfies_proposal(&view.decoded, &acceptedParams, demoProvider, &rejectReason) !=
            kFfiSuccess) {
            return makeVerifyEligibilityError(
                verdictForPolicyReject(rejectReason),
                QStringLiteral("new stream policy check failed (reject_reason=%1)").arg(rejectReason));
        }
    }

    if (ps_ffi_stream_satisfies_policy(&view.fold.folded_stream, &terms, &rejectReason) != kFfiSuccess) {
        return makeVerifyEligibilityError(
            verdictForPolicyReject(rejectReason),
            QStringLiteral("stream policy check failed (reject_reason=%1)").arg(rejectReason));
    }

    QJsonObject row;
    row.insert(QStringLiteral("vault_id"), static_cast<qint64>(vaultId));
    row.insert(QStringLiteral("provider_id_hex"), providerIdHex);
    row.insert(QStringLiteral("stream_id"), static_cast<qint64>(proof.stream_id));
    row.insert(QStringLiteral("session_public_key_hex"), bytes32ToHexLower(sessionPublic));
    QJsonObject policyObj;
    policyObj.insert(QStringLiteral("min_rate"), QStringLiteral("1"));
    policyObj.insert(QStringLiteral("min_allocation"), QStringLiteral("1"));
    policyObj.insert(QStringLiteral("max_create_stream_deadline_delay"), QStringLiteral("3600"));
    row.insert(QStringLiteral("policy_at_acceptance"), policyObj);
    QJsonArray arr = providerAcceptancesArray();
    const int idx = findProviderAcceptanceIndex(vaultId, providerIdHex);
    if (idx >= 0) {
        arr.replace(idx, row);
    } else {
        arr.append(row);
    }
    setProviderAcceptancesArray(arr);
    persistIfDirty();
    return makeVerifyOk();
}
