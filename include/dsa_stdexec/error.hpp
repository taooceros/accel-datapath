#pragma once
#ifndef DSA_STDEXEC_ERROR_HPP
#define DSA_STDEXEC_ERROR_HPP

#include <cstdint>
#include <exception>
#include <stacktrace>
#include <string>
#include <string_view>

#include <fmt/format.h>

extern "C" {
#include <linux/idxd.h>
}

namespace dsa_stdexec {

// Convert DSA completion status code to human-readable string
constexpr std::string_view dsa_status_to_string(uint8_t status) noexcept {
  switch (status) {
  case DSA_COMP_NONE:
    return "No status (operation not complete)";
  case DSA_COMP_SUCCESS:
    return "Success";
  case DSA_COMP_SUCCESS_PRED:
    return "Success with predicate";
  case DSA_COMP_PAGE_FAULT_NOBOF:
    return "Page fault without block-on-fault";
  case DSA_COMP_PAGE_FAULT_IR:
    return "Page fault with interrupt request";
  case DSA_COMP_BATCH_FAIL:
    return "Batch operation failed";
  case DSA_COMP_BATCH_PAGE_FAULT:
    return "Batch operation page fault";
  case DSA_COMP_DR_OFFSET_NOINC:
    return "Delta record offset not incrementing";
  case DSA_COMP_DR_OFFSET_ERANGE:
    return "Delta record offset out of range";
  case DSA_COMP_DIF_ERR:
    return "DIF error";
  case DSA_COMP_BAD_OPCODE:
    return "Invalid opcode";
  case DSA_COMP_INVALID_FLAGS:
    return "Invalid flags";
  case DSA_COMP_NOZERO_RESERVE:
    return "Non-zero reserved field";
  case DSA_COMP_XFER_ERANGE:
    return "Transfer size out of range";
  case DSA_COMP_DESC_CNT_ERANGE:
    return "Descriptor count out of range";
  case DSA_COMP_DR_ERANGE:
    return "Delta record size out of range";
  case DSA_COMP_OVERLAP_BUFFERS:
    return "Overlapping buffers";
  case DSA_COMP_DCAST_ERR:
    return "Dualcast error";
  case DSA_COMP_DESCLIST_ALIGN:
    return "Descriptor list alignment error";
  case DSA_COMP_INT_HANDLE_INVAL:
    return "Invalid interrupt handle";
  case DSA_COMP_CRA_XLAT:
    return "Completion record address translation error";
  case DSA_COMP_CRA_ALIGN:
    return "Completion record address alignment error";
  case DSA_COMP_ADDR_ALIGN:
    return "Address alignment error";
  case DSA_COMP_PRIV_BAD:
    return "Privilege error";
  case DSA_COMP_TRAFFIC_CLASS_CONF:
    return "Traffic class configuration error";
  case DSA_COMP_PFAULT_RDBA:
    return "Page fault on readback address";
  case DSA_COMP_HW_ERR1:
    return "Hardware error 1";
  case DSA_COMP_HW_ERR_DRB:
    return "Hardware error (DRB)";
  case DSA_COMP_TRANSLATION_FAIL:
    return "Address translation failure";
  case DSA_COMP_DRAIN_EVL:
    return "Drain event log";
  case DSA_COMP_BATCH_EVL_ERR:
    return "Batch event log error";
  default:
    return "Unknown error";
  }


}

// Convert DSA opcode to human-readable string
constexpr std::string_view dsa_opcode_to_string(uint8_t opcode) noexcept {
  switch (opcode) {
  case DSA_OPCODE_NOOP:
    return "NOOP";
  case DSA_OPCODE_BATCH:
    return "BATCH";
  case DSA_OPCODE_DRAIN:
    return "DRAIN";
  case DSA_OPCODE_MEMMOVE:
    return "MEMMOVE";
  case DSA_OPCODE_MEMFILL:
    return "MEMFILL";
  case DSA_OPCODE_COMPARE:
    return "COMPARE";
  case DSA_OPCODE_COMPVAL:
    return "COMPVAL";
  case DSA_OPCODE_CR_DELTA:
    return "CR_DELTA";
  case DSA_OPCODE_AP_DELTA:
    return "AP_DELTA";
  case DSA_OPCODE_DUALCAST:
    return "DUALCAST";
  case DSA_OPCODE_TRANSL_FETCH:
    return "TRANSL_FETCH";
  case DSA_OPCODE_CRCGEN:
    return "CRCGEN";
  case DSA_OPCODE_COPY_CRC:
    return "COPY_CRC";
  case DSA_OPCODE_DIF_CHECK:
    return "DIF_CHECK";
  case DSA_OPCODE_DIF_INS:
    return "DIF_INS";
  case DSA_OPCODE_DIF_STRP:
    return "DIF_STRP";
  case DSA_OPCODE_DIF_UPDT:
    return "DIF_UPDT";
  case DSA_OPCODE_DIX_GEN:
    return "DIX_GEN";
  case DSA_OPCODE_CFLUSH:
    return "CFLUSH";
  default:
    return "UNKNOWN";
  }
}

// DSA-specific exception with detailed error information and stacktrace
class DsaError : public std::exception {
public:
  // Constructor for completion record errors
  DsaError(uint8_t status, const dsa_completion_record &comp,
           uint8_t opcode = 0, std::string_view context = "")
      : status_(status), opcode_(opcode), bytes_completed_(comp.bytes_completed),
        fault_addr_(comp.fault_addr), fault_info_(comp.fault_info),
        stacktrace_(std::stacktrace::current()) {
    build_message(context);
  }

  // Constructor for general DSA errors (e.g., submission failures)
  explicit DsaError(std::string_view message)
      : status_(0), opcode_(0), bytes_completed_(0), fault_addr_(0),
        fault_info_(0), stacktrace_(std::stacktrace::current()) {
    message_ = fmt::format("DSA error: {}", message);
  }

  // Constructor with status code only
  explicit DsaError(uint8_t status, std::string_view context = "")
      : status_(status), opcode_(0), bytes_completed_(0), fault_addr_(0),
        fault_info_(0), stacktrace_(std::stacktrace::current()) {
    build_message(context);
  }

  const char *what() const noexcept override { return message_.c_str(); }

  // Accessors
  uint8_t status() const noexcept { return status_; }
  uint8_t opcode() const noexcept { return opcode_; }
  uint32_t bytes_completed() const noexcept { return bytes_completed_; }
  uint64_t fault_addr() const noexcept { return fault_addr_; }
  uint8_t fault_info() const noexcept { return fault_info_; }

  std::string_view status_string() const noexcept {
    return dsa_status_to_string(status_);
  }

  std::string_view opcode_string() const noexcept {
    return dsa_opcode_to_string(opcode_);
  }

  const std::stacktrace &stacktrace() const noexcept {
    return stacktrace_;
  }

  // Get full error report including stacktrace
  std::string full_report() const {
    std::string report = message_;
    report += "Stacktrace:";
    report += std::to_string(stacktrace_);
    return report;
  }

private:
  void build_message(std::string_view context) {
    if (context.empty()) {
      message_ = fmt::format(
          "DSA operation failed: {} (status=0x{:02x}, opcode={}, "
          "bytes_completed={}, fault_addr=0x{:016x}, fault_info=0x{:02x})",
          dsa_status_to_string(status_), status_, dsa_opcode_to_string(opcode_),
          bytes_completed_, fault_addr_, fault_info_);
    } else {
      message_ = fmt::format(
          "DSA operation failed [{}]: {} (status=0x{:02x}, opcode={}, "
          "bytes_completed={}, fault_addr=0x{:016x}, fault_info=0x{:02x})",
          context, dsa_status_to_string(status_), status_,
          dsa_opcode_to_string(opcode_), bytes_completed_, fault_addr_,
          fault_info_);
    }
  }

  uint8_t status_;
  uint8_t opcode_;
  uint32_t bytes_completed_;
  uint64_t fault_addr_;
  uint8_t fault_info_;
  std::string message_;
  std::stacktrace stacktrace_;
};

// Exception for DSA submission errors
class DsaSubmitError : public DsaError {
public:
  explicit DsaSubmitError(std::string_view reason)
      : DsaError(fmt::format("Failed to submit DSA descriptor: {}", reason)) {}

  DsaSubmitError(std::string_view reason, int error_code)
      : DsaError(fmt::format("Failed to submit DSA descriptor: {} (errno={})",
                             reason, error_code)),
        error_code_(error_code) {}

  int error_code() const noexcept { return error_code_; }

private:
  int error_code_ = 0;
};

// Exception for DSA initialization errors
class DsaInitError : public DsaError {
public:
  explicit DsaInitError(std::string_view reason)
      : DsaError(fmt::format("DSA initialization failed: {}", reason)) {}

  DsaInitError(std::string_view reason, int error_code)
      : DsaError(fmt::format("DSA initialization failed: {} (errno={})", reason,
                             error_code)),
        error_code_(error_code) {}

  int error_code() const noexcept { return error_code_; }

private:
  int error_code_ = 0;
};

} // namespace dsa_stdexec

// fmt formatter for DsaError
template <> struct fmt::formatter<dsa_stdexec::DsaError> {
  constexpr auto parse(format_parse_context &ctx) { return ctx.begin(); }

  template <typename FormatContext>
  auto format(const dsa_stdexec::DsaError &err, FormatContext &ctx) const {
    return fmt::format_to(ctx.out(), "{}", err.what());
  }
};

#endif
