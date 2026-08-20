#pragma once

#include <QByteArray>
#include <QSet>
#include <QString>
#include <QStringList>

#include <cstdint>
#include <cstring>

// Header-only account-list helpers for writes.cpp and Qt unit tests.
// Do not pull Logos module deps into the test target.

namespace payment_streams_accounts {

constexpr int kAccountIdHexLen = 64;

enum class WithdrawDestClass : uint8_t {
    Omit = 0,
    Empty = 1,
    EqualOwner = 2,
    Distinct = 3,
};

inline WithdrawDestClass classifyWithdrawDest(bool keyPresent,
                                              bool presentEmpty,
                                              const uint8_t owner[32],
                                              const uint8_t withdrawTo[32]) {
    if (!keyPresent) {
        return WithdrawDestClass::Omit;
    }
    if (presentEmpty) {
        return WithdrawDestClass::Empty;
    }
    return accountIdsEqual32(owner, withdrawTo) ? WithdrawDestClass::EqualOwner
                                                : WithdrawDestClass::Distinct;
}

inline bool accountIdsEqual32(const uint8_t left[32], const uint8_t right[32]) {
    return std::memcmp(left, right, 32) == 0;
}

// Rejects length that is not a multiple of 64, then splits lowercase 64-hex ids.
// Duplicate ids (case-insensitive) return duplicate_account_ids.
inline bool parseUniqueAccountsHex(const QByteArray& accountsHex,
                                   QStringList* idsOut,
                                   QString* errorOut) {
    const int n = accountsHex.size();
    if (n % kAccountIdHexLen != 0) {
        if (errorOut != nullptr) {
            *errorOut = QStringLiteral("malformed_account_list");
        }
        return false;
    }
    QStringList ids;
    QSet<QString> seen;
    const QString all = QString::fromLatin1(accountsHex).toLower();
    for (int i = 0; i < n; i += kAccountIdHexLen) {
        const QString id = all.mid(i, kAccountIdHexLen);
        if (seen.contains(id)) {
            if (errorOut != nullptr) {
                *errorOut = QStringLiteral("duplicate_account_ids");
            }
            return false;
        }
        seen.insert(id);
        ids.append(id);
    }
    if (idsOut != nullptr) {
        *idsOut = ids;
    }
    return true;
}

}  // namespace payment_streams_accounts
