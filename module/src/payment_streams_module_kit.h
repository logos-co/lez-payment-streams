#pragma once

#include <QByteArray>
#include <QJsonObject>
#include <QString>
#include <QVariant>

#include <cstddef>
#include <cstdint>
#include <functional>

// Shared module helpers. Named namespace so one definition is linked across
// translation units (D50.3).

namespace payment_streams_kit {

// Unix-epoch millisecond threshold used by
// lez_payment_streams_core::chain_timestamp_to_fold_seconds.
constexpr quint64 kMsEpochThreshold = 1'000'000'000'000ULL;
constexpr int kAccountIdHexLen = 64;
constexpr uint32_t kFfiSuccess = 0u;
// Private submit LogosAPIClient Timeout. 20s is too short for stub or real prove.
constexpr int kPrivateSubmitTimeoutMs = 1800000;

quint64 chainTimestampToFoldSeconds(quint64 ts);

QJsonObject mergePersistedDiskKeys(const QJsonObject& memory, const QJsonObject& disk);

QString makeErrorJson(const QString& message);
QString makeErrorJson(const QString& message, const QJsonObject& extra);
QString makeOkJson(const QJsonObject& payload);
QString makePlainError(const QString& message);
QString makeEligibilityError(const QString& code, const QString& message);
QString makeVerifyEligibilityError(const QString& eligibility, const QString& message);

bool parseWalletAccountJson(const QString& accountJson,
                            QByteArray* dataOut,
                            QString* errorOut = nullptr,
                            QString* balanceHexOut = nullptr);

bool hex32FromQString(const QString& hexIn, uint8_t out[32]);
quint64 variantToU64(const QVariant& value, bool* okOut);

QString resolveRepoRelativePath(const QString& path);
bool findRepoFile(const QString& relativePath, QString* absoluteOut);
QString fixtureManifestPath();
bool loadFixtureManifest(QJsonObject* out, QString* errorOut);

// Wallet home for Basecamp logos_host: WALLET_HOME, else LEE_WALLET_HOME_DIR,
// else NSSA_WALLET_HOME_DIR.
struct WalletHomePaths {
    QString home;
    QString config;
    QString storage;
    QString statistics;
};

QString walletHomeFromEnv();
bool resolveWalletHomePaths(const QString& home, WalletHomePaths* out, QString* errorOut);
bool ensureWalletStatisticsFile(const QString& statisticsPath, QString* errorOut);

bool ffiBufferTwoPhase(const std::function<uint32_t(uint8_t*, size_t, size_t*)>& call,
                       QByteArray* out,
                       QString* errorOut,
                       const QString& sizeFail = QString(),
                       const QString& writeFail = QString());

bool isAccountIdHex64(const QString& trimmed);
QString accountIdHexFromField(const QString& field,
                              const std::function<QString(const QString&)>& base58ToHex,
                              QString* errorOut);
bool accountIdBytesFromField(const QString& field,
                             uint8_t out[32],
                             const std::function<QString(const QString&)>& base58ToHex,
                             QString* errorOut);

template <typename Wallet>
QString walletAccountIdHexFromBase58(Wallet& wallet, const QString& accountIdBase58) {
    const QString trimmed = accountIdBase58.trimmed();
    if (trimmed.isEmpty()) {
        return {};
    }
    return QString::fromStdString(wallet.account_id_from_base58(trimmed.toStdString()));
}

}  // namespace payment_streams_kit
