use idxd_sys::{
    IAX_COMP_NONE, IAX_COMP_OUTBUF_OVERFLOW, IAX_COMP_PAGE_FAULT_IR, IAX_COMP_STATUS_MASK,
    IAX_COMP_SUCCESS, IAX_CRC64_FLAGS_OFFSET, IAX_CRC64_POLY_OFFSET, IAX_CRC64_POLY_T10DIF,
    IAX_CRC64_RESULT_OFFSET, IAX_OPCODE_COMPRESS, IAX_OPCODE_CRC64, IAX_OPCODE_DECOMPRESS,
    IAX_OPCODE_MEMMOVE, IAX_OPCODE_NOOP, IAX_STATUS_ANALYTICS_ERROR, IDXD_OP_FLAG_CRAV,
    IDXD_OP_FLAG_RCR, IaxCompletionRecord, IaxHwDesc, crc16_t10dif, crc64_t10dif_field,
    drain_iax_completions, idxd_uapi, poll_iax_completion, reset_iax_completion,
    touch_iax_fault_page,
};
use std::mem::{align_of, offset_of, size_of};
use std::ptr;

#[test]
fn generated_iax_descriptor_layout_matches_linux_uapi_contract() {
    assert_eq!(
        size_of::<idxd_uapi::iax_hw_desc>(),
        64,
        "bindgen iax_hw_desc size drifted from the 64-byte hardware descriptor ABI"
    );
    assert_eq!(
        align_of::<idxd_uapi::iax_hw_desc>(),
        1,
        "bindgen should preserve linux/idxd.h packed descriptor alignment"
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_hw_desc, completion_addr),
        8,
        "completion_addr offset drifted in generated iax_hw_desc"
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_hw_desc, src1_addr),
        16,
        "src1_addr offset drifted in generated iax_hw_desc"
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_hw_desc, dst_addr),
        24,
        "dst_addr offset drifted in generated iax_hw_desc"
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_hw_desc, src1_size),
        32,
        "src1_size offset drifted in generated iax_hw_desc"
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_hw_desc, int_handle),
        36,
        "int_handle offset drifted in generated iax_hw_desc"
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_hw_desc, __bindgen_anon_1),
        IAX_CRC64_FLAGS_OFFSET,
        "crc64 flags offset drifted in generated iax_hw_desc"
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_hw_desc, src2_addr),
        40,
        "src2_addr offset drifted in generated iax_hw_desc"
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_hw_desc, max_dst_size),
        48,
        "max_dst_size offset drifted in generated iax_hw_desc"
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_hw_desc, src2_size),
        52,
        "src2_size offset drifted in generated iax_hw_desc"
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_hw_desc, filter_flags),
        IAX_CRC64_POLY_OFFSET,
        "crc64 polynomial offset drifted in generated iax_hw_desc"
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_hw_desc, num_inputs),
        60,
        "num_inputs offset drifted in generated iax_hw_desc"
    );
}

#[test]
fn generated_iax_completion_layout_matches_linux_uapi_contract() {
    assert_eq!(
        size_of::<idxd_uapi::iax_completion_record>(),
        64,
        "bindgen iax_completion_record size drifted from the 64-byte completion ABI"
    );
    assert_eq!(
        align_of::<idxd_uapi::iax_completion_record>(),
        1,
        "bindgen should preserve linux/idxd.h packed completion alignment"
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_completion_record, status),
        0,
        "status offset drifted in generated iax_completion_record"
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_completion_record, error_code),
        1,
        "error_code offset drifted in generated iax_completion_record"
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_completion_record, bytes_completed),
        4,
        "bytes_completed offset drifted in generated iax_completion_record"
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_completion_record, fault_addr),
        8,
        "fault_addr offset drifted in generated iax_completion_record"
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_completion_record, invalid_flags),
        16,
        "invalid_flags offset drifted in generated iax_completion_record"
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_completion_record, output_size),
        24,
        "output_size offset drifted in generated iax_completion_record"
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_completion_record, output_bits),
        28,
        "output_bits offset drifted in generated iax_completion_record"
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_completion_record, rsvd5),
        IAX_CRC64_RESULT_OFFSET,
        "crc64 result offset drifted in generated iax_completion_record"
    );
}

#[test]
fn public_iax_helpers_preserve_hardware_submission_alignment() {
    assert_eq!(
        size_of::<IaxHwDesc>(),
        size_of::<idxd_uapi::iax_hw_desc>(),
        "IaxHwDesc wrapper must not change descriptor size"
    );
    assert_eq!(
        align_of::<IaxHwDesc>(),
        64,
        "IaxHwDesc wrapper must restore 64-byte descriptor alignment"
    );
    assert_eq!(
        size_of::<IaxCompletionRecord>(),
        size_of::<idxd_uapi::iax_completion_record>(),
        "IaxCompletionRecord wrapper must not change completion size"
    );
    assert_eq!(
        align_of::<IaxCompletionRecord>(),
        64,
        "IaxCompletionRecord wrapper must restore 64-byte completion alignment"
    );
}

#[test]
fn public_iax_constants_are_sourced_from_generated_uapi_or_documented_raw_values() {
    assert_eq!(
        IAX_OPCODE_NOOP,
        idxd_uapi::iax_opcode::IAX_OPCODE_NOOP as u8
    );
    assert_eq!(
        IAX_OPCODE_MEMMOVE,
        idxd_uapi::iax_opcode::IAX_OPCODE_MEMMOVE as u8
    );
    assert_eq!(
        IAX_OPCODE_DECOMPRESS,
        idxd_uapi::iax_opcode::IAX_OPCODE_DECOMPRESS as u8
    );
    assert_eq!(
        IAX_OPCODE_COMPRESS,
        idxd_uapi::iax_opcode::IAX_OPCODE_COMPRESS as u8
    );
    assert_eq!(IAX_OPCODE_CRC64, 0x44);
    assert_eq!(IAX_STATUS_ANALYTICS_ERROR, 0x0a);
    assert_eq!(IAX_CRC64_POLY_T10DIF, 0x8BB7_0000_0000_0000);
    assert_eq!(IAX_CRC64_FLAGS_OFFSET, 38);
    assert_eq!(IAX_CRC64_POLY_OFFSET, 56);
    assert_eq!(IAX_CRC64_RESULT_OFFSET, 32);

    assert_eq!(
        IAX_COMP_NONE,
        idxd_uapi::iax_completion_status::IAX_COMP_NONE as u8
    );
    assert_eq!(
        IAX_COMP_SUCCESS,
        idxd_uapi::iax_completion_status::IAX_COMP_SUCCESS as u8
    );
    assert_eq!(
        IAX_COMP_PAGE_FAULT_IR,
        idxd_uapi::iax_completion_status::IAX_COMP_PAGE_FAULT_IR as u8
    );
    assert_eq!(
        IAX_COMP_OUTBUF_OVERFLOW,
        idxd_uapi::iax_completion_status::IAX_COMP_OUTBUF_OVERFLOW as u8
    );
    assert_eq!(IAX_COMP_STATUS_MASK, idxd_uapi::DSA_COMP_STATUS_MASK as u8);
}

#[test]
fn crc_reference_helpers_match_t10dif_known_vector() {
    assert_eq!(crc16_t10dif(b"123456789"), 0xD0DB);
    assert_eq!(crc64_t10dif_field(b"123456789"), 0xD0DB_0000_0000_0000);
}

#[test]
fn crc64_helper_writes_generated_opcode_flags_and_raw_fields() {
    let src = [0x5a_u8; 17];
    let mut desc = IaxHwDesc::default();
    let mut completion = IaxCompletionRecord::default();

    desc.fill_crc64(src.as_ptr(), src.len() as u32);
    desc.set_completion(&mut completion);

    assert_eq!(
        desc.opcode(),
        IAX_OPCODE_CRC64,
        "fill_crc64 must write the IAX crc64 opcode"
    );
    assert_eq!(
        desc.flags(),
        IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV,
        "fill_crc64 must request completion records and a valid completion address"
    );
    assert_eq!(
        desc.src1_addr(),
        src.as_ptr() as u64,
        "fill_crc64 must write src1_addr through the generated descriptor field"
    );
    assert_eq!(
        desc.src1_size(),
        src.len() as u32,
        "fill_crc64 must write src1_size through the generated descriptor field"
    );
    assert_eq!(
        desc.crc64_flags(),
        0,
        "fill_crc64 must clear the crc64 flags field at the raw offset"
    );
    assert_eq!(
        desc.crc64_poly(),
        IAX_CRC64_POLY_T10DIF,
        "fill_crc64 must write the T10DIF polynomial at the raw offset"
    );
    assert_eq!(
        desc.completion_addr(),
        (&mut completion as *mut IaxCompletionRecord) as u64,
        "set_completion must write the generated completion_addr field"
    );
}

#[test]
fn completion_accessors_read_generated_packed_fields_and_crc64_result_offset() {
    let mut completion = IaxCompletionRecord::default();
    let base = (&mut completion as *mut IaxCompletionRecord).cast::<u8>();

    // SAFETY: The wrapper is a repr(C) single-field wrapper around the generated
    // packed completion record with a larger alignment. These writes target the
    // generated field offsets guarded above and use unaligned stores for packed
    // multi-byte fields, matching the raw ABI contract without requiring hardware.
    unsafe {
        ptr::write(
            base.add(offset_of!(idxd_uapi::iax_completion_record, status)),
            0x81,
        );
        ptr::write(
            base.add(offset_of!(idxd_uapi::iax_completion_record, error_code)),
            IAX_STATUS_ANALYTICS_ERROR,
        );
        ptr::write_unaligned(
            base.add(offset_of!(idxd_uapi::iax_completion_record, fault_addr))
                .cast::<u64>(),
            0x1122_3344_5566_7788,
        );
        ptr::write_unaligned(
            base.add(offset_of!(idxd_uapi::iax_completion_record, invalid_flags))
                .cast::<u32>(),
            0x5566_7788,
        );
        ptr::write_unaligned(
            base.add(IAX_CRC64_RESULT_OFFSET).cast::<u64>(),
            0xD0DB_0000_0000_0000,
        );
    }

    assert_eq!(completion.status(), 0x81);
    assert_eq!(completion.error_code(), IAX_STATUS_ANALYTICS_ERROR);
    assert_eq!(completion.fault_addr(), 0x1122_3344_5566_7788);
    assert_eq!(completion.invalid_flags(), 0x5566_7788);
    assert_eq!(completion.crc64(), 0xD0DB_0000_0000_0000);
}

#[test]
fn reset_and_poll_helpers_preserve_raw_completion_contract() {
    let mut completion = IaxCompletionRecord::default();
    let bytes = (&mut completion as *mut IaxCompletionRecord).cast::<u8>();

    // SAFETY: The byte pattern only makes the host-free record non-zero so the
    // reset helper's raw zeroing effect is observable; no hardware observes it.
    unsafe {
        ptr::write_bytes(bytes, 0xa5, size_of::<IaxCompletionRecord>());
    }
    assert_ne!(completion.status(), IAX_COMP_NONE);

    reset_iax_completion(&mut completion);

    let bytes = (&completion as *const IaxCompletionRecord).cast::<u8>();
    for index in 0..size_of::<IaxCompletionRecord>() {
        // SAFETY: The wrapper is initialized, and reading its object
        // representation as bytes is valid for the reset contract check.
        let byte = unsafe { ptr::read(bytes.add(index)) };
        assert_eq!(byte, 0, "reset_iax_completion left byte {index} non-zero");
    }
    assert_eq!(completion.status(), IAX_COMP_NONE);

    let status = (&mut completion as *mut IaxCompletionRecord).cast::<u8>();
    // SAFETY: The wrapper is initialized. Writing only the status byte makes the
    // host-free poll helper take the immediate-completion branch without spinning.
    unsafe {
        ptr::write(status, IAX_COMP_SUCCESS | 0x80);
    }
    assert_eq!(poll_iax_completion(&completion), IAX_COMP_SUCCESS);
    drain_iax_completions(std::slice::from_ref(&completion));
    touch_iax_fault_page(&completion);
}
