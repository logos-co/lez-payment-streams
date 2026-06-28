#include "payment_streams_module_impl.h"

#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QMetaType>
#include <QVariant>

#include <logos_api.h>
#include <logos_api_client.h>
#include <logos_sdk.h>

#include "payment_streams_ffi_bridge.h"

namespace {

constexpr const char* kDefaultClock10Base58 = "4BdcjoXkq786TMWcBGGHqcxeLYMZmn17rL4eM9ZyRWNU";

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

QString walletAccountIdHexFromBase58(LogosAPIClient* client, const QString& accountIdBase58) {
    const QString trimmed = accountIdBase58.trimmed();
    if (trimmed.isEmpty()) {
        return {};
    }
    return invokeWalletString(client, "account_id_from_base58", trimmed);
}

bool parseWalletAccountJson(const QString& accountJson, QByteArray* dataOut, QString* errorOut) {
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
    if (dataOut != nullptr) {
        *dataOut = data;
    }
    return true;
}

QByteArray accountDataBytesFromBase58(LogosAPIClient* client, const QString& accountIdBase58, QString* errorOut) {
    const QString accountIdHex = walletAccountIdHexFromBase58(client, accountIdBase58);
    if (accountIdHex.isEmpty()) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("account_id_from_base58 failed or returned empty");
        }
        return {};
    }
    const QString accountJson = invokeWalletString(client, "get_account_public", accountIdHex);
    if (accountJson.isEmpty()) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("get_account_public failed or returned empty");
        }
        return {};
    }
    QByteArray data;
    if (!parseWalletAccountJson(accountJson, &data, errorOut)) {
        return {};
    }
    return data;
}

QString ffiStatusMessage(uint32_t status) {
    switch (status) {
    case 0u:
        return QStringLiteral("success");
    case 1u:
        return QStringLiteral("null pointer");
    case 2u:
        return QStringLiteral("malformed account data");
    case 3u:
        return QStringLiteral("unsupported account version");
    default:
        return QStringLiteral("decode failed (status %1)").arg(status);
    }
}

QJsonObject vaultConfigToJson(const PsFfiDecodedVaultConfig& decoded) {
    QJsonObject obj;
    obj.insert(QStringLiteral("version"), static_cast<qint64>(decoded.version));
    obj.insert(QStringLiteral("privacy_tier"), static_cast<qint64>(decoded.privacy_tier));
    obj.insert(QStringLiteral("owner_hex"),
               QString::fromLatin1(QByteArray(reinterpret_cast<const char*>(decoded.owner), 32).toHex()));
    obj.insert(QStringLiteral("vault_id"), static_cast<qint64>(decoded.vault_id));
    obj.insert(QStringLiteral("next_stream_id"), static_cast<qint64>(decoded.next_stream_id));
    obj.insert(QStringLiteral("total_allocated_lo"), static_cast<qint64>(decoded.total_allocated_lo));
    obj.insert(QStringLiteral("total_allocated_hi"), static_cast<qint64>(decoded.total_allocated_hi));
    return obj;
}

QJsonObject vaultHoldingToJson(const PsFfiDecodedVaultHolding& decoded) {
    QJsonObject obj;
    obj.insert(QStringLiteral("version"), static_cast<qint64>(decoded.version));
    return obj;
}

quint64 chainTimestampToFoldSeconds(quint64 ts) {
    if (ts > 1'000'000'000'000ULL) {
        return ts / 1000;
    }
    return ts;
}

QJsonObject streamConfigToJson(const PsFfiDecodedStreamConfig& decoded) {
    QJsonObject obj;
    obj.insert(QStringLiteral("version"), static_cast<qint64>(decoded.version));
    obj.insert(QStringLiteral("stream_state"), static_cast<qint64>(decoded.stream_state));
    obj.insert(QStringLiteral("stream_id"), static_cast<qint64>(decoded.stream_id));
    obj.insert(QStringLiteral("provider_hex"),
               QString::fromLatin1(QByteArray(reinterpret_cast<const char*>(decoded.provider), 32).toHex()));
    obj.insert(QStringLiteral("rate_tokens_per_second"), static_cast<qint64>(decoded.rate_tokens_per_second));
    obj.insert(QStringLiteral("allocation_lo"), static_cast<qint64>(decoded.allocation_lo));
    obj.insert(QStringLiteral("allocation_hi"), static_cast<qint64>(decoded.allocation_hi));
    obj.insert(QStringLiteral("accrued_lo"), static_cast<qint64>(decoded.accrued_lo));
    obj.insert(QStringLiteral("accrued_hi"), static_cast<qint64>(decoded.accrued_hi));
    // On-chain checkpoint (LEZ 510+ ms); fold math uses seconds (see accrued_as_of_seconds).
    obj.insert(QStringLiteral("accrued_as_of"), static_cast<qint64>(decoded.accrued_as_of));
    obj.insert(QStringLiteral("accrued_as_of_seconds"),
               static_cast<qint64>(chainTimestampToFoldSeconds(decoded.accrued_as_of)));
    return obj;
}

QJsonObject clockToJson(const PsFfiDecodedClock& decoded) {
    QJsonObject obj;
    obj.insert(QStringLiteral("block_id"), static_cast<qint64>(decoded.block_id));
    // Already normalized to seconds by ps_ffi_decode_clock (LEZ 510+ ms on wire).
    obj.insert(QStringLiteral("timestamp"), static_cast<qint64>(decoded.timestamp));
    obj.insert(QStringLiteral("timestamp_seconds"), static_cast<qint64>(decoded.timestamp));
    return obj;
}

QString decodeVaultConfigPayload(LogosAPIClient* client, const QString& accountIdBase58) {
    QString readError;
    const QByteArray data = accountDataBytesFromBase58(client, accountIdBase58, &readError);
    if (data.isEmpty()) {
        return makeErrorJson(readError.isEmpty() ? QStringLiteral("empty account data") : readError);
    }
    PsFfiDecodedVaultConfig decoded{};
    const uint32_t status = ps_ffi_decode_vault_config(
        reinterpret_cast<const uint8_t*>(data.constData()), static_cast<size_t>(data.size()), &decoded);
    if (status != 0u) {
        return makeErrorJson(ffiStatusMessage(status));
    }
    QJsonObject payload;
    payload.insert(QStringLiteral("account_id_base58"), accountIdBase58.trimmed());
    payload.insert(QStringLiteral("decoded"), vaultConfigToJson(decoded));
    return makeOkJson(payload);
}

QString decodeVaultHoldingPayload(LogosAPIClient* client, const QString& accountIdBase58) {
    QString readError;
    const QByteArray data = accountDataBytesFromBase58(client, accountIdBase58, &readError);
    if (data.isEmpty()) {
        return makeErrorJson(readError.isEmpty() ? QStringLiteral("empty account data") : readError);
    }
    PsFfiDecodedVaultHolding decoded{};
    const uint32_t status = ps_ffi_decode_vault_holding(
        reinterpret_cast<const uint8_t*>(data.constData()), static_cast<size_t>(data.size()), &decoded);
    if (status != 0u) {
        return makeErrorJson(ffiStatusMessage(status));
    }
    QJsonObject payload;
    payload.insert(QStringLiteral("account_id_base58"), accountIdBase58.trimmed());
    payload.insert(QStringLiteral("decoded"), vaultHoldingToJson(decoded));
    return makeOkJson(payload);
}

QString decodeStreamConfigPayload(LogosAPIClient* client, const QString& accountIdBase58) {
    QString readError;
    const QByteArray data = accountDataBytesFromBase58(client, accountIdBase58, &readError);
    if (data.isEmpty()) {
        return makeErrorJson(readError.isEmpty() ? QStringLiteral("empty account data") : readError);
    }
    PsFfiDecodedStreamConfig decoded{};
    const uint32_t status = ps_ffi_decode_stream_config(
        reinterpret_cast<const uint8_t*>(data.constData()), static_cast<size_t>(data.size()), &decoded);
    if (status != 0u) {
        return makeErrorJson(ffiStatusMessage(status));
    }
    QJsonObject payload;
    payload.insert(QStringLiteral("account_id_base58"), accountIdBase58.trimmed());
    payload.insert(QStringLiteral("decoded"), streamConfigToJson(decoded));
    return makeOkJson(payload);
}

QString decodeClockPayload(LogosAPIClient* client, const QString& accountIdBase58) {
    QString readError;
    const QByteArray data = accountDataBytesFromBase58(client, accountIdBase58, &readError);
    if (data.isEmpty()) {
        return makeErrorJson(readError.isEmpty() ? QStringLiteral("empty account data") : readError);
    }
    PsFfiDecodedClock decoded{};
    const uint32_t status = ps_ffi_decode_clock(
        reinterpret_cast<const uint8_t*>(data.constData()), static_cast<size_t>(data.size()), &decoded);
    if (status != 0u) {
        return makeErrorJson(ffiStatusMessage(status));
    }
    QJsonObject payload;
    payload.insert(QStringLiteral("account_id_base58"), accountIdBase58.trimmed());
    payload.insert(QStringLiteral("decoded"), clockToJson(decoded));
    return makeOkJson(payload);
}

}  // namespace

QString PaymentStreamsModuleImpl::accountIdHexFromBase58(const QVariant& accountIdBase58) {
    LogosAPIClient* client = walletClientOrNull(modules().api);
    if (client == nullptr) {
        return makeErrorJson(QStringLiteral("logos_execution_zone client unavailable (load wallet first)"));
    }
    const QString base58 = accountIdBase58.toString();
    const QString hex = walletAccountIdHexFromBase58(client, base58);
    if (hex.isEmpty()) {
        return makeErrorJson(QStringLiteral("account_id_from_base58 returned empty"));
    }
    QJsonObject payload;
    payload.insert(QStringLiteral("account_id_hex"), hex);
    return makeOkJson(payload);
}

QString PaymentStreamsModuleImpl::readVaultConfigDecoded(const QVariant& vaultConfigAccountIdBase58) {
    LogosAPIClient* client = walletClientOrNull(modules().api);
    if (client == nullptr) {
        return makeErrorJson(QStringLiteral("logos_execution_zone client unavailable (load wallet first)"));
    }
    return decodeVaultConfigPayload(client, vaultConfigAccountIdBase58.toString());
}

QString PaymentStreamsModuleImpl::readVaultHoldingDecoded(const QVariant& vaultHoldingAccountIdBase58) {
    LogosAPIClient* client = walletClientOrNull(modules().api);
    if (client == nullptr) {
        return makeErrorJson(QStringLiteral("logos_execution_zone client unavailable (load wallet first)"));
    }
    return decodeVaultHoldingPayload(client, vaultHoldingAccountIdBase58.toString());
}

QString PaymentStreamsModuleImpl::readStreamConfigDecoded(const QVariant& streamConfigAccountIdBase58) {
    LogosAPIClient* client = walletClientOrNull(modules().api);
    if (client == nullptr) {
        return makeErrorJson(QStringLiteral("logos_execution_zone client unavailable (load wallet first)"));
    }
    return decodeStreamConfigPayload(client, streamConfigAccountIdBase58.toString());
}

QString PaymentStreamsModuleImpl::readClockDecoded(const QVariant& clockAccountIdBase58) {
    LogosAPIClient* client = walletClientOrNull(modules().api);
    if (client == nullptr) {
        return makeErrorJson(QStringLiteral("logos_execution_zone client unavailable (load wallet first)"));
    }
    return decodeClockPayload(client, clockAccountIdBase58.toString());
}

QString PaymentStreamsModuleImpl::readClock10Decoded() {
    return readClockDecoded(QString::fromUtf8(kDefaultClock10Base58));
}
