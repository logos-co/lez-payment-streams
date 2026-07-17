cmake_minimum_required(VERSION 3.14)
project(LogosPaymentStreamsModule LANGUAGES CXX C)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(DEFINED ENV{LOGOS_MODULE_BUILDER_ROOT})
  include($ENV{LOGOS_MODULE_BUILDER_ROOT}/cmake/LogosModule.cmake)
else()
  message(FATAL_ERROR "LogosModule.cmake not found. Set LOGOS_MODULE_BUILDER_ROOT.")
endif()

logos_module(
  NAME payment_streams_module
    SOURCES
    src/payment_streams_module_impl.h
    src/payment_streams_module_impl.cpp
    src/payment_streams_module_writes.cpp
    src/payment_streams_privacy_policy.h
    src/payment_streams_privacy_policy.cpp
    src/payment_streams_ffi_bridge.c
    src/payment_streams_ffi_bridge_writes.c
    src/payment_streams_module_eligibility.cpp
  EXTERNAL_LIBS
    lez_payment_streams_ffi
)

set_source_files_properties(src/payment_streams_ffi_bridge.c src/payment_streams_ffi_bridge_writes.c PROPERTIES LANGUAGE C)
