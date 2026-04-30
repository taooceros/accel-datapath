use idxd_rust::{
    CompletionAction, CompletionSnapshot, DEFAULT_DEVICE_PATH, DEFAULT_MAX_PAGE_FAULT_RETRIES,
    DsaConfig, DsaSession, MemmoveError, MemmovePhase, MemmoveRequest, MemmoveRetry,
    MemmoveValidationReport, classify_memmove_completion,
};
use idxd_sys::{DsaCompletionRecord, DsaHwDesc};
use std::error::Error as StdError;
use std::mem::{align_of, size_of};

fn test_config() -> DsaConfig {
    DsaConfig::builder()
        .device_path(std::path::PathBuf::from("/dev/dsa/wq0.0"))
        .max_page_fault_retries(1)
        .build()
        .expect("test config")
}

fn assert_display_excludes_payload_bytes(message: &str) {
    for forbidden in [
        "[1, 2, 3, 4]",
        "source_bytes",
        "destination_bytes",
        "payload",
    ] {
        assert!(
            !message.contains(forbidden),
            "display leaked forbidden payload marker {forbidden:?}: {message}"
        );
    }
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
fn builder_uses_default_device_path_and_page_fault_retries() {
    let config = DsaConfig::builder()
        .build()
        .expect("default builder config should validate");

    assert_eq!(config.device_path().to_str(), Some(DEFAULT_DEVICE_PATH));
    assert_eq!(
        config.max_page_fault_retries(),
        DEFAULT_MAX_PAGE_FAULT_RETRIES
    );
}

#[test]
fn builder_accepts_explicit_device_path_and_page_fault_retries() {
    let config = DsaConfig::builder()
        .device_path(std::path::PathBuf::from("/dev/dsa/wq1.2"))
        .max_page_fault_retries(3)
        .build()
        .expect("explicit builder config should validate");

    assert_eq!(config.device_path().to_str(), Some("/dev/dsa/wq1.2"));
    assert_eq!(config.max_page_fault_retries(), 3);
}

#[test]
fn builder_preserves_zero_page_fault_retries() {
    let config = DsaConfig::builder()
        .device_path(std::path::PathBuf::from("/dev/dsa/wq1.2"))
        .max_page_fault_retries(0)
        .build()
        .expect("zero retries is a valid explicit budget");

    assert_eq!(config.max_page_fault_retries(), 0);
}

#[test]
fn builder_rejects_empty_device_path_before_queue_open() {
    let err = DsaConfig::builder()
        .device_path(std::path::PathBuf::from(""))
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
fn session_builder_rejects_empty_device_path_before_queue_open() {
    let err = DsaConfig::builder()
        .device_path(std::path::PathBuf::from(""))
        .build()
        .and_then(DsaSession::open_config)
        .err()
        .expect("empty builder/config device paths should fail validation");

    assert!(matches!(err, MemmoveError::InvalidDevicePath { .. }));
}

#[test]
fn session_builder_preserves_explicit_config_on_queue_open_failure() {
    let config = DsaConfig::builder()
        .device_path(std::path::PathBuf::from(
            "/dev/dsa/nonexistent-builder-test",
        ))
        .max_page_fault_retries(7)
        .build()
        .expect("non-empty paths should validate before queue open");

    let err = DsaSession::builder()
        .dsa_config(config)
        .open()
        .err()
        .expect("missing work queue should surface queue-open diagnostics");

    assert_eq!(err.kind(), "queue_open");
    assert_eq!(
        err.device_path().and_then(|path| path.to_str()),
        Some("/dev/dsa/nonexistent-builder-test")
    );
    assert_eq!(err.phase(), Some(MemmovePhase::QueueOpen));
}

#[test]
fn queue_open_failure_preserves_io_source_chain() {
    let err = DsaSession::open("/dev/dsa/nonexistent-source-chain-test")
        .err()
        .expect("missing work queue should surface queue-open diagnostics");

    let source = StdError::source(&err).expect("queue-open errors should expose io source");
    assert!(source.is::<std::io::Error>());
    assert_eq!(err.kind(), "queue_open");
    assert_eq!(err.phase(), Some(MemmovePhase::QueueOpen));

    let message = err.to_string();
    assert!(message.contains("/dev/dsa/nonexistent-source-chain-test"));
    assert!(message.contains("queue_open"));
    assert_display_excludes_payload_bytes(&message);
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

#[test]
fn error_accessors_preserve_status_and_requested_boundaries() {
    let malformed = classify_memmove_completion(
        &test_config(),
        1024,
        CompletionSnapshot::new(3, 2, 128, 0xdead_beef),
        0,
    )
    .expect_err("malformed completions should retain status metadata");

    assert_eq!(malformed.kind(), "malformed_completion");
    assert_eq!(malformed.phase(), Some(MemmovePhase::PageFaultRetry));
    assert_eq!(malformed.page_fault_retries(), Some(0));
    assert_eq!(malformed.final_status(), Some(3));
    assert_eq!(malformed.requested_bytes(), None);

    let status = classify_memmove_completion(
        &test_config(),
        1024,
        CompletionSnapshot::new(0x12, 0, 64, 0xfeed_face),
        2,
    )
    .expect_err("non-success completions should retain final status metadata");

    assert_eq!(status.kind(), "completion_status");
    assert_eq!(status.phase(), Some(MemmovePhase::CompletionPoll));
    assert_eq!(status.page_fault_retries(), Some(2));
    assert_eq!(status.final_status(), Some(0x12));
    assert_eq!(status.requested_bytes(), None);

    let mismatch = MemmoveError::ByteMismatch {
        device_path: std::path::PathBuf::from("/dev/dsa/wq0.0"),
        phase: MemmovePhase::PostCopyVerify,
        requested_bytes: 4,
        mismatch_offset: 2,
        final_status: 1,
        page_fault_retries: 1,
    };

    assert_eq!(mismatch.kind(), "byte_mismatch");
    assert_eq!(mismatch.phase(), Some(MemmovePhase::PostCopyVerify));
    assert_eq!(mismatch.page_fault_retries(), Some(1));
    assert_eq!(mismatch.final_status(), Some(1));
    assert_eq!(mismatch.requested_bytes(), Some(4));

    let invalid_len = MemmoveRequest::new(0).expect_err("zero-length requests should fail");
    assert_eq!(invalid_len.kind(), "invalid_length");
    assert_eq!(invalid_len.device_path(), None);
    assert_eq!(invalid_len.phase(), None);
    assert_eq!(invalid_len.page_fault_retries(), None);
    assert_eq!(invalid_len.final_status(), None);
    assert_eq!(invalid_len.requested_bytes(), Some(0));

    let too_small = MemmoveRequest::for_buffers(255, 256)
        .expect_err("destination preconditions should fail before queue-open");
    assert_eq!(too_small.kind(), "destination_too_small");
    assert_eq!(too_small.requested_bytes(), Some(256));
}

#[test]
fn display_preserves_completion_metadata_without_payload_bytes() {
    let timeout = classify_memmove_completion(
        &test_config(),
        1024,
        CompletionSnapshot::new(0xff, 0, 0, 0),
        1,
    )
    .expect_err("timeout sentinel should map to a typed timeout")
    .to_string();
    assert!(timeout.contains("completion_poll"));
    assert!(timeout.contains("after 1 retries"));
    assert_display_excludes_payload_bytes(&timeout);

    let malformed = classify_memmove_completion(
        &test_config(),
        1024,
        CompletionSnapshot::new(3, 2, 128, 0xdead_beef),
        0,
    )
    .expect_err("invalid page-fault direction should be rejected as malformed")
    .to_string();
    assert!(malformed.contains("page_fault_retry"));
    assert!(malformed.contains("status=0x03"));
    assert!(malformed.contains("result=2"));
    assert!(malformed.contains("bytes_completed=128"));
    assert!(malformed.contains("fault_addr=0xdeadbeef"));
    assert_display_excludes_payload_bytes(&malformed);

    let exhausted = classify_memmove_completion(
        &test_config(),
        1024,
        CompletionSnapshot::new(3, 0, 128, 0xdead_beef),
        1,
    )
    .expect_err("retry exhaustion should surface explicitly")
    .to_string();
    assert!(exhausted.contains("retries=1"));
    assert!(exhausted.contains("bytes_completed=128"));
    assert!(exhausted.contains("fault_addr=0xdeadbeef"));
    assert_display_excludes_payload_bytes(&exhausted);

    let status = classify_memmove_completion(
        &test_config(),
        1024,
        CompletionSnapshot::new(0x12, 0, 64, 0xfeed_face),
        2,
    )
    .expect_err("non-success statuses should surface directly")
    .to_string();
    assert!(status.contains("status=0x12"));
    assert!(status.contains("after 2 retries"));
    assert!(status.contains("bytes_completed=64"));
    assert!(status.contains("fault_addr=0xfeedface"));
    assert_display_excludes_payload_bytes(&status);

    let mismatch = MemmoveError::ByteMismatch {
        device_path: std::path::PathBuf::from("/dev/dsa/wq0.0"),
        phase: MemmovePhase::PostCopyVerify,
        requested_bytes: 4,
        mismatch_offset: 2,
        final_status: 1,
        page_fault_retries: 1,
    }
    .to_string();
    assert!(mismatch.contains("post_copy_verify"));
    assert!(mismatch.contains("mismatch_offset=2"));
    assert!(mismatch.contains("requested_bytes=4"));
    assert!(mismatch.contains("final_status=0x01"));
    assert!(mismatch.contains("retries=1"));
    assert_display_excludes_payload_bytes(&mismatch);
}
