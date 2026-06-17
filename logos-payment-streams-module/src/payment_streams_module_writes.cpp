#include "payment_streams_module_impl.h"

#include <QDir>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QJsonParseError>
#include <QMetaType>
#include <QVariant>

#include <functional>

#include <logos_api.h>
#include <logos_api_client.h>
#include <logos_sdk.h>

#include "payment_streams_ffi_bridge.h"

#include <cstring>

namespace {

constexpr int kAccountIdHexLen = 64;
constexpr uint8_t kPrivacyTierPublic = 0;
constexpr uint32_t kFfiSuccess = 0u;

QString makeErrorJson(const QString& message) {
    QJsonObject obj;
    obj.insert(QStringLiteral("status"), QStringLiteral("error"));
    obj.insert(QStringLiteral("message"), message);
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

LogosAPIClient* walletClientOrNull(LogosAPI* api) {
    if (api == nullptr) {
        return nullptr;
    }
    return api->getClient(QStringLiteral("logos_execution_zone"));
}

QString invokeWalletString(LogosAPIClient* client, const char* method, const QVariant& arg = {}) {
    if (client == nullptr) {
        return {};
    }
    const QString moduleName = QStringLiteral("logos_execution_zone");
    const QString methodName = QString::fromUtf8(method);
    QVariant result;
    if (arg.isValid() && !arg.isNull()) {
        result = client->invokeRemoteMethod(moduleName, methodName, arg);
    } else {
        result = client->invokeRemoteMethod(moduleName, methodName);
    }
    if (!result.isValid()) {
        return {};
    }
    return result.toString();
}

QString invokeWalletMulti(LogosAPIClient* client,
                          const char* method,
                          const QVariant& a1,
                          const QVariant& a2 = {},
                          const QVariant& a3 = {},
                          const QVariant& a4 = {},
                          const QVariant& a5 = {}) {
    if (client == nullptr) {
        return {};
    }
    const QString moduleName = QStringLiteral("logos_execution_zone");
    const QString methodName = QString::fromUtf8(method);
    QVariant result = client->invokeRemoteMethod(moduleName, methodName, a1, a2, a3, a4, a5);
    if (!result.isValid()) {
        return {};
    }
    return result.toString();
}

QString walletAccountIdHexFromBase58(LogosAPIClient* client, const QString& accountIdBase58) {
    const QString trimmed = accountIdBase58.trimmed();
    if (trimmed.isEmpty()) {
        return {};
    }
    return invokeWalletString(client, "account_id_from_base58", trimmed);
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

struct FixtureConfig {
    QString programIdHex;
    QString clock10Base58;
    bool loaded = false;
};

FixtureConfig& fixtureConfig() {
    static FixtureConfig cfg;
    return cfg;
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

bool ensureFixtureLoaded(QString* errorOut) {
    FixtureConfig& cfg = fixtureConfig();
    if (cfg.loaded) {
        return true;
    }
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
            *errorOut = QStringLiteral("fixture manifest JSON parse failed: %1").arg(parseError.errorString());
        }
        return false;
    }
    const QJsonObject obj = doc.object();
    cfg.programIdHex = obj.value(QStringLiteral("program_id_hex")).toString().trimmed();
    cfg.clock10Base58 = obj.value(QStringLiteral("clock_10_account_id"))
                            .toString()
                            .trimmed();
    if (cfg.clock10Base58.isEmpty()) {
        cfg.clock10Base58 = QStringLiteral("4BdcjoXkq786TMWcBGGHqcxeLYMZmn17rL4eM9ZyRWSs");
    }
    if (cfg.programIdHex.size() != 64) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("fixture program_id_hex must be 64 hex chars");
        }
        return false;
    }
    cfg.loaded = true;
    return true;
}

bool programIdBytes(uint8_t out[32], QString* errorOut) {
    if (!ensureFixtureLoaded(errorOut)) {
        return false;
    }
    return hex32FromQString(fixtureConfig().programIdHex, out);
}

bool ownerBytesFromBase58(LogosAPIClient* client, const QString& base58, uint8_t out[32], QString* errorOut) {
    const QString hex = walletAccountIdHexFromBase58(client, base58);
    if (hex.size() != 64) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("account_id_from_base58 failed");
        }
        return false;
    }
    return hex32FromQString(hex, out);
}

bool clockBytes(uint8_t out[32], QString* errorOut) {
    const auto status = ps_ffi_fixed_clock_10_account_id(out);
    if (status != kFfiSuccess) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("fixed clock account id FFI failed");
        }
        return false;
    }
    return true;
}

QString guestElfPath() {
    const QByteArray env = qgetenv("PAYMENT_STREAMS_GUEST_BIN");
    if (!env.isEmpty()) {
        return resolveRepoRelativePath(QString::fromUtf8(env));
    }
    QString found;
    const QString relative =
        QStringLiteral("methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin");
    if (findRepoFile(relative, &found)) {
        return found;
    }
    return resolveRepoRelativePath(relative);
}

bool loadGuestElfBytes(QByteArray* out, QString* errorOut) {
    const QString path = guestElfPath();
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("cannot read guest ELF: %1").arg(path);
        }
        return false;
    }
    *out = file.readAll();
    if (out->isEmpty()) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("guest ELF is empty");
        }
        return false;
    }
    return true;
}

QStringList bytesToDecimalStringList(const QByteArray& bytes) {
    QStringList list;
    list.reserve(bytes.size());
    for (unsigned char byte : bytes) {
        list.append(QString::number(static_cast<uint>(byte)));
    }
    return list;
}

QStringList decimalWordsFromInstructionBytes(const QByteArray& instructionBytes) {
    QStringList words;
    if (instructionBytes.size() % 4 != 0) {
        return words;
    }
    for (int i = 0; i + 3 < instructionBytes.size(); i += 4) {
        const uint32_t word = static_cast<uint32_t>(static_cast<unsigned char>(instructionBytes[i])) |
                              (static_cast<uint32_t>(static_cast<unsigned char>(instructionBytes[i + 1])) << 8) |
                              (static_cast<uint32_t>(static_cast<unsigned char>(instructionBytes[i + 2])) << 16) |
                              (static_cast<uint32_t>(static_cast<unsigned char>(instructionBytes[i + 3])) << 24);
        words.append(QString::number(word));
    }
    return words;
}

bool ffiSerializeInitializeVault(uint64_t vaultId, QByteArray* out, QString* errorOut) {
    size_t required = 0;
    if (ps_ffi_serialize_initialize_vault(vaultId, kPrivacyTierPublic, nullptr, 0, &required) != kFfiSuccess) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("initialize_vault serialize sizing failed");
        }
        return false;
    }
    out->resize(static_cast<int>(required));
    if (ps_ffi_serialize_initialize_vault(vaultId,
                                          kPrivacyTierPublic,
                                          reinterpret_cast<uint8_t*>(out->data()),
                                          required,
                                          &required) != kFfiSuccess) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("initialize_vault serialize failed");
        }
        return false;
    }
    out->resize(static_cast<int>(required));
    return true;
}

bool ffiPlanInitializeVault(const uint8_t programId[32],
                            const uint8_t owner[32],
                            uint64_t vaultId,
                            QByteArray* hexOut,
                            QString* errorOut) {
    size_t required = 0;
    if (ps_ffi_plan_initialize_vault(programId, owner, vaultId, nullptr, 0, &required) != kFfiSuccess) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("initialize_vault plan sizing failed");
        }
        return false;
    }
    hexOut->resize(static_cast<int>(required));
    if (ps_ffi_plan_initialize_vault(programId,
                                    owner,
                                    vaultId,
                                    reinterpret_cast<uint8_t*>(hexOut->data()),
                                    required,
                                    &required) != kFfiSuccess) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("initialize_vault plan failed");
        }
        return false;
    }
    hexOut->resize(static_cast<int>(required));
    return true;
}

bool ffiSerializeTwoPhase(const std::function<uint32_t(
                              uint8_t*, uintptr_t, uintptr_t*)>& call,
                          QByteArray* out,
                          QString* errorOut) {
    uintptr_t required = 0;
    const auto sizing = call(nullptr, 0, &required);
    if (sizing != kFfiSuccess) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("instruction serialize sizing failed (%1)").arg(static_cast<uint>(sizing));
        }
        return false;
    }
    out->resize(static_cast<int>(required));
    const auto write = call(reinterpret_cast<uint8_t*>(out->data()), required, &required);
    if (write != kFfiSuccess) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("instruction serialize failed (%1)").arg(static_cast<uint>(write));
        }
        return false;
    }
    out->resize(static_cast<int>(required));
    return true;
}

bool ffiPlanAccountsTwoPhase(const std::function<uint32_t(
                                 uint8_t*, uintptr_t, uintptr_t*)>& call,
                             QByteArray* hexOut,
                             QString* errorOut) {
    uintptr_t required = 0;
    const auto sizing = call(nullptr, 0, &required);
    if (sizing != kFfiSuccess) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("plan accounts sizing failed (%1)").arg(static_cast<uint>(sizing));
        }
        return false;
    }
    hexOut->resize(static_cast<int>(required));
    const auto write = call(reinterpret_cast<uint8_t*>(hexOut->data()), required, &required);
    if (write != kFfiSuccess) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("plan accounts failed (%1)").arg(static_cast<uint>(write));
        }
        return false;
    }
    hexOut->resize(static_cast<int>(required));
    return true;
}

QStringList splitAccountsHex(const QByteArray& accountsHex) {
    QStringList ids;
    const QString all = QString::fromLatin1(accountsHex);
    for (int i = 0; i + kAccountIdHexLen <= all.size(); i += kAccountIdHexLen) {
        ids.append(all.mid(i, kAccountIdHexLen));
    }
    return ids;
}

QList<bool> signingRequirementsForAccounts(const QStringList& accountHexIds, const QString& signerHex) {
    const QString signer = signerHex.trimmed().toLower();
    QList<bool> flags;
    flags.reserve(accountHexIds.size());
    for (const QString& id : accountHexIds) {
        flags.append(id.trimmed().toLower() == signer);
    }
    return flags;
}

QList<uint8_t> bytesToUint8List(const QByteArray& bytes) {
    QList<uint8_t> list;
    list.reserve(bytes.size());
    for (unsigned char byte : bytes) {
        list.append(static_cast<uint8_t>(byte));
    }
    return list;
}

QList<uint8_t> instructionBytesForWallet(LogosAPIClient* client, const QByteArray& borshBytes, QString* errorOut) {
    Q_UNUSED(client);
    const QList<uint8_t> borshList = bytesToUint8List(borshBytes);
    if (borshList.isEmpty()) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("instruction bytes empty");
        }
        return {};
    }
    return borshList;
}

bool guestElfLoadedInWalletProcess() {
    return !qEnvironmentVariableIsEmpty("PAYMENT_STREAMS_GUEST_BIN");
}

QList<uint8_t> walletAuthenticatedTransferElfBytes(LogosAPIClient* client, QString* errorOut) {
    if (client == nullptr) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("wallet client missing");
        }
        return {};
    }
    const QVariant raw =
        client->invokeRemoteMethod(QStringLiteral("logos_execution_zone"), QStringLiteral("authenticated_transfer_elf"));
    if (raw.canConvert<QList<uint8_t>>()) {
        return raw.value<QList<uint8_t>>();
    }
    if (raw.canConvert<QByteArray>()) {
        return bytesToUint8List(raw.toByteArray());
    }
    if (raw.canConvert<QStringList>()) {
        QList<uint8_t> out;
        for (const QString& part : raw.toStringList()) {
            out.append(static_cast<uint8_t>(part.toUInt()));
        }
        return out;
    }
    if (errorOut != nullptr) {
        *errorOut = QStringLiteral("authenticated_transfer_elf unexpected response type");
    }
    return {};
}

QString parseWalletSubmitJson(const QString& walletJson, QJsonObject* fieldsOut, QString* errorOut) {
    QJsonParseError parseError{};
    const QJsonDocument doc = QJsonDocument::fromJson(walletJson.toUtf8(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !doc.isObject()) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("wallet submit JSON parse failed");
        }
        return makeErrorJson(errorOut != nullptr ? *errorOut : QStringLiteral("parse failed"));
    }
    const QJsonObject obj = doc.object();
    if (fieldsOut != nullptr) {
        *fieldsOut = obj;
    }
    QJsonObject payload;
    payload.insert(QStringLiteral("wallet"), obj);
    if (obj.value(QStringLiteral("success")).toBool()) {
        payload.insert(QStringLiteral("success"), true);
        payload.insert(QStringLiteral("tx_hash"), obj.value(QStringLiteral("tx_hash")).toString());
        return makeOkJson(payload);
    }
    const QString err = obj.value(QStringLiteral("error")).toString();
    if (errorOut != nullptr) {
        *errorOut = err.isEmpty() ? QStringLiteral("wallet submit failed") : err;
    }
    return makeErrorJson(err.isEmpty() ? QStringLiteral("wallet submit failed") : err);
}

QString submitGenericPublic(LogosAPIClient* client,
                            const QStringList& accountHexIds,
                            const QList<bool>& signingFlags,
                            const QList<uint8_t>& instructionBytes,
                            const QList<uint8_t>& programElfBytes,
                            const QList<QList<uint8_t>>& programDependencies,
                            QString* errorOut) {
    QJsonObject payload;
    QJsonArray accountIdsJson;
    for (const QString& id : accountHexIds) {
        accountIdsJson.append(id);
    }
    payload.insert(QStringLiteral("account_ids"), accountIdsJson);
    QJsonArray signingJson;
    for (bool flag : signingFlags) {
        signingJson.append(flag);
    }
    payload.insert(QStringLiteral("signing_requirements"), signingJson);

    QByteArray instructionRaw;
    instructionRaw.reserve(instructionBytes.size());
    for (uint8_t byte : instructionBytes) {
        instructionRaw.append(static_cast<char>(byte));
    }
    payload.insert(QStringLiteral("instruction_hex"), QString::fromLatin1(instructionRaw.toHex()));

    QByteArray programRaw;
    programRaw.reserve(programElfBytes.size());
    for (uint8_t byte : programElfBytes) {
        programRaw.append(static_cast<char>(byte));
    }
    payload.insert(QStringLiteral("program_elf_hex"), QString::fromLatin1(programRaw.toHex()));

    QJsonArray depsJson;
    for (const QList<uint8_t>& depList : programDependencies) {
        QByteArray depRaw;
        depRaw.reserve(depList.size());
        for (uint8_t byte : depList) {
            depRaw.append(static_cast<char>(byte));
        }
        depsJson.append(QString::fromLatin1(depRaw.toHex()));
    }
    payload.insert(QStringLiteral("program_dependencies_hex"), depsJson);

    const QString payloadJson = QJsonDocument(payload).toJson(QJsonDocument::Compact);
    QString walletJson =
        invokeWalletString(client, "send_generic_public_transaction_json", payloadJson);
    if (walletJson.isEmpty()) {
        walletJson = invokeWalletMulti(client,
                                       "send_generic_public_transaction",
                                       QVariant::fromValue(accountHexIds),
                                       QVariant::fromValue(signingFlags),
                                       QVariant::fromValue(instructionBytes),
                                       QVariant::fromValue(programElfBytes),
                                       QVariant::fromValue(programDependencies));
    }
    if (walletJson.isEmpty()) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("send_generic_public_transaction returned empty");
        }
        return makeErrorJson(QStringLiteral("send_generic_public_transaction returned empty"));
    }
    QJsonObject fields;
    return parseWalletSubmitJson(walletJson, &fields, errorOut);
}

QString buildAndSubmit(LogosAPIClient* client,
                       const QString& signerBase58,
                       const QByteArray& instructionBytes,
                       const QByteArray& accountsHex,
                       bool includeTransferDep,
                       QString* errorOut) {
    QString loadErr;
    if (!ensureFixtureLoaded(&loadErr)) {
        return makeErrorJson(loadErr);
    }

    const QString signerHex = walletAccountIdHexFromBase58(client, signerBase58.trimmed());
    if (signerHex.size() != 64) {
        return makeErrorJson(QStringLiteral("invalid signer account"));
    }

    const QStringList accountIds = splitAccountsHex(accountsHex);
    if (accountIds.isEmpty()) {
        return makeErrorJson(QStringLiteral("planned account list is empty"));
    }

    const QList<bool> signing = signingRequirementsForAccounts(accountIds, signerHex);

    const QList<uint8_t> instructionList = instructionBytesForWallet(client, instructionBytes, &loadErr);
    if (instructionList.isEmpty()) {
        return makeErrorJson(loadErr.isEmpty() ? QStringLiteral("instruction encoding failed") : loadErr);
    }

    QList<uint8_t> programElfList;
    if (guestElfLoadedInWalletProcess()) {
        programElfList = {};
    } else {
        QByteArray guestElf;
        if (!loadGuestElfBytes(&guestElf, &loadErr)) {
            return makeErrorJson(loadErr);
        }
        programElfList = bytesToUint8List(guestElf);
    }

    QList<QList<uint8_t>> deps;
    if (includeTransferDep) {
        if (guestElfLoadedInWalletProcess()) {
            deps = {};
        } else {
            const QList<uint8_t> transferElf = walletAuthenticatedTransferElfBytes(client, &loadErr);
            if (transferElf.isEmpty()) {
                return makeErrorJson(loadErr.isEmpty() ? QStringLiteral("authenticated_transfer_elf failed")
                                                        : loadErr);
            }
            deps.append(transferElf);
        }
    }

    return submitGenericPublic(client,
                               accountIds,
                               signing,
                               instructionList,
                               programElfList,
                               deps,
                               errorOut);
}

quint64 variantToU64(const QVariant& value, bool* okOut) {
    bool ok = false;
    const qulonglong v = value.toULongLong(&ok);
    if (okOut != nullptr) {
        *okOut = ok;
    }
    return static_cast<quint64>(v);
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

QByteArray accountDataBytesFromHex(LogosAPIClient* client, const QString& accountHex, QString* errorOut) {
    const QString accountJson = invokeWalletString(client, "get_account_public", accountHex);
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

}  // namespace

QString PaymentStreamsModuleImpl::initializeVault(const QVariant& signerAccountIdBase58, const QVariant& vaultId) {
    LogosAPIClient* client = walletClientOrNull(modules().api);
    if (client == nullptr) {
        return makeErrorJson(QStringLiteral("logos_execution_zone client unavailable (load wallet first)"));
    }
    bool ok = false;
    const quint64 vid = variantToU64(vaultId, &ok);
    if (!ok) {
        return makeErrorJson(QStringLiteral("vaultId must be unsigned integer"));
    }

    uint8_t programId[32]{};
    uint8_t owner[32]{};
    QString err;
    if (!programIdBytes(programId, &err)) {
        return makeErrorJson(err);
    }
    if (!ownerBytesFromBase58(client, signerAccountIdBase58.toString(), owner, &err)) {
        return makeErrorJson(err);
    }

    QByteArray instruction;
    if (!ffiSerializeInitializeVault(vid, &instruction, &err)) {
        return makeErrorJson(err);
    }

    QByteArray accountsHex;
    if (!ffiPlanInitializeVault(programId, owner, vid, &accountsHex, &err)) {
        return makeErrorJson(err);
    }

    return buildAndSubmit(client, signerAccountIdBase58.toString(), instruction, accountsHex, false, &err);
}

QString PaymentStreamsModuleImpl::deposit(const QVariant& signerAccountIdBase58,
                                          const QVariant& vaultId,
                                          const QVariant& amountLo,
                                          const QVariant& amountHi) {
    LogosAPIClient* client = walletClientOrNull(modules().api);
    if (client == nullptr) {
        return makeErrorJson(QStringLiteral("logos_execution_zone client unavailable (load wallet first)"));
    }
    bool ok = false;
    const quint64 vid = variantToU64(vaultId, &ok);
    const quint64 lo = variantToU64(amountLo, &ok);
    const quint64 hi = amountHi.isValid() && !amountHi.isNull() ? variantToU64(amountHi, &ok) : 0;
    if (!ok) {
        return makeErrorJson(QStringLiteral("invalid numeric argument"));
    }

    uint8_t programId[32]{};
    uint8_t owner[32]{};
    uint8_t transferPid[32]{};
    QString err;
    if (!programIdBytes(programId, &err)) {
        return makeErrorJson(err);
    }
    if (!ownerBytesFromBase58(client, signerAccountIdBase58.toString(), owner, &err)) {
        return makeErrorJson(err);
    }
    if (ps_ffi_authenticated_transfer_program_id(transferPid) !=
        kFfiSuccess) {
        return makeErrorJson(QStringLiteral("authenticated transfer program id FFI failed"));
    }

    QByteArray instruction;
    if (!ffiSerializeTwoPhase(
            [&](uint8_t* ptr, uintptr_t cap, uintptr_t* len) {
                return ps_ffi_serialize_deposit(
                    vid, lo, hi, transferPid, ptr, cap, len);
            },
            &instruction, &err)) {
        return makeErrorJson(err);
    }

    QByteArray accountsHex;
    if (!ffiPlanAccountsTwoPhase(
            [&](uint8_t* ptr, uintptr_t cap, uintptr_t* len) {
                return ps_ffi_plan_deposit(
                    programId, owner, vid, ptr, cap, len);
            },
            &accountsHex, &err)) {
        return makeErrorJson(err);
    }

    return buildAndSubmit(client, signerAccountIdBase58.toString(), instruction, accountsHex, true, &err);
}

QString PaymentStreamsModuleImpl::withdraw(const QVariant& signerAccountIdBase58,
                                           const QVariant& vaultId,
                                           const QVariant& amountLo,
                                           const QVariant& amountHi,
                                           const QVariant& withdrawToAccountIdBase58) {
    LogosAPIClient* client = walletClientOrNull(modules().api);
    if (client == nullptr) {
        return makeErrorJson(QStringLiteral("logos_execution_zone client unavailable (load wallet first)"));
    }
    bool ok = false;
    const quint64 vid = variantToU64(vaultId, &ok);
    const quint64 lo = variantToU64(amountLo, &ok);
    const quint64 hi = variantToU64(amountHi, &ok);
    if (!ok) {
        return makeErrorJson(QStringLiteral("invalid numeric argument"));
    }

    const QString withdrawBase58 = withdrawToAccountIdBase58.isValid() && !withdrawToAccountIdBase58.isNull()
                                       ? withdrawToAccountIdBase58.toString()
                                       : signerAccountIdBase58.toString();

    uint8_t programId[32]{};
    uint8_t owner[32]{};
    uint8_t withdrawTo[32]{};
    QString err;
    if (!programIdBytes(programId, &err)) {
        return makeErrorJson(err);
    }
    if (!ownerBytesFromBase58(client, signerAccountIdBase58.toString(), owner, &err)) {
        return makeErrorJson(err);
    }
    if (!ownerBytesFromBase58(client, withdrawBase58, withdrawTo, &err)) {
        return makeErrorJson(err);
    }

    QByteArray instruction;
    if (!ffiSerializeTwoPhase(
            [&](uint8_t* ptr, uintptr_t cap, uintptr_t* len) {
                return ps_ffi_serialize_withdraw(vid, lo, hi, ptr, cap, len);
            },
            &instruction, &err)) {
        return makeErrorJson(err);
    }

    QByteArray accountsHex;
    if (!ffiPlanAccountsTwoPhase(
            [&](uint8_t* ptr, uintptr_t cap, uintptr_t* len) {
                return ps_ffi_plan_withdraw(
                    programId, owner, vid, withdrawTo, ptr, cap, len);
            },
            &accountsHex, &err)) {
        return makeErrorJson(err);
    }

    return buildAndSubmit(client, signerAccountIdBase58.toString(), instruction, accountsHex, false, &err);
}

QString PaymentStreamsModuleImpl::createStream(const QVariant& signerAccountIdBase58,
                                             const QVariant& vaultId,
                                             const QVariant& streamId,
                                             const QVariant& providerAccountIdBase58,
                                             const QVariant& rateTokensPerSecond,
                                             const QVariant& allocationLo,
                                             const QVariant& allocationHi) {
    LogosAPIClient* client = walletClientOrNull(modules().api);
    if (client == nullptr) {
        return makeErrorJson(QStringLiteral("logos_execution_zone client unavailable (load wallet first)"));
    }
    bool ok = false;
    const quint64 vid = variantToU64(vaultId, &ok);
    const quint64 sid = variantToU64(streamId, &ok);
    const quint64 rate = variantToU64(rateTokensPerSecond, &ok);
    const quint64 allocLo = variantToU64(allocationLo, &ok);
    const quint64 allocHi =
        allocationHi.isValid() && !allocationHi.isNull() ? variantToU64(allocationHi, &ok) : 0;
    if (!ok) {
        return makeErrorJson(QStringLiteral("invalid numeric argument"));
    }

    uint8_t programId[32]{};
    uint8_t owner[32]{};
    uint8_t provider[32]{};
    uint8_t clock[32]{};
    QString err;
    if (!programIdBytes(programId, &err)) {
        return makeErrorJson(err);
    }
    if (!ownerBytesFromBase58(client, signerAccountIdBase58.toString(), owner, &err)) {
        return makeErrorJson(err);
    }
    if (!ownerBytesFromBase58(client, providerAccountIdBase58.toString(), provider, &err)) {
        return makeErrorJson(err);
    }
    if (!clockBytes(clock, &err)) {
        return makeErrorJson(err);
    }

    QByteArray instruction;
    if (!ffiSerializeTwoPhase(
            [&](uint8_t* ptr, uintptr_t cap, uintptr_t* len) {
                return ps_ffi_serialize_create_stream(
                    vid, sid, provider, rate, allocLo, allocHi, ptr, cap, len);
            },
            &instruction, &err)) {
        return makeErrorJson(err);
    }

    QByteArray accountsHex;
    if (!ffiPlanAccountsTwoPhase(
            [&](uint8_t* ptr, uintptr_t cap, uintptr_t* len) {
                return ps_ffi_plan_create_stream(
                    programId, owner, vid, sid, clock, ptr, cap, len);
            },
            &accountsHex, &err)) {
        return makeErrorJson(err);
    }

    return buildAndSubmit(client, signerAccountIdBase58.toString(), instruction, accountsHex, false, &err);
}

QString PaymentStreamsModuleImpl::pauseStream(const QVariant& signerAccountIdBase58,
                                              const QVariant& vaultId,
                                              const QVariant& streamId) {
    LogosAPIClient* client = walletClientOrNull(modules().api);
    if (client == nullptr) {
        return makeErrorJson(QStringLiteral("logos_execution_zone client unavailable (load wallet first)"));
    }
    bool ok = false;
    const quint64 vid = variantToU64(vaultId, &ok);
    const quint64 sid = variantToU64(streamId, &ok);
    if (!ok) {
        return makeErrorJson(QStringLiteral("invalid numeric argument"));
    }

    uint8_t programId[32]{};
    uint8_t owner[32]{};
    uint8_t clock[32]{};
    QString err;
    if (!programIdBytes(programId, &err) || !ownerBytesFromBase58(client, signerAccountIdBase58.toString(), owner, &err) ||
        !clockBytes(clock, &err)) {
        return makeErrorJson(err);
    }

    QByteArray instruction;
    if (!ffiSerializeTwoPhase(
            [&](uint8_t* ptr, uintptr_t cap, uintptr_t* len) {
                return ps_ffi_serialize_pause_stream(vid, sid, ptr, cap, len);
            },
            &instruction, &err)) {
        return makeErrorJson(err);
    }

    QByteArray accountsHex;
    if (!ffiPlanAccountsTwoPhase(
            [&](uint8_t* ptr, uintptr_t cap, uintptr_t* len) {
                return ps_ffi_plan_pause_stream(
                    programId, owner, vid, sid, clock, ptr, cap, len);
            },
            &accountsHex, &err)) {
        return makeErrorJson(err);
    }

    return buildAndSubmit(client, signerAccountIdBase58.toString(), instruction, accountsHex, false, &err);
}

QString PaymentStreamsModuleImpl::resumeStream(const QVariant& signerAccountIdBase58,
                                               const QVariant& vaultId,
                                               const QVariant& streamId) {
    LogosAPIClient* client = walletClientOrNull(modules().api);
    if (client == nullptr) {
        return makeErrorJson(QStringLiteral("logos_execution_zone client unavailable (load wallet first)"));
    }
    bool ok = false;
    const quint64 vid = variantToU64(vaultId, &ok);
    const quint64 sid = variantToU64(streamId, &ok);
    if (!ok) {
        return makeErrorJson(QStringLiteral("invalid numeric argument"));
    }

    uint8_t programId[32]{};
    uint8_t owner[32]{};
    uint8_t clock[32]{};
    QString err;
    if (!programIdBytes(programId, &err) || !ownerBytesFromBase58(client, signerAccountIdBase58.toString(), owner, &err) ||
        !clockBytes(clock, &err)) {
        return makeErrorJson(err);
    }

    QByteArray instruction;
    if (!ffiSerializeTwoPhase(
            [&](uint8_t* ptr, uintptr_t cap, uintptr_t* len) {
                return ps_ffi_serialize_resume_stream(vid, sid, ptr, cap, len);
            },
            &instruction, &err)) {
        return makeErrorJson(err);
    }

    QByteArray accountsHex;
    if (!ffiPlanAccountsTwoPhase(
            [&](uint8_t* ptr, uintptr_t cap, uintptr_t* len) {
                return ps_ffi_plan_resume_stream(
                    programId, owner, vid, sid, clock, ptr, cap, len);
            },
            &accountsHex, &err)) {
        return makeErrorJson(err);
    }

    return buildAndSubmit(client, signerAccountIdBase58.toString(), instruction, accountsHex, false, &err);
}

QString PaymentStreamsModuleImpl::topUpStream(const QVariant& signerAccountIdBase58,
                                              const QVariant& vaultId,
                                              const QVariant& streamId,
                                              const QVariant& increaseLo,
                                              const QVariant& increaseHi) {
    LogosAPIClient* client = walletClientOrNull(modules().api);
    if (client == nullptr) {
        return makeErrorJson(QStringLiteral("logos_execution_zone client unavailable (load wallet first)"));
    }
    bool ok = false;
    const quint64 vid = variantToU64(vaultId, &ok);
    const quint64 sid = variantToU64(streamId, &ok);
    const quint64 lo = variantToU64(increaseLo, &ok);
    const quint64 hi = increaseHi.isValid() && !increaseHi.isNull() ? variantToU64(increaseHi, &ok) : 0;
    if (!ok) {
        return makeErrorJson(QStringLiteral("invalid numeric argument"));
    }

    uint8_t programId[32]{};
    uint8_t owner[32]{};
    uint8_t clock[32]{};
    QString err;
    if (!programIdBytes(programId, &err) || !ownerBytesFromBase58(client, signerAccountIdBase58.toString(), owner, &err) ||
        !clockBytes(clock, &err)) {
        return makeErrorJson(err);
    }

    QByteArray instruction;
    if (!ffiSerializeTwoPhase(
            [&](uint8_t* ptr, uintptr_t cap, uintptr_t* len) {
                return ps_ffi_serialize_top_up_stream(vid, sid, lo, hi, ptr, cap, len);
            },
            &instruction, &err)) {
        return makeErrorJson(err);
    }

    QByteArray accountsHex;
    if (!ffiPlanAccountsTwoPhase(
            [&](uint8_t* ptr, uintptr_t cap, uintptr_t* len) {
                return ps_ffi_plan_top_up_stream(
                    programId, owner, vid, sid, clock, ptr, cap, len);
            },
            &accountsHex, &err)) {
        return makeErrorJson(err);
    }

    return buildAndSubmit(client, signerAccountIdBase58.toString(), instruction, accountsHex, false, &err);
}

QString PaymentStreamsModuleImpl::closeStream(const QVariant& signerAccountIdBase58,
                                              const QVariant& vaultId,
                                              const QVariant& streamId,
                                              const QVariant& authorityAccountIdBase58) {
    LogosAPIClient* client = walletClientOrNull(modules().api);
    if (client == nullptr) {
        return makeErrorJson(QStringLiteral("logos_execution_zone client unavailable (load wallet first)"));
    }
    bool ok = false;
    const quint64 vid = variantToU64(vaultId, &ok);
    const quint64 sid = variantToU64(streamId, &ok);
    if (!ok) {
        return makeErrorJson(QStringLiteral("invalid numeric argument"));
    }

    const QString authorityBase58 = authorityAccountIdBase58.isValid() && !authorityAccountIdBase58.isNull()
                                        ? authorityAccountIdBase58.toString()
                                        : signerAccountIdBase58.toString();

    uint8_t programId[32]{};
    uint8_t owner[32]{};
    uint8_t authority[32]{};
    uint8_t clock[32]{};
    QString err;
    if (!programIdBytes(programId, &err) || !ownerBytesFromBase58(client, signerAccountIdBase58.toString(), owner, &err) ||
        !ownerBytesFromBase58(client, authorityBase58, authority, &err) || !clockBytes(clock, &err)) {
        return makeErrorJson(err);
    }

    QByteArray instruction;
    if (!ffiSerializeTwoPhase(
            [&](uint8_t* ptr, uintptr_t cap, uintptr_t* len) {
                return ps_ffi_serialize_close_stream(vid, sid, ptr, cap, len);
            },
            &instruction, &err)) {
        return makeErrorJson(err);
    }

    QByteArray accountsHex;
    if (!ffiPlanAccountsTwoPhase(
            [&](uint8_t* ptr, uintptr_t cap, uintptr_t* len) {
                return ps_ffi_plan_close_stream(
                    programId, owner, vid, sid, authority, clock, ptr, cap, len);
            },
            &accountsHex, &err)) {
        return makeErrorJson(err);
    }

    return buildAndSubmit(client, authorityBase58, instruction, accountsHex, false, &err);
}

QString PaymentStreamsModuleImpl::claim(const QVariant& providerAccountIdBase58,
                                        const QVariant& vaultId,
                                        const QVariant& streamId) {
    LogosAPIClient* client = walletClientOrNull(modules().api);
    if (client == nullptr) {
        return makeErrorJson(QStringLiteral("logos_execution_zone client unavailable (load wallet first)"));
    }
    bool ok = false;
    const quint64 vid = variantToU64(vaultId, &ok);
    const quint64 sid = variantToU64(streamId, &ok);
    if (!ok) {
        return makeErrorJson(QStringLiteral("invalid numeric argument"));
    }

    QString err;
    if (!ensureFixtureLoaded(&err)) {
        return makeErrorJson(err);
    }
    QFile manifestFile(fixtureManifestPath());
    if (!manifestFile.open(QIODevice::ReadOnly)) {
        return makeErrorJson(QStringLiteral("fixture manifest required for claim owner"));
    }
    const QJsonObject manifest = QJsonDocument::fromJson(manifestFile.readAll()).object();
    const QString ownerBase58 = manifest.value(QStringLiteral("owner_account_id")).toString().trimmed();
    if (ownerBase58.isEmpty()) {
        return makeErrorJson(QStringLiteral("fixture owner_account_id missing"));
    }

    uint8_t programId[32]{};
    uint8_t owner[32]{};
    uint8_t provider[32]{};
    uint8_t clock[32]{};
    if (!programIdBytes(programId, &err) || !ownerBytesFromBase58(client, ownerBase58, owner, &err) ||
        !ownerBytesFromBase58(client, providerAccountIdBase58.toString(), provider, &err) ||
        !clockBytes(clock, &err)) {
        return makeErrorJson(err);
    }

    QByteArray instruction;
    if (!ffiSerializeTwoPhase(
            [&](uint8_t* ptr, uintptr_t cap, uintptr_t* len) {
                return ps_ffi_serialize_claim(vid, sid, ptr, cap, len);
            },
            &instruction, &err)) {
        return makeErrorJson(err);
    }

    QByteArray accountsHex;
    if (!ffiPlanAccountsTwoPhase(
            [&](uint8_t* ptr, uintptr_t cap, uintptr_t* len) {
                return ps_ffi_plan_claim(
                    programId, owner, vid, sid, provider, clock, ptr, cap, len);
            },
            &accountsHex, &err)) {
        return makeErrorJson(err);
    }

    return buildAndSubmit(client, providerAccountIdBase58.toString(), instruction, accountsHex, false, &err);
}

QString PaymentStreamsModuleImpl::getVaultStatus(const QVariant& ownerAccountIdBase58,
                                                 const QVariant& vaultId,
                                                 const QVariant& streamId) {
    Q_UNUSED(streamId);
    LogosAPIClient* client = walletClientOrNull(modules().api);
    if (client == nullptr) {
        return makeErrorJson(QStringLiteral("logos_execution_zone client unavailable (load wallet first)"));
    }
    bool ok = false;
    const quint64 vid = variantToU64(vaultId, &ok);
    if (!ok) {
        return makeErrorJson(QStringLiteral("invalid vaultId"));
    }

    uint8_t programId[32]{};
    uint8_t owner[32]{};
    uint8_t vaultCfg[32]{};
    uint8_t vaultHolding[32]{};
    QString err;
    if (!programIdBytes(programId, &err) || !ownerBytesFromBase58(client, ownerAccountIdBase58.toString(), owner, &err)) {
        return makeErrorJson(err);
    }
    if (ps_ffi_derive_vault_account_ids(programId, owner, vid, vaultCfg, vaultHolding) !=
        kFfiSuccess) {
        return makeErrorJson(QStringLiteral("derive vault accounts failed"));
    }

    const QString cfgHex = bytes32ToHexLower(vaultCfg);
    const QString holdingHex = bytes32ToHexLower(vaultHolding);

    const QByteArray cfgData = accountDataBytesFromHex(client, cfgHex, &err);
    if (cfgData.isEmpty()) {
        return makeErrorJson(err);
    }
    PsFfiDecodedVaultConfig decodedCfg{};
    if (ps_ffi_decode_vault_config(reinterpret_cast<const uint8_t*>(cfgData.constData()),
                                   static_cast<size_t>(cfgData.size()),
                                   &decodedCfg) != 0u) {
        return makeErrorJson(QStringLiteral("vault config decode failed"));
    }

    const QString holdingJson = invokeWalletString(client, "get_account_public", holdingHex);
    QString holdingBalanceHex;
    parseWalletAccountJson(holdingJson, nullptr, &holdingBalanceHex);

    QJsonObject payload;
    payload.insert(QStringLiteral("vault_id"), static_cast<qint64>(vid));
    payload.insert(QStringLiteral("vault_config_account_id_hex"), cfgHex);
    payload.insert(QStringLiteral("vault_holding_account_id_hex"), holdingHex);
    payload.insert(QStringLiteral("vault_config"),
                   QJsonObject{
                       {QStringLiteral("vault_id"), static_cast<qint64>(decodedCfg.vault_id)},
                       {QStringLiteral("next_stream_id"), static_cast<qint64>(decodedCfg.next_stream_id)},
                       {QStringLiteral("total_allocated_lo"), static_cast<qint64>(decodedCfg.total_allocated_lo)},
                       {QStringLiteral("total_allocated_hi"), static_cast<qint64>(decodedCfg.total_allocated_hi)},
                   });
    payload.insert(QStringLiteral("vault_holding_balance_hex"), holdingBalanceHex);
    return makeOkJson(payload);
}

QString PaymentStreamsModuleImpl::getStreamStatus(const QVariant& ownerAccountIdBase58,
                                                  const QVariant& vaultId,
                                                  const QVariant& streamId) {
    LogosAPIClient* client = walletClientOrNull(modules().api);
    if (client == nullptr) {
        return makeErrorJson(QStringLiteral("logos_execution_zone client unavailable (load wallet first)"));
    }
    bool ok = false;
    const quint64 vid = variantToU64(vaultId, &ok);
    const quint64 sid = variantToU64(streamId, &ok);
    if (!ok) {
        return makeErrorJson(QStringLiteral("invalid numeric argument"));
    }

    uint8_t programId[32]{};
    uint8_t owner[32]{};
    uint8_t vaultCfg[32]{};
    uint8_t vaultHolding[32]{};
    uint8_t streamCfg[32]{};
    uint8_t clock[32]{};
    QString err;
    if (!programIdBytes(programId, &err) || !ownerBytesFromBase58(client, ownerAccountIdBase58.toString(), owner, &err) ||
        !clockBytes(clock, &err)) {
        return makeErrorJson(err);
    }
    if (ps_ffi_derive_vault_account_ids(programId, owner, vid, vaultCfg, vaultHolding) !=
            kFfiSuccess ||
        ps_ffi_derive_stream_config_account_id(programId, vaultCfg, sid, streamCfg) !=
            kFfiSuccess) {
        return makeErrorJson(QStringLiteral("derive stream account failed"));
    }

    const QString streamHex = bytes32ToHexLower(streamCfg);
    const QString clockHex = bytes32ToHexLower(clock);

    const QByteArray streamData = accountDataBytesFromHex(client, streamHex, &err);
    if (streamData.isEmpty()) {
        return makeErrorJson(err);
    }
    PsFfiDecodedStreamConfig decodedStream{};
    if (ps_ffi_decode_stream_config(reinterpret_cast<const uint8_t*>(streamData.constData()),
                                    static_cast<size_t>(streamData.size()),
                                    &decodedStream) != 0u) {
        return makeErrorJson(QStringLiteral("stream config decode failed"));
    }

    const QByteArray clockData = accountDataBytesFromHex(client, clockHex, &err);
    if (clockData.isEmpty()) {
        return makeErrorJson(err);
    }
    PsFfiDecodedClock decodedClock{};
    if (ps_ffi_decode_clock(reinterpret_cast<const uint8_t*>(clockData.constData()),
                            static_cast<size_t>(clockData.size()),
                            &decodedClock) != 0u) {
        return makeErrorJson(QStringLiteral("clock decode failed"));
    }

    PsFfiStreamFoldAtTime fold{};
    uint32_t guestError = 0;
    const auto foldStatus = ps_ffi_fold_stream_at(
        &decodedStream,
        decodedClock.timestamp,
        &fold,
        &guestError);
    if (foldStatus != kFfiSuccess) {
        return makeErrorJson(QStringLiteral("stream fold failed (%1)").arg(foldStatus));
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("vault_id"), static_cast<qint64>(vid));
    payload.insert(QStringLiteral("stream_id"), static_cast<qint64>(sid));
    payload.insert(QStringLiteral("stream_config_account_id_hex"), streamHex);
    payload.insert(QStringLiteral("as_of"), static_cast<qint64>(decodedClock.timestamp));
    payload.insert(QStringLiteral("stream_state"), static_cast<qint64>(decodedStream.stream_state));
    payload.insert(QStringLiteral("accrued_lo"), static_cast<qint64>(fold.accrued_lo));
    payload.insert(QStringLiteral("accrued_hi"), static_cast<qint64>(fold.accrued_hi));
    payload.insert(QStringLiteral("unaccrued_lo"), static_cast<qint64>(fold.unaccrued_lo));
    payload.insert(QStringLiteral("unaccrued_hi"), static_cast<qint64>(fold.unaccrued_hi));
    return makeOkJson(payload);
}

QString PaymentStreamsModuleImpl::chainAction(const QVariant& operation, const QVariant& paramsJson) {
    const QString op = operation.toString().trimmed();
    QJsonParseError parseError {};
    const QJsonDocument doc = QJsonDocument::fromJson(paramsJson.toString().toUtf8(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !doc.isObject()) {
        return makeErrorJson(QStringLiteral("chainAction params must be a JSON object"));
    }
    const QJsonObject p = doc.object();
    const auto qv = [&p](const char* key) -> QVariant {
        return p.value(QString::fromUtf8(key)).toVariant();
    };

    if (op == QLatin1String("initializeVault")) {
        return initializeVault(qv("signer"), qv("vault_id"));
    }
    if (op == QLatin1String("deposit")) {
        return deposit(qv("signer"), qv("vault_id"), qv("amount_lo"), qv("amount_hi"));
    }
    if (op == QLatin1String("withdraw")) {
        return withdraw(qv("signer"), qv("vault_id"), qv("amount_lo"), qv("amount_hi"), qv("withdraw_to"));
    }
    if (op == QLatin1String("createStream")) {
        return createStream(qv("signer"),
                            qv("vault_id"),
                            qv("stream_id"),
                            qv("provider"),
                            qv("rate"),
                            qv("allocation_lo"),
                            qv("allocation_hi"));
    }
    if (op == QLatin1String("pauseStream")) {
        return pauseStream(qv("signer"), qv("vault_id"), qv("stream_id"));
    }
    if (op == QLatin1String("resumeStream")) {
        return resumeStream(qv("signer"), qv("vault_id"), qv("stream_id"));
    }
    if (op == QLatin1String("topUpStream")) {
        return topUpStream(qv("signer"), qv("vault_id"), qv("stream_id"), qv("increase_lo"), qv("increase_hi"));
    }
    if (op == QLatin1String("closeStream")) {
        return closeStream(qv("signer"), qv("vault_id"), qv("stream_id"), qv("authority"));
    }
    if (op == QLatin1String("claim")) {
        return claim(qv("provider"), qv("vault_id"), qv("stream_id"));
    }
    if (op == QLatin1String("getVaultStatus")) {
        return getVaultStatus(qv("owner"), qv("vault_id"), {});
    }
    if (op == QLatin1String("getStreamStatus")) {
        return getStreamStatus(qv("owner"), qv("vault_id"), qv("stream_id"));
    }
    return makeErrorJson(QStringLiteral("unknown chainAction operation: %1").arg(op));
}
