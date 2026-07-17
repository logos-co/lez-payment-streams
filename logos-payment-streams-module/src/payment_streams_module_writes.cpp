#include "payment_streams_module_impl.h"
#include "payment_streams_module_inventory.h"

#include <QDir>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QJsonParseError>
#include <QMetaType>
#include <QProcess>
#include <QProcessEnvironment>
#include <QVariant>

#include <functional>

#include <logos_api.h>
#include <logos_api_client.h>
#include <logos_sdk.h>

#include "payment_streams_ffi_bridge.h"
#include "payment_streams_privacy_policy.h"

#include <cstring>

namespace {

constexpr int kAccountIdHexLen = 64;
constexpr uint8_t kPrivacyTierPublic = 0;
constexpr uint8_t kPrivacyTierPseudonymousFunder = 1;
constexpr uint8_t kPrivacyTierReadFromChain = 255;
constexpr uint32_t kFfiSuccess = 0u;
// Private submit runs the privacy-preserving prover. Default LogosAPIClient
// Timeout is 20s, which is too short even for RISC0_DEV_MODE stub receipts on
// a cold path and far too short for real proving.
constexpr int kPrivateSubmitTimeoutMs = 600000;

enum class VaultIxLayout : uint8_t {
    InitOrDeposit3,
    StreamOwner5,
    StreamAuthority6,
};

QString parseWalletSubmitJson(const QString& walletJson, QJsonObject* fieldsOut, QString* errorOut);

QByteArray accountDataBytesFromHex(LogosExecutionZone& wallet, const QString& accountHex, QString* errorOut);

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

QString invokeWalletQtString(LogosAPIClient* client,
                             const char* method,
                             const QVariant& arg = {},
                             Timeout timeout = Timeout(),
                             QString* errorOut = nullptr) {
    if (client == nullptr) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("logos_execution_zone client unavailable");
        }
        return {};
    }
    const QString methodName = QString::fromUtf8(method);
    QVariant result;
    if (arg.isValid() && !arg.isNull()) {
        result = client->invokeRemoteMethod(
            QStringLiteral("logos_execution_zone"), methodName, arg, timeout);
    } else {
        result = client->invokeRemoteMethod(
            QStringLiteral("logos_execution_zone"), methodName, QVariantList{}, timeout);
    }
    if (!result.isValid()) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("IPC returned invalid result (timeout or dispatch failure; timeout_ms=%1)")
                            .arg(timeout.ms);
        }
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
        cfg.clock10Base58 = QStringLiteral("4BdcjoXkq786TMWcBGGHqcxeLYMZmn17rL4eM9ZyRWNU");
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

QString signerAccountIdHex(LogosExecutionZone& wallet, const QString& signerField, QString* errorOut) {
    const QString trimmed = signerField.trimmed();
    if (trimmed.size() == kAccountIdHexLen && trimmed.indexOf(QLatin1Char('{')) < 0) {
        return trimmed.toLower();
    }
    const QString hex = walletAccountIdHexFromBase58(wallet, trimmed);
    if (hex.size() != kAccountIdHexLen) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("invalid signer account id");
        }
        return {};
    }
    return hex.toLower();
}

bool ownerBytesFromSignerField(LogosExecutionZone& wallet, const QString& signerField, uint8_t out[32], QString* errorOut) {
    const QString hex = signerAccountIdHex(wallet, signerField, errorOut);
    if (hex.size() != kAccountIdHexLen) {
        return false;
    }
    return hex32FromQString(hex, out);
}

bool loadVaultConfigOnChain(LogosExecutionZone& wallet,
                            const uint8_t programId[32],
                            const uint8_t vaultOwner[32],
                            quint64 vaultId,
                            PsFfiDecodedVaultConfig* decodedOut,
                            QString* errorOut) {
    uint8_t vaultCfgAccount[32]{};
    uint8_t vaultHoldingAccount[32]{};
    if (ps_ffi_derive_vault_account_ids(programId, vaultOwner, vaultId, vaultCfgAccount, vaultHoldingAccount) !=
        kFfiSuccess) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("derive vault accounts failed");
        }
        return false;
    }
    const QByteArray vaultCfgData =
        accountDataBytesFromHex(wallet, bytes32ToHexLower(vaultCfgAccount), errorOut);
    if (vaultCfgData.isEmpty()) {
        return false;
    }
    if (ps_ffi_decode_vault_config(reinterpret_cast<const uint8_t*>(vaultCfgData.constData()),
                                   static_cast<size_t>(vaultCfgData.size()),
                                   decodedOut) != 0u) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("vault config decode failed");
        }
        return false;
    }
    return true;
}

bool vaultPrivacyTierForSubmit(LogosExecutionZone& wallet,
                               const uint8_t programId[32],
                               const uint8_t vaultOwner[32],
                               quint64 vaultId,
                               uint8_t initPrivacyTier,
                               uint8_t* tierOut,
                               PsFfiDecodedVaultConfig* decodedOut,
                               QString* errorOut) {
    if (initPrivacyTier != kPrivacyTierReadFromChain) {
        *tierOut = initPrivacyTier;
        if (decodedOut != nullptr && initPrivacyTier == kPrivacyTierPseudonymousFunder) {
            std::memset(decodedOut, 0, sizeof(PsFfiDecodedVaultConfig));
            std::memcpy(decodedOut->owner, vaultOwner, 32);
            decodedOut->privacy_tier = kPrivacyTierPseudonymousFunder;
            decodedOut->vault_id = vaultId;
        }
        return true;
    }
    PsFfiDecodedVaultConfig decoded{};
    if (!loadVaultConfigOnChain(wallet, programId, vaultOwner, vaultId, &decoded, errorOut)) {
        return false;
    }
    *tierOut = decoded.privacy_tier;
    if (decodedOut != nullptr) {
        *decodedOut = decoded;
    }
    return true;
}

QString resolutionForPseudonymousSlot(VaultIxLayout layout, int index, const QString& signerHexLower, const QString& authorityHexLower) {
    switch (layout) {
    case VaultIxLayout::InitOrDeposit3:
        return index == 2 ? QStringLiteral("private") : QStringLiteral("public_no_sign");
    case VaultIxLayout::StreamOwner5:
        return index == 3 ? QStringLiteral("private") : QStringLiteral("public_no_sign");
    case VaultIxLayout::StreamAuthority6:
        if (index == 3) {
            return QStringLiteral("private");
        }
        if (index == 4) {
            return QStringLiteral("public_sign");
        }
        return QStringLiteral("public_no_sign");
    }
    return QStringLiteral("public_no_sign");
}

QJsonArray accountSlotsJsonForSubmit(VaultIxLayout layout,
                                     bool pseudonymousFunder,
                                     const QStringList& accountHexIds,
                                     const QList<bool>& signingFlags) {
    QJsonArray slotList;
    for (int i = 0; i < accountHexIds.size(); ++i) {
        QJsonObject slot;
        slot.insert(QStringLiteral("account_id_hex"), accountHexIds.at(i));
        if (pseudonymousFunder) {
            slot.insert(QStringLiteral("resolution"), resolutionForPseudonymousSlot(layout, i, {}, {}));
        } else {
            slot.insert(QStringLiteral("resolution"),
                        signingFlags.at(i) ? QStringLiteral("public_sign") : QStringLiteral("public_no_sign"));
        }
        slotList.append(slot);
    }
    return slotList;
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

bool ffiSerializeInitializeVault(uint64_t vaultId, uint8_t privacyTier, QByteArray* out, QString* errorOut) {
    size_t required = 0;
    if (ps_ffi_serialize_initialize_vault(vaultId, privacyTier, nullptr, 0, &required) != kFfiSuccess) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("initialize_vault serialize sizing failed");
        }
        return false;
    }
    out->resize(static_cast<int>(required));
    if (ps_ffi_serialize_initialize_vault(vaultId,
                                          privacyTier,
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

QList<uint8_t> instructionBytesForWallet(const QByteArray& borshBytes, QString* errorOut) {
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

bool chainUsesTestnetSubmit() {
    // Retirement pending live-testnet verification; dispatched unconditionally to
    // FFI in Step 26. Remove chainUsesTestnetSubmit,
    // submitGenericPublicViaTestnetHelper, tools/lez-testnet-submit/, and
    // LEZ_TESTNET_SUBMIT plumbing once MODE=store CHAIN=testnet passes on the
    // live testnet.
    return false;
}

QString buildGenericPublicPayloadJson(const QStringList& accountHexIds,
                                      const QList<bool>& signingFlags,
                                      const QList<uint8_t>& instructionBytes,
                                      const QString& programIdHex) {
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

    payload.insert(QStringLiteral("program_id_hex"), programIdHex);

    return QJsonDocument(payload).toJson(QJsonDocument::Compact);
}

QString submitGenericPublicViaFfi(LogosAPI* api,
                                  const QString& payloadJson,
                                  QString* errorOut) {
    // send_generic_public_transaction_json is a repo-local Qt-only patch
    // (N10); not in the lp typed API, so dispatch dynamically through the Qt
    // LogosAPIClient. The multi-arg fallback below likewise uses Qt dynamic
    // dispatch (invokeRemoteMethod) since the lp typed wrapper would require
    // nlohmann::json marshaling for LogosList/LogosMap.
    LogosAPIClient* qtClient = walletQtClientOrNull(api);
    QString walletJson =
        invokeWalletQtString(qtClient, "send_generic_public_transaction_json", payloadJson);
    if (walletJson.isEmpty()) {
        QJsonParseError parseError{};
        const QJsonDocument doc = QJsonDocument::fromJson(payloadJson.toUtf8(), &parseError);
        if (parseError.error != QJsonParseError::NoError || !doc.isObject()) {
            if (errorOut != nullptr) {
                *errorOut = QStringLiteral("send_generic_public_transaction returned empty");
            }
            return makeErrorJson(QStringLiteral("send_generic_public_transaction returned empty"));
        }
        const QJsonObject obj = doc.object();
        QStringList accountHexIds;
        for (const QJsonValue value : obj.value(QStringLiteral("account_ids")).toArray()) {
            accountHexIds.append(value.toString());
        }
        QList<bool> signingFlags;
        for (const QJsonValue value : obj.value(QStringLiteral("signing_requirements")).toArray()) {
            signingFlags.append(value.toBool());
        }
        const QByteArray instructionBytes =
            QByteArray::fromHex(obj.value(QStringLiteral("instruction_hex")).toString().toLatin1());
        QList<uint8_t> instructionList = bytesToUint8List(instructionBytes);
        const QString programIdHex = obj.value(QStringLiteral("program_id_hex")).toString();
        if (qtClient != nullptr) {
            walletJson = qtClient->invokeRemoteMethod(
                QStringLiteral("logos_execution_zone"),
                QStringLiteral("send_generic_public_transaction"),
                QVariant::fromValue(accountHexIds),
                QVariant::fromValue(signingFlags),
                QVariant::fromValue(instructionList),
                QVariant::fromValue(programIdHex)).toString();
        }
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

QString submitGenericPublicViaTestnetHelper(const QString& payloadJson, QString* errorOut) {
    QByteArray walletConfig = qgetenv("LEZ_TESTNET_WALLET_CONFIG");
    QByteArray walletStorage = qgetenv("LEZ_TESTNET_WALLET_STORAGE");
    if (walletConfig.isEmpty()) {
        walletConfig = qgetenv("WALLET_CONFIG");
    }
    if (walletStorage.isEmpty()) {
        walletStorage = qgetenv("WALLET_STORAGE");
    }
    if (walletConfig.isEmpty() || walletStorage.isEmpty()) {
        const QString msg = QStringLiteral(
            "CHAIN=testnet requires WALLET_CONFIG/WALLET_STORAGE (unified rc5 testnet wallet)");
        if (errorOut != nullptr) {
            *errorOut = msg;
        }
        return makeErrorJson(msg);
    }

    const QByteArray helperEnv = qgetenv("LEZ_TESTNET_SUBMIT");
    const QString helperProgram =
        helperEnv.isEmpty() ? QStringLiteral("lez-testnet-submit") : QString::fromUtf8(helperEnv);

    QProcess process;
    process.setProgram(helperProgram);
    QStringList args{QStringLiteral("submit-public-tx"),
                     QStringLiteral("--wallet-config"),
                     QString::fromUtf8(walletConfig),
                     QStringLiteral("--wallet-storage"),
                     QString::fromUtf8(walletStorage)};
    const QByteArray guestBin = qgetenv("PAYMENT_STREAMS_GUEST_BIN");
    if (!guestBin.isEmpty()) {
        args << QStringLiteral("--program-elf") << QString::fromUtf8(guestBin);
    }
    process.setArguments(args);
    process.setProcessChannelMode(QProcess::MergedChannels);

    QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    process.setProcessEnvironment(env);

    process.start();
    if (!process.waitForStarted(15000)) {
        const QString msg = QStringLiteral("lez-testnet-submit failed to start: %1").arg(process.errorString());
        if (errorOut != nullptr) {
            *errorOut = msg;
        }
        return makeErrorJson(msg);
    }
    process.write(payloadJson.toUtf8());
    process.closeWriteChannel();
    if (!process.waitForFinished(300000)) {
        process.kill();
        const QString msg = QStringLiteral("lez-testnet-submit timed out");
        if (errorOut != nullptr) {
            *errorOut = msg;
        }
        return makeErrorJson(msg);
    }

    const QString walletJson = QString::fromUtf8(process.readAllStandardOutput()).trimmed();
    if (process.exitCode() != 0) {
        QJsonObject fields;
        if (!walletJson.isEmpty()) {
            return parseWalletSubmitJson(walletJson, &fields, errorOut);
        }
        const QString msg =
            QStringLiteral("lez-testnet-submit exit %1").arg(process.exitCode());
        if (errorOut != nullptr) {
            *errorOut = msg;
        }
        return makeErrorJson(msg);
    }
    if (walletJson.isEmpty()) {
        const QString msg = QStringLiteral("lez-testnet-submit returned empty stdout");
        if (errorOut != nullptr) {
            *errorOut = msg;
        }
        return makeErrorJson(msg);
    }
    QJsonObject fields;
    return parseWalletSubmitJson(walletJson, &fields, errorOut);
}

QList<uint8_t> walletAuthenticatedTransferElfBytes(LogosAPI* api, QString* errorOut) {
    // authenticated_transfer_elf is in the lp typed API but returns LogosMap
    // (nlohmann::json); the byte-extraction here is QVariant-shaped (Qt path),
    // so dispatch dynamically through the Qt LogosAPIClient to preserve the
    // QList<uint8_t>/QByteArray/QStringList handling the caller expects.
    LogosAPIClient* qtClient = walletQtClientOrNull(api);
    if (qtClient == nullptr) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("wallet client missing");
        }
        return {};
    }
    const QVariant raw =
        qtClient->invokeRemoteMethod(QStringLiteral("logos_execution_zone"), QStringLiteral("authenticated_transfer_elf"));
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

QString submitGenericPublic(LogosAPI* api,
                            const QStringList& accountHexIds,
                            const QList<bool>& signingFlags,
                            const QList<uint8_t>& instructionBytes,
                            const QString& programIdHex,
                            QString* errorOut) {
    const QString payloadJson = buildGenericPublicPayloadJson(accountHexIds,
                                                              signingFlags,
                                                              instructionBytes,
                                                              programIdHex);
    if (chainUsesTestnetSubmit()) {
        return submitGenericPublicViaTestnetHelper(payloadJson, errorOut);
    }
    return submitGenericPublicViaFfi(api, payloadJson, errorOut);
}

QString submitGenericPrivateViaFfi(LogosAPI* api, const QJsonObject& payload, QString* errorOut) {
    LogosAPIClient* qtClient = walletQtClientOrNull(api);
    QString invokeErr;
    // Must pass QString (not QByteArray): the wallet slot is std::string/QString.
    // QByteArray variants fail QRemoteObjects dispatch and return an empty QString.
    const QString payloadJson =
        QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact));
    const QString walletJson = invokeWalletQtString(
        qtClient,
        "send_generic_private_transaction_json",
        payloadJson,
        Timeout(kPrivateSubmitTimeoutMs),
        &invokeErr);
    if (walletJson.isEmpty()) {
        const QString msg = invokeErr.isEmpty()
            ? QStringLiteral("send_generic_private_transaction_json returned empty")
            : QStringLiteral("send_generic_private_transaction_json: %1").arg(invokeErr);
        if (errorOut != nullptr) {
            *errorOut = msg;
        }
        return makeErrorJson(msg);
    }
    QJsonObject fields;
    return parseWalletSubmitJson(walletJson, &fields, errorOut);
}

struct VaultSubmitContext {
    uint8_t programId[32]{};
    uint8_t vaultOwner[32]{};
    quint64 vaultId = 0;
    uint8_t initPrivacyTier = kPrivacyTierReadFromChain;
    VaultIxLayout layout = VaultIxLayout::InitOrDeposit3;
    bool requireAuthTransferDep = false;
    bool enforceDepositSignerEqualsOwner = false;
};

QString buildAndSubmit(LogosExecutionZone& wallet,
                       LogosAPI* api,
                       const QString& signerBase58,
                       const QByteArray& instructionBytes,
                       const QByteArray& accountsHex,
                       QString* errorOut,
                       const VaultSubmitContext* vaultCtx = nullptr) {
    QString loadErr;
    if (!ensureFixtureLoaded(&loadErr)) {
        return makeErrorJson(loadErr);
    }

    const QString signerHex = signerAccountIdHex(wallet, signerBase58, &loadErr);
    if (signerHex.size() != kAccountIdHexLen) {
        return makeErrorJson(loadErr.isEmpty() ? QStringLiteral("invalid signer account") : loadErr);
    }

    const QStringList accountIds = splitAccountsHex(accountsHex);
    if (accountIds.isEmpty()) {
        return makeErrorJson(QStringLiteral("planned account list is empty"));
    }

    const QList<bool> signing = signingRequirementsForAccounts(accountIds, signerHex);

    const QList<uint8_t> instructionList = instructionBytesForWallet(instructionBytes, &loadErr);
    if (instructionList.isEmpty()) {
        return makeErrorJson(loadErr.isEmpty() ? QStringLiteral("instruction encoding failed") : loadErr);
    }

    uint8_t privacyTier = kPrivacyTierPublic;
    PsFfiDecodedVaultConfig vaultCfg{};
    if (vaultCtx != nullptr) {
        if (!vaultPrivacyTierForSubmit(wallet,
                                       vaultCtx->programId,
                                       vaultCtx->vaultOwner,
                                       vaultCtx->vaultId,
                                       vaultCtx->initPrivacyTier,
                                       &privacyTier,
                                       &vaultCfg,
                                       &loadErr)) {
            return makeErrorJson(loadErr);
        }
    }

    const QString vaultOwnerHex =
        vaultCtx != nullptr ? bytes32ToHexLower(vaultCfg.owner) : QString();
    const bool enforceDepositSigner =
        vaultCtx != nullptr && vaultCtx->enforceDepositSignerEqualsOwner;
    const auto submitDecision = payment_streams_privacy::decideVaultSubmitPath(
        privacyTier, enforceDepositSigner, signerHex, vaultOwnerHex);
    if (!submitDecision.ok) {
        return makeErrorJson(submitDecision.error);
    }

    if (submitDecision.path == payment_streams_privacy::VaultSubmitPath::Private) {
        // Pass the guest ELF by path (or PAYMENT_STREAMS_GUEST_BIN in the wallet
        // process). Embedding program_elf_hex (~700KB+) exceeds the practical
        // QRemoteObjects payload size used for inter-module IPC and returns empty.
        const QString guestPath = guestElfPath();
        if (guestPath.isEmpty() || !QFile::exists(guestPath)) {
            return makeErrorJson(QStringLiteral("guest ELF missing at %1").arg(guestPath));
        }
        QJsonObject payload;
        payload.insert(QStringLiteral("account_slots"),
                       accountSlotsJsonForSubmit(vaultCtx != nullptr ? vaultCtx->layout : VaultIxLayout::InitOrDeposit3,
                                                 true,
                                                 accountIds,
                                                 signing));
        payload.insert(QStringLiteral("instruction_hex"), QString::fromLatin1(instructionBytes.toHex()));
        payload.insert(QStringLiteral("program_elf_path"), guestPath);
        if (vaultCtx != nullptr && vaultCtx->requireAuthTransferDep) {
            payload.insert(QStringLiteral("include_authenticated_transfer_elf"), true);
        }
        return submitGenericPrivateViaFfi(api, payload, errorOut);
    }

    const QString programIdHex = fixtureConfig().programIdHex;
    return submitGenericPublic(api, accountIds, signing, instructionList, programIdHex, errorOut);
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

}  // namespace

QString PaymentStreamsModuleImpl::initializeVault(const QVariant& signerAccountIdBase58,
                                                  const QVariant& vaultId,
                                                  const QVariant& privacyTier) {
    LogosExecutionZone& wallet = modules().logos_execution_zone;
    bool ok = false;
    const quint64 vid = variantToU64(vaultId, &ok);
    if (!ok) {
        return makeErrorJson(QStringLiteral("vaultId must be unsigned integer"));
    }

    uint8_t tierByte = kPrivacyTierPublic;
    if (privacyTier.isValid() && !privacyTier.isNull()) {
        const quint64 tierVal = variantToU64(privacyTier, &ok);
        if (!ok || (tierVal != kPrivacyTierPublic && tierVal != kPrivacyTierPseudonymousFunder)) {
            return makeErrorJson(QStringLiteral("privacy_tier must be 0 (Public) or 1 (PseudonymousFunder)"));
        }
        tierByte = static_cast<uint8_t>(tierVal);
    }

    uint8_t programId[32]{};
    uint8_t owner[32]{};
    QString err;
    if (!programIdBytes(programId, &err)) {
        return makeErrorJson(err);
    }
    if (!ownerBytesFromSignerField(wallet, signerAccountIdBase58.toString(), owner, &err)) {
        return makeErrorJson(err);
    }

    QByteArray instruction;
    if (!ffiSerializeInitializeVault(vid, tierByte, &instruction, &err)) {
        return makeErrorJson(err);
    }

    QByteArray accountsHex;
    if (!ffiPlanInitializeVault(programId, owner, vid, &accountsHex, &err)) {
        return makeErrorJson(err);
    }

    VaultSubmitContext ctx{};
    std::memcpy(ctx.programId, programId, 32);
    std::memcpy(ctx.vaultOwner, owner, 32);
    ctx.vaultId = vid;
    ctx.initPrivacyTier = tierByte;
    ctx.layout = VaultIxLayout::InitOrDeposit3;
    return buildAndSubmit(wallet, modules().api, signerAccountIdBase58.toString(), instruction, accountsHex, &err, &ctx);
}

QString PaymentStreamsModuleImpl::deposit(const QVariant& signerAccountIdBase58,
                                          const QVariant& vaultId,
                                          const QVariant& amountLo,
                                          const QVariant& amountHi) {
    LogosExecutionZone& wallet = modules().logos_execution_zone;
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
    if (!ownerBytesFromSignerField(wallet, signerAccountIdBase58.toString(), owner, &err)) {
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

    VaultSubmitContext ctx{};
    std::memcpy(ctx.programId, programId, 32);
    std::memcpy(ctx.vaultOwner, owner, 32);
    ctx.vaultId = vid;
    ctx.layout = VaultIxLayout::InitOrDeposit3;
    ctx.requireAuthTransferDep = true;
    ctx.enforceDepositSignerEqualsOwner = true;
    return buildAndSubmit(wallet, modules().api, signerAccountIdBase58.toString(), instruction, accountsHex, &err, &ctx);
}

QString PaymentStreamsModuleImpl::withdraw(const QVariant& signerAccountIdBase58,
                                           const QVariant& vaultId,
                                           const QVariant& amountLo,
                                           const QVariant& amountHi,
                                           const QVariant& withdrawToAccountIdBase58) {
    LogosExecutionZone& wallet = modules().logos_execution_zone;
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
    if (!ownerBytesFromBase58(wallet, signerAccountIdBase58.toString(), owner, &err)) {
        return makeErrorJson(err);
    }
    if (!ownerBytesFromBase58(wallet, withdrawBase58, withdrawTo, &err)) {
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

    return buildAndSubmit(wallet, modules().api, signerAccountIdBase58.toString(), instruction, accountsHex, &err);
}

QString PaymentStreamsModuleImpl::createStream(const QVariant& signerAccountIdBase58,
                                             const QVariant& vaultId,
                                             const QVariant& streamId,
                                             const QVariant& providerAccountIdBase58,
                                             const QVariant& rateTokensPerSecond,
                                             const QVariant& allocationLo,
                                             const QVariant& allocationHi) {
    LogosExecutionZone& wallet = modules().logos_execution_zone;
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
    if (!ownerBytesFromSignerField(wallet, signerAccountIdBase58.toString(), owner, &err)) {
        return makeErrorJson(err);
    }
    if (!ownerBytesFromBase58(wallet, providerAccountIdBase58.toString(), provider, &err)) {
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

    VaultSubmitContext ctx{};
    std::memcpy(ctx.programId, programId, 32);
    std::memcpy(ctx.vaultOwner, owner, 32);
    ctx.vaultId = vid;
    ctx.layout = VaultIxLayout::StreamOwner5;
    const QString submitResult =
        buildAndSubmit(wallet, modules().api, signerAccountIdBase58.toString(), instruction, accountsHex, &err, &ctx);
    QJsonParseError submitParse{};
    const QJsonDocument submitDoc = QJsonDocument::fromJson(submitResult.toUtf8(), &submitParse);
    if (submitParse.error == QJsonParseError::NoError && submitDoc.isObject()) {
        const QJsonObject submitObj = submitDoc.object();
        if (submitObj.value(QStringLiteral("status")).toString() == QLatin1String("ok") &&
            submitObj.value(QStringLiteral("success")).toBool()) {
            paymentStreamsModuleRecordStreamInventory(vid, sid);
        }
    }
    return submitResult;
}

QString PaymentStreamsModuleImpl::pauseStream(const QVariant& signerAccountIdBase58,
                                              const QVariant& vaultId,
                                              const QVariant& streamId) {
    LogosExecutionZone& wallet = modules().logos_execution_zone;
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
    if (!programIdBytes(programId, &err) || !ownerBytesFromSignerField(wallet, signerAccountIdBase58.toString(), owner, &err) ||
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

    VaultSubmitContext ctx{};
    std::memcpy(ctx.programId, programId, 32);
    std::memcpy(ctx.vaultOwner, owner, 32);
    ctx.vaultId = vid;
    ctx.layout = VaultIxLayout::StreamOwner5;
    return buildAndSubmit(wallet, modules().api, signerAccountIdBase58.toString(), instruction, accountsHex, &err, &ctx);
}

QString PaymentStreamsModuleImpl::resumeStream(const QVariant& signerAccountIdBase58,
                                               const QVariant& vaultId,
                                               const QVariant& streamId) {
    LogosExecutionZone& wallet = modules().logos_execution_zone;
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
    if (!programIdBytes(programId, &err) || !ownerBytesFromSignerField(wallet, signerAccountIdBase58.toString(), owner, &err) ||
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

    VaultSubmitContext ctx{};
    std::memcpy(ctx.programId, programId, 32);
    std::memcpy(ctx.vaultOwner, owner, 32);
    ctx.vaultId = vid;
    ctx.layout = VaultIxLayout::StreamOwner5;
    return buildAndSubmit(wallet, modules().api, signerAccountIdBase58.toString(), instruction, accountsHex, &err, &ctx);
}

QString PaymentStreamsModuleImpl::topUpStream(const QVariant& signerAccountIdBase58,
                                              const QVariant& vaultId,
                                              const QVariant& streamId,
                                              const QVariant& increaseLo,
                                              const QVariant& increaseHi) {
    LogosExecutionZone& wallet = modules().logos_execution_zone;
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
    if (!programIdBytes(programId, &err) || !ownerBytesFromSignerField(wallet, signerAccountIdBase58.toString(), owner, &err) ||
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

    VaultSubmitContext ctx{};
    std::memcpy(ctx.programId, programId, 32);
    std::memcpy(ctx.vaultOwner, owner, 32);
    ctx.vaultId = vid;
    ctx.layout = VaultIxLayout::StreamOwner5;
    return buildAndSubmit(wallet, modules().api, signerAccountIdBase58.toString(), instruction, accountsHex, &err, &ctx);
}

QString PaymentStreamsModuleImpl::closeStream(const QVariant& signerAccountIdBase58,
                                              const QVariant& vaultId,
                                              const QVariant& streamId,
                                              const QVariant& authorityAccountIdBase58) {
    LogosExecutionZone& wallet = modules().logos_execution_zone;
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
    if (!programIdBytes(programId, &err) || !ownerBytesFromSignerField(wallet, signerAccountIdBase58.toString(), owner, &err) ||
        !ownerBytesFromBase58(wallet, authorityBase58, authority, &err) || !clockBytes(clock, &err)) {
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

    VaultSubmitContext ctx{};
    std::memcpy(ctx.programId, programId, 32);
    std::memcpy(ctx.vaultOwner, owner, 32);
    ctx.vaultId = vid;
    ctx.layout = VaultIxLayout::StreamAuthority6;
    return buildAndSubmit(wallet, modules().api, authorityBase58, instruction, accountsHex, &err, &ctx);
}

QString PaymentStreamsModuleImpl::claim(const QVariant& ownerAccountIdBase58,
                                        const QVariant& providerAccountIdBase58,
                                        const QVariant& vaultId,
                                        const QVariant& streamId) {
    LogosExecutionZone& wallet = modules().logos_execution_zone;
    bool ok = false;
    const quint64 vid = variantToU64(vaultId, &ok);
    const quint64 sid = variantToU64(streamId, &ok);
    if (!ok) {
        return makeErrorJson(QStringLiteral("invalid numeric argument"));
    }

    // The stream coordinate (owner, vault_id, stream_id) is delivered to the
    // provider out-of-band (the stream-creation advertisement); the owner here
    // is the stream creator whose account id drives vault_config / stream_config
    // PDA derivation. It must NOT be read from the fixture manifest, which holds
    // the seeder baseline owner and diverges from the actual vault creator in the
    // fresh-owner localnet flow.
    const QString ownerBase58 = ownerAccountIdBase58.toString().trimmed();
    if (ownerBase58.isEmpty()) {
        return makeErrorJson(QStringLiteral("claim requires owner (stream creator) account id"));
    }

    QString err;
    uint8_t programId[32]{};
    uint8_t owner[32]{};
    uint8_t provider[32]{};
    uint8_t clock[32]{};
    if (!programIdBytes(programId, &err) || !ownerBytesFromSignerField(wallet, ownerBase58, owner, &err) ||
        !ownerBytesFromBase58(wallet, providerAccountIdBase58.toString(), provider, &err) ||
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

    VaultSubmitContext ctx{};
    std::memcpy(ctx.programId, programId, 32);
    std::memcpy(ctx.vaultOwner, owner, 32);
    ctx.vaultId = vid;
    ctx.layout = VaultIxLayout::StreamAuthority6;
    return buildAndSubmit(wallet, modules().api, providerAccountIdBase58.toString(), instruction, accountsHex, &err, &ctx);
}

QString PaymentStreamsModuleImpl::getVaultStatus(const QVariant& ownerAccountIdBase58,
                                                 const QVariant& vaultId,
                                                 const QVariant& streamId) {
    Q_UNUSED(streamId);
    LogosExecutionZone& wallet = modules().logos_execution_zone;
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
    if (!programIdBytes(programId, &err) || !ownerBytesFromBase58(wallet, ownerAccountIdBase58.toString(), owner, &err)) {
        return makeErrorJson(err);
    }
    if (ps_ffi_derive_vault_account_ids(programId, owner, vid, vaultCfg, vaultHolding) !=
        kFfiSuccess) {
        return makeErrorJson(QStringLiteral("derive vault accounts failed"));
    }

    const QString cfgHex = bytes32ToHexLower(vaultCfg);
    const QString holdingHex = bytes32ToHexLower(vaultHolding);

    const QByteArray cfgData = accountDataBytesFromHex(wallet, cfgHex, &err);
    if (cfgData.isEmpty()) {
        return makeErrorJson(err);
    }
    PsFfiDecodedVaultConfig decodedCfg{};
    if (ps_ffi_decode_vault_config(reinterpret_cast<const uint8_t*>(cfgData.constData()),
                                   static_cast<size_t>(cfgData.size()),
                                   &decodedCfg) != 0u) {
        return makeErrorJson(QStringLiteral("vault config decode failed"));
    }

    const QString holdingJson = QString::fromStdString(wallet.get_account_public(holdingHex.toStdString()));
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
    LogosExecutionZone& wallet = modules().logos_execution_zone;
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
    if (!programIdBytes(programId, &err) || !ownerBytesFromBase58(wallet, ownerAccountIdBase58.toString(), owner, &err) ||
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

    const QByteArray streamData = accountDataBytesFromHex(wallet, streamHex, &err);
    if (streamData.isEmpty()) {
        return makeErrorJson(err);
    }
    PsFfiDecodedStreamConfig decodedStream{};
    if (ps_ffi_decode_stream_config(reinterpret_cast<const uint8_t*>(streamData.constData()),
                                    static_cast<size_t>(streamData.size()),
                                    &decodedStream) != 0u) {
        return makeErrorJson(QStringLiteral("stream config decode failed"));
    }

    const QByteArray clockData = accountDataBytesFromHex(wallet, clockHex, &err);
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
        return initializeVault(qv("signer"), qv("vault_id"), qv("privacy_tier"));
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
        return claim(qv("owner"), qv("provider"), qv("vault_id"), qv("stream_id"));
    }
    if (op == QLatin1String("getVaultStatus")) {
        return getVaultStatus(qv("owner"), qv("vault_id"), {});
    }
    if (op == QLatin1String("getStreamStatus")) {
        return getStreamStatus(qv("owner"), qv("vault_id"), qv("stream_id"));
    }
    return makeErrorJson(QStringLiteral("unknown chainAction operation: %1").arg(op));
}
