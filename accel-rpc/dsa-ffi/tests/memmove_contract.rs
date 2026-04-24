use dsa_ffi::{
    classify_memmove_completion, CompletionAction, CompletionSnapshot, DsaSession, MemmoveError,
    MemmoveRequest,
};

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
fn rejects_empty_device_path_before_queue_open() {
    let err = DsaSession::open("")
        .err()
        .expect("empty device paths should fail validation");
    assert!(matches!(err, MemmoveError::InvalidDevicePath { .. }));
}

#[test]
fn classifies_success_completion() {
    let action = classify_memmove_completion(
        "/dev/dsa/wq0.0".into(),
        1024,
        CompletionSnapshot::new(1, 0, 0, 0),
        0,
        1,
    )
    .expect("success status should pass");

    assert_eq!(action, CompletionAction::Success);
}

#[test]
fn advances_offsets_for_forward_page_fault_retry() {
    let action = classify_memmove_completion(
        "/dev/dsa/wq0.0".into(),
        1024,
        CompletionSnapshot::new(3, 0, 128, 0xdead_beef),
        0,
        1,
    )
    .expect("recoverable page fault should request a retry");

    assert_eq!(
        action,
        CompletionAction::Retry(dsa_ffi::MemmoveRetry {
            next_src_offset: 128,
            next_dst_offset: 128,
            remaining_bytes: 896,
        })
    );
}

#[test]
fn preserves_offsets_for_reverse_copy_page_fault_retry() {
    let action = classify_memmove_completion(
        "/dev/dsa/wq0.0".into(),
        1024,
        CompletionSnapshot::new(3, 1, 128, 0xdead_beef),
        0,
        1,
    )
    .expect("reverse-copy page fault should keep original base pointers");

    assert_eq!(
        action,
        CompletionAction::Retry(dsa_ffi::MemmoveRetry {
            next_src_offset: 0,
            next_dst_offset: 0,
            remaining_bytes: 896,
        })
    );
}

#[test]
fn rejects_page_fault_without_fault_address() {
    let err = classify_memmove_completion(
        "/dev/dsa/wq0.0".into(),
        1024,
        CompletionSnapshot::new(3, 0, 128, 0),
        0,
        1,
    )
    .expect_err("page fault without address should be rejected");

    assert!(matches!(err, MemmoveError::MalformedCompletion { .. }));
}

#[test]
fn rejects_page_fault_when_retry_budget_is_exhausted() {
    let err = classify_memmove_completion(
        "/dev/dsa/wq0.0".into(),
        1024,
        CompletionSnapshot::new(3, 0, 128, 0xdead_beef),
        1,
        1,
    )
    .expect_err("retry exhaustion should surface explicitly");

    assert!(matches!(err, MemmoveError::PageFaultRetryExhausted { .. }));
}

#[test]
fn maps_poll_timeout_to_completion_timeout() {
    let err = classify_memmove_completion(
        "/dev/dsa/wq0.0".into(),
        1024,
        CompletionSnapshot::new(0xff, 0, 0, 0),
        0,
        1,
    )
    .expect_err("timeout sentinel should map to a typed timeout");

    assert!(matches!(err, MemmoveError::CompletionTimeout { .. }));
}

#[test]
fn surfaces_non_success_completion_statuses() {
    let err = classify_memmove_completion(
        "/dev/dsa/wq0.0".into(),
        1024,
        CompletionSnapshot::new(0x12, 0, 0, 0),
        0,
        1,
    )
    .expect_err("non-success statuses should surface directly");

    assert!(matches!(
        err,
        MemmoveError::CompletionStatus { status: 0x12, .. }
    ));
}
