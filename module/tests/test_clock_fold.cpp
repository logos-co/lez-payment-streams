#include <logos_test.h>

#include "payment_streams_module_kit.h"

using payment_streams_kit::chainTimestampToFoldSeconds;
using payment_streams_kit::kMsEpochThreshold;

LOGOS_TEST(clock_fold_seconds_below_threshold_unchanged) {
    LOGOS_ASSERT_TRUE(chainTimestampToFoldSeconds(1'000'000'000ULL - 1) == 1'000'000'000ULL - 1);
    LOGOS_ASSERT_TRUE(chainTimestampToFoldSeconds(kMsEpochThreshold - 1) == kMsEpochThreshold - 1);
}

LOGOS_TEST(clock_fold_milliseconds_at_threshold) {
    LOGOS_ASSERT_TRUE(chainTimestampToFoldSeconds(kMsEpochThreshold) == kMsEpochThreshold / 1000);
}

LOGOS_TEST(clock_fold_milliseconds_above_threshold) {
    LOGOS_ASSERT_TRUE(chainTimestampToFoldSeconds(kMsEpochThreshold + 1500) ==
                      (kMsEpochThreshold + 1500) / 1000);
}
