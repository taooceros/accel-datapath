use idxd_rust::{
    CompletionAction, CompletionSnapshot, DEFAULT_DEVICE_PATH, DEFAULT_MAX_PAGE_FAULT_RETRIES,
    DsaSession, MemmoveError, MemmovePhase, MemmoveRequest, MemmoveRetry,
    MemmoveValidationConfig, MemmoveValidationReport, classify_memmove_completion,
};
use idxd_sys::{DsaCompletionRecord, DsaHwDesc};
use std::mem::{align_of, size_of};

fn test_config() -> MemmoveValidationConfig {
    MemmoveValidationConfig::builder()
        .device_path("/dev/dsa/wq0.0")
        .max_page_fault_retries(1)
        .build()
        .expect("test config")
}

#[test]
fn descriptor_helpers_are_aligned_over_generated_uapi_records() {
    assert_eq!(
        size_of::<DsaHwDesc>(),
        size_of::<idxd_sys::idxd_uapi::dsa_hw_desc>()
    );
    assert_eq!(align_of::<DsaHwDesc>(), 64);
    assert_eq!(
        size_of::<DsaCompletionRecord>(),
        size_of::<idxd_sys::idxd_uapi::dsa_completion_record>()
    );
    assert_eq!(align_of::<DsaCompletionRecord>(), 32);

    let src = [0u8; 8];
    let mut dst = [0u8; 8];
    let mut desc = DsaHwDesc::default();
    let mut comp = DsaCompletionRecord::default();
    desc.fill_memmove(src.as_ptr(), dst.as_mut_ptr(), src.len() as u32);
    desc.set_completion(&mut comp);
}

#[test]
fn exposes_stable_validation_config_fields() {
    let config = MemmoveValidationConfig::with_retries("/dev/dsa/wq1.2", 3)
        .expect("non-empty device paths should be accepted");

    assert_eq!(config.device_path().to_str(), Some("/dev/dsa/wq1.2"));
    assert_eq!(config.max_page_fault_retries(), 3);
}

#[test]
fn builder_uses_default_device_path_and_retry_budget() {
    let config = MemmoveValidationConfig::builder()
        .build()
        .expect("default builder config should validate");

    assert_eq!(config.device_path().to_str(), Some(DEFAULT_DEVICE_PATH));
    assert_eq!(
        config.max_page_fault_retries(),
        DEFAULT_MAX_PAGE_FAULT_RETRIES
    );
}

#[test]
fn builder_accepts_explicit_device_path_and_retry_budget() {
    let config = MemmoveValidationConfig::builder()
        .device_path("/dev/dsa/wq1.2")
        .max_page_fault_retries(3)
        .build()
        .expect("explicit builder config should validate");

    assert_eq!(config.device_path().to_str(), Some("/dev/dsa/wq1.2"));
    assert_eq!(config.max_page_fault_retries(), 3);
}

#[test]
fn builder_preserves_zero_retry_budget() {
    let config = MemmoveValidationConfig::builder()
        .device_path("/dev/dsa/wq1.2")
        .max_page_fault_retries(0)
        .build()
        .expect("zero retries is a valid explicit budget");

    assert_eq!(config.max_page_fault_retries(), 0);
}

#[test]
fn legacy_validation_config_constructors_still_match_builder_behavior() {
    let default_retry = MemmoveValidationConfig::new("/dev/dsa/wq2.0")
        .expect("legacy constructor should keep default retry behavior");
    let explicit_retry = MemmoveValidationConfig::with_retries("/dev/dsa/wq2.0", 7)
        .expect("legacy retry constructor should remain compatible");

    assert_eq!(default_retry.device_path().to_str(), Some("/dev/dsa/wq2.0"));
    assert_eq!(
        default_retry.max_page_fault_retries(),
        DEFAULT_MAX_PAGE_FAULT_RETRIES
    );
    assert_eq!(explicit_retry.device_path().to_str(), Some("/dev/dsa/wq2.0"));
    assert_eq!(explicit_retry.max_page_fault_retries(), 7);
}

#[test]
fn builder_rejects_empty_device_path_before_queue_open() {
    let err = MemmoveValidationConfig::builder()
        .device_path("")
        .build()
        .expect_err("empty builder device paths should fail validation");

    assert!(matches!(err, MemmoveError::InvalidDevicePath { .. }));
}

#[test]
fn rejects_zero_length_requests() {
    let err = MemmoveRequest::new(0).expect_err("zero-length requests should fail");
    assert!(matches!(
        err,
        MemmoveError::InvalidLength {
            requested_len: 0,
            ..
        }
    ));
}

#[test]
fn accepts_tiny_and_frame_sized_requests() {
    assert_eq!(MemmoveRequest::new(1).unwrap().len(), 1);
    assert_eq!(MemmoveRequest::new(4096).unwrap().len(), 4096);
}

#[test]
fn rejects_oversized_requests() {
    let err = MemmoveRequest::new((u32::MAX as usize) + 1)
        .expect_err("transfer sizes beyond the descriptor limit should fail");
    assert!(matches!(
        err,
        MemmoveError::InvalidLength {
            requested_len,
            max_len,
        } if requested_len == (u32::MAX as usize) + 1 && max_len == u32::MAX as usize
    ));
}

#[test]
fn rejects_destination_too_small_before_queue_open() {
    let err = MemmoveRequest::for_buffers(255, 256)
        .expect_err("destination preconditions should fail before queue-open");
    assert!(matches!(
        err,
        MemmoveError::DestinationTooSmall {
            src_len: 256,
            dst_len: 255,
        }
    ));
}

#[test]
fn accepts_destination_capacity_larger_than_requested_source_length() {
    let request = MemmoveRequest::for_buffers(4096, 1024)
        .expect("destination capacity may exceed the requested source length");

    assert_eq!(request.len(), 1024);
    assert!(!request.is_empty());
}

#[test]
fn validation_report_records_requested_bytes_not_destination_capacity() {
    let request = MemmoveRequest::for_buffers(16, 4)
        .expect("oversized destinations should still validate against source length");
    let report = MemmoveValidationReport::new("/dev/dsa/wq0.1", request, 0, 1)
        .expect("valid report inputs should build");

    assert_eq!(report.requested_bytes, 4);
}

#[test]
fn rejects_empty_device_path_before_queue_open() {
    let err = DsaSession::open("")
        .err()
        .expect("empty device paths should fail validation");
    assert!(matches!(err, MemmoveError::InvalidDevicePath { .. }));
}

#[test]
fn builds_validation_report_with_requested_bytes_and_retry_metadata() {
    let request = MemmoveRequest::new(4096).expect("request should validate");
    let report = MemmoveValidationReport::new("/dev/dsa/wq0.1", request, 2, 1)
        .expect("valid report inputs should build");

    assert_eq!(report.device_path.to_str(), Some("/dev/dsa/wq0.1"));
    assert_eq!(report.requested_bytes, 4096);
    assert_eq!(report.page_fault_retries, 2);
    assert_eq!(report.final_status, 1);
}

#[test]
fn classifies_success_completion() {
    let action =
        classify_memmove_completion(&test_config(), 1024, CompletionSnapshot::new(1, 0, 0, 0), 0)
            .expect("success status should pass");

    assert_eq!(action, CompletionAction::Success);
}

#[test]
fn advances_offsets_for_forward_page_fault_retry() {
    let action = classify_memmove_completion(
        &test_config(),
        1024,
        CompletionSnapshot::new(3, 0, 128, 0xdead_beef),
        0,
    )
    .expect("recoverable page fault should request a retry");

    assert_eq!(
        action,
        CompletionAction::Retry(MemmoveRetry {
            next_src_offset: 128,
            next_dst_offset: 128,
            remaining_bytes: 896,
        })
    );
}

#[test]
fn preserves_offsets_for_reverse_copy_page_fault_retry() {
    let action = classify_memmove_completion(
        &test_config(),
        1024,
        CompletionSnapshot::new(3, 1, 128, 0xdead_beef),
        0,
    )
    .expect("reverse-copy page fault should keep original base pointers");

    assert_eq!(
        action,
        CompletionAction::Retry(MemmoveRetry {
            next_src_offset: 0,
            next_dst_offset: 0,
            remaining_bytes: 896,
        })
    );
}

#[test]
fn rejects_page_fault_without_fault_address() {
    let err = classify_memmove_completion(
        &test_config(),
        1024,
        CompletionSnapshot::new(3, 0, 128, 0),
        0,
    )
    .expect_err("page fault without address should be rejected");

    assert!(matches!(
        err,
        MemmoveError::MalformedCompletion {
            phase: MemmovePhase::PageFaultRetry,
            page_fault_retries: 0,
            ..
        }
    ));
}

#[test]
fn rejects_page_fault_with_invalid_direction_bit() {
    let err = classify_memmove_completion(
        &test_config(),
        1024,
        CompletionSnapshot::new(3, 2, 128, 0xdead_beef),
        0,
    )
    .expect_err("invalid page-fault direction should be rejected as malformed");

    assert!(matches!(
        err,
        MemmoveError::MalformedCompletion {
            phase: MemmovePhase::PageFaultRetry,
            status: 3,
            result: 2,
            page_fault_retries: 0,
            ..
        }
    ));
}

#[test]
fn rejects_page_fault_when_retry_budget_is_exhausted() {
    let err = classify_memmove_completion(
        &test_config(),
        1024,
        CompletionSnapshot::new(3, 0, 128, 0xdead_beef),
        1,
    )
    .expect_err("retry exhaustion should surface explicitly");

    assert!(matches!(
        err,
        MemmoveError::PageFaultRetryExhausted {
            phase: MemmovePhase::PageFaultRetry,
            retries: 1,
            ..
        }
    ));
}

#[test]
fn maps_poll_timeout_to_completion_timeout() {
    let err = classify_memmove_completion(
        &test_config(),
        1024,
        CompletionSnapshot::new(0xff, 0, 0, 0),
        1,
    )
    .expect_err("timeout sentinel should map to a typed timeout");

    assert!(matches!(
        err,
        MemmoveError::CompletionTimeout {
            phase: MemmovePhase::CompletionPoll,
            page_fault_retries: 1,
            ..
        }
    ));
}

#[test]
fn surfaces_non_success_completion_statuses() {
    let err = classify_memmove_completion(
        &test_config(),
        1024,
        CompletionSnapshot::new(0x12, 0, 0, 0),
        2,
    )
    .expect_err("non-success statuses should surface directly");

    assert!(matches!(
        err,
        MemmoveError::CompletionStatus {
            phase: MemmovePhase::CompletionPoll,
            status: 0x12,
            page_fault_retries: 2,
            ..
        }
    ));
}

#[test]
fn error_accessors_preserve_phase_path_and_retry_context() {
    let err = classify_memmove_completion(
        &test_config(),
        1024,
        CompletionSnapshot::new(0x12, 0, 0, 0),
        2,
    )
    .expect_err("non-success statuses should surface directly");

    assert_eq!(
        err.device_path().and_then(|path| path.to_str()),
        Some("/dev/dsa/wq0.0")
    );
    assert_eq!(err.phase(), Some(MemmovePhase::CompletionPoll));
    assert_eq!(err.page_fault_retries(), Some(2));
}
