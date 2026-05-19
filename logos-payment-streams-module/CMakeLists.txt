cmake_minimum_required(VERSION 3.14)
project(LogosPaymentStreamsModule LANGUAGES CXX)

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
    src/i_payment_streams_module.h
    src/payment_streams_module_plugin.h
    src/payment_streams_module_plugin.cpp
  EXTERNAL_LIBS
    lez_payment_streams_ffi
)
