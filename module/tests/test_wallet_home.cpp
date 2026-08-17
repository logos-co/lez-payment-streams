#include <logos_test.h>

#include "payment_streams_module_kit.h"

#include <QDir>
#include <QFile>
#include <QTemporaryDir>

using payment_streams_kit::ensureWalletStatisticsFile;
using payment_streams_kit::resolveWalletHomePaths;
using payment_streams_kit::walletHomeFromEnv;
using payment_streams_kit::WalletHomePaths;

namespace {

class EnvRestore {
public:
    EnvRestore() {
        m_wallet = qgetenv("WALLET_HOME");
        m_lee = qgetenv("LEE_WALLET_HOME_DIR");
        m_nssa = qgetenv("NSSA_WALLET_HOME_DIR");
    }
    ~EnvRestore() {
        restore("WALLET_HOME", m_wallet);
        restore("LEE_WALLET_HOME_DIR", m_lee);
        restore("NSSA_WALLET_HOME_DIR", m_nssa);
    }

private:
    static void restore(const char* key, const QByteArray& previous) {
        if (previous.isEmpty()) {
            qunsetenv(key);
        } else {
            qputenv(key, previous);
        }
    }

    QByteArray m_wallet;
    QByteArray m_lee;
    QByteArray m_nssa;
};

bool writeText(const QString& path, const QByteArray& body) {
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
        return false;
    }
    file.write(body);
    return true;
}

}  // namespace

LOGOS_TEST(wallet_home_prefers_WALLET_HOME) {
    EnvRestore restore;
    qputenv("WALLET_HOME", "/tmp/ps-wallet-home");
    qputenv("LEE_WALLET_HOME_DIR", "/tmp/ps-lee-home");
    qputenv("NSSA_WALLET_HOME_DIR", "/tmp/ps-nssa-home");
    LOGOS_ASSERT_EQ(walletHomeFromEnv(), QStringLiteral("/tmp/ps-wallet-home"));
}

LOGOS_TEST(wallet_home_falls_back_to_LEE) {
    EnvRestore restore;
    qunsetenv("WALLET_HOME");
    qputenv("LEE_WALLET_HOME_DIR", "/tmp/ps-lee-home");
    qunsetenv("NSSA_WALLET_HOME_DIR");
    LOGOS_ASSERT_EQ(walletHomeFromEnv(), QStringLiteral("/tmp/ps-lee-home"));
}

LOGOS_TEST(wallet_home_empty_when_unset) {
    EnvRestore restore;
    qunsetenv("WALLET_HOME");
    qunsetenv("LEE_WALLET_HOME_DIR");
    qunsetenv("NSSA_WALLET_HOME_DIR");
    LOGOS_ASSERT_TRUE(walletHomeFromEnv().isEmpty());
}

LOGOS_TEST(resolve_wallet_home_requires_config_and_storage) {
    QTemporaryDir dir;
    LOGOS_ASSERT_TRUE(dir.isValid());
    QString err;
    WalletHomePaths paths;
    LOGOS_ASSERT_TRUE(!resolveWalletHomePaths(dir.path(), &paths, &err));
    LOGOS_ASSERT_TRUE(err.contains(QStringLiteral("wallet_config.json")));

    LOGOS_ASSERT_TRUE(writeText(dir.filePath(QStringLiteral("wallet_config.json")), "{}\n"));
    LOGOS_ASSERT_TRUE(!resolveWalletHomePaths(dir.path(), &paths, &err));
    LOGOS_ASSERT_TRUE(err.contains(QStringLiteral("storage.json")));

    LOGOS_ASSERT_TRUE(writeText(dir.filePath(QStringLiteral("storage.json")), "{}\n"));
    LOGOS_ASSERT_TRUE(resolveWalletHomePaths(dir.path(), &paths, &err));
    LOGOS_ASSERT_EQ(paths.config, dir.filePath(QStringLiteral("wallet_config.json")));
    LOGOS_ASSERT_EQ(paths.storage, dir.filePath(QStringLiteral("storage.json")));
    LOGOS_ASSERT_EQ(paths.statistics, dir.filePath(QStringLiteral("statistics.json")));
}

LOGOS_TEST(ensure_wallet_statistics_creates_empty_object) {
    QTemporaryDir dir;
    LOGOS_ASSERT_TRUE(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("statistics.json"));
    QString err;
    LOGOS_ASSERT_TRUE(ensureWalletStatisticsFile(path, &err));
    LOGOS_ASSERT_TRUE(QFile::exists(path));
    LOGOS_ASSERT_TRUE(ensureWalletStatisticsFile(path, &err));
}
