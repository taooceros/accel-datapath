use idxd_rust::{
    classify_memmove_completion, CompletionAction, CompletionSnapshot, DsaSession, MemmoveError,
    MemmovePhase, MemmoveRequest, MemmoveRetry, MemmoveValidationConfig, MemmoveValidationReport,
};

fn test_config() -> MemmoveValidationConfig {
    MemmoveValidationConfig::with_retries("/dev/dsa/wq0.0", 1).expect("test config")
}

#[test]
fn exposes_stable_validation_config_fields() {
    let config = MemmoveValidationConfig::with_retries("/dev/dsa/wq1.2", 3)
        .expect("non-empty device paths should be accepted");

    assert_eq!(config.device_path().to_str(), Some("/dev/dsa/wq1.2"));
    assert_eq!(config.max_page_fault_retries(), 3);
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
