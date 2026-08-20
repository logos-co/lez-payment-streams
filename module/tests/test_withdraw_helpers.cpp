#include <logos_test.h>

#include "payment_streams_account_lists.h"
#include "payment_streams_module_kit.h"

#include <QByteArray>
#include <QString>
#include <QStringList>

#include <cstring>

using payment_streams_accounts::classifyWithdrawDest;
using payment_streams_accounts::kAccountIdHexLen;
using payment_streams_accounts::parseUniqueAccountsHex;
using payment_streams_accounts::WithdrawDestClass;
using payment_streams_kit::accountIdBytesFromField;
using payment_streams_kit::withdrawUiAmountFromHoldingAndAllocated;
using payment_streams_kit::withdrawUiAmountFromUnallocated;

namespace {

QString hexId(int n, bool upper = false) {
    QString id = QStringLiteral("%1").arg(n, kAccountIdHexLen, 16, QLatin1Char('0'));
    return upper ? id.toUpper() : id;
}

QByteArray concatIds(const QStringList& ids) {
    QByteArray out;
    for (const QString& id : ids) {
        out += id.toLatin1();
    }
    return out;
}

bool parseOkUnique(const QByteArray& hex, int expectedCount) {
    QStringList ids;
    QString err;
    if (!parseUniqueAccountsHex(hex, &ids, &err)) {
        return false;
    }
    return err.isEmpty() && ids.size() == expectedCount;
}

void fillOwnerBytes(uint8_t out[32], quint8 fill) {
    std::memset(out, fill, 32);
}

}  // namespace

LOGOS_TEST(unique_account_lists_of_live_planner_widths) {
    for (int count : {3, 4, 5, 6}) {
        QStringList ids;
        for (int i = 0; i < count; ++i) {
            ids.append(hexId(i + 1));
        }
        LOGOS_ASSERT_TRUE(parseOkUnique(concatIds(ids), count));
    }
}

LOGOS_TEST(duplicate_account_lists_of_live_planner_widths) {
    for (int count : {3, 4, 5, 6}) {
        QStringList ids;
        for (int i = 0; i < count; ++i) {
            ids.append(hexId(i + 1));
        }
        ids[count - 1] = ids[0];
        QStringList parsed;
        QString err;
        LOGOS_ASSERT_FALSE(parseUniqueAccountsHex(concatIds(ids), &parsed, &err));
        LOGOS_ASSERT_EQ(err, QStringLiteral("duplicate_account_ids"));
    }
}

LOGOS_TEST(malformed_account_list_rejects_64k_plus_r) {
    const QByteArray hex(kAccountIdHexLen * 2 + 1, 'a');
    QStringList ids;
    QString err;
    LOGOS_ASSERT_FALSE(parseUniqueAccountsHex(hex, &ids, &err));
    LOGOS_ASSERT_EQ(err, QStringLiteral("malformed_account_list"));
}

LOGOS_TEST(mixed_case_unique_account_list_is_accepted) {
    const QStringList ids{hexId(1, true), hexId(2, false), hexId(3, true)};
    QStringList parsed;
    QString err;
    LOGOS_ASSERT_TRUE(parseUniqueAccountsHex(concatIds(ids), &parsed, &err));
    LOGOS_ASSERT_EQ(parsed.size(), 3);
    LOGOS_ASSERT_EQ(parsed.at(0), hexId(1, false));
}

LOGOS_TEST(mixed_case_same_id_is_duplicate) {
    const QStringList ids{hexId(7, true), hexId(7, false), hexId(8)};
    QStringList parsed;
    QString err;
    LOGOS_ASSERT_FALSE(parseUniqueAccountsHex(concatIds(ids), &parsed, &err));
    LOGOS_ASSERT_EQ(err, QStringLiteral("duplicate_account_ids"));
}

LOGOS_TEST(classify_omit_empty_equal_and_distinct) {
    uint8_t owner[32]{};
    uint8_t other[32]{};
    fillOwnerBytes(owner, 0xaa);
    fillOwnerBytes(other, 0xbb);

    LOGOS_ASSERT_EQ(static_cast<int>(classifyWithdrawDest(false, false, owner, other)),
                    static_cast<int>(WithdrawDestClass::Omit));
    LOGOS_ASSERT_EQ(static_cast<int>(classifyWithdrawDest(true, true, owner, other)),
                    static_cast<int>(WithdrawDestClass::Empty));
    LOGOS_ASSERT_EQ(static_cast<int>(classifyWithdrawDest(true, false, owner, owner)),
                    static_cast<int>(WithdrawDestClass::EqualOwner));
    LOGOS_ASSERT_EQ(static_cast<int>(classifyWithdrawDest(true, false, owner, other)),
                    static_cast<int>(WithdrawDestClass::Distinct));
}

LOGOS_TEST(classify_equal_owner_hex_and_base58_encodings) {
    const QString ownerHex = hexId(0x11);
    const QString ownerB58 = QStringLiteral("OwnerSameIdAsHexField");
    const auto decode = [&](const QString& field) {
        if (field == ownerB58) {
            return ownerHex;
        }
        return QString();
    };

    uint8_t fromHex[32]{};
    uint8_t fromB58[32]{};
    QString err;
    LOGOS_ASSERT_TRUE(accountIdBytesFromField(ownerHex, fromHex, decode, &err));
    LOGOS_ASSERT_TRUE(accountIdBytesFromField(ownerB58, fromB58, decode, &err));
    LOGOS_ASSERT_EQ(static_cast<int>(classifyWithdrawDest(true, false, fromB58, fromHex)),
                    static_cast<int>(WithdrawDestClass::EqualOwner));
    LOGOS_ASSERT_EQ(static_cast<int>(classifyWithdrawDest(true, false, fromHex, fromB58)),
                    static_cast<int>(WithdrawDestClass::EqualOwner));
}

LOGOS_TEST(withdraw_ui_amount_zero_is_disabled) {
    const auto a = withdrawUiAmountFromUnallocated(0, 0);
    LOGOS_ASSERT_FALSE(a.enabled);
    LOGOS_ASSERT_EQ(a.amountLo, static_cast<quint64>(0));
}

LOGOS_TEST(withdraw_ui_amount_two_to_the_fifty_three_fits) {
    const quint64 two53 = 1ULL << 53;
    const auto a = withdrawUiAmountFromUnallocated(two53, 0);
    LOGOS_ASSERT_TRUE(a.enabled);
    LOGOS_ASSERT_EQ(a.amountLo, two53);
}

LOGOS_TEST(withdraw_ui_amount_u64_max_fits) {
    const quint64 maxU64 = ~quint64{0};
    const auto a = withdrawUiAmountFromUnallocated(maxU64, 0);
    LOGOS_ASSERT_TRUE(a.enabled);
    LOGOS_ASSERT_EQ(a.amountLo, maxU64);
}

LOGOS_TEST(withdraw_ui_amount_nonzero_high_limb_is_disabled) {
    const auto a = withdrawUiAmountFromUnallocated(1, 1);
    LOGOS_ASSERT_FALSE(a.enabled);
}

LOGOS_TEST(withdraw_ui_amount_from_holding_minus_allocated) {
    const auto leftover = withdrawUiAmountFromHoldingAndAllocated(500, 0, 400, 0);
    LOGOS_ASSERT_TRUE(leftover.enabled);
    LOGOS_ASSERT_EQ(leftover.amountLo, static_cast<quint64>(100));

    const auto none = withdrawUiAmountFromHoldingAndAllocated(400, 0, 400, 0);
    LOGOS_ASSERT_FALSE(none.enabled);
}
