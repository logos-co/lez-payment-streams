cmake_minimum_required(VERSION 3.14)
project(PaymentStreamsModuleTests LANGUAGES CXX)

include(LogosTest)

logos_test(
    NAME payment_streams_module_tests
    MODULE_SOURCES
        ../src/payment_streams_privacy_policy.cpp
    TEST_SOURCES
        main.cpp
        test_privacy_submit_policy.cpp
    EXTRA_INCLUDES
        ../src
)

set_target_properties(payment_streams_module_tests PROPERTIES CXX_STANDARD 20)
