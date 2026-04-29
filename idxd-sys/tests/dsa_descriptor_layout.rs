use idxd_sys::{
    DSA_COMP_NONE, DSA_COMP_PAGE_FAULT_NOBOF, DSA_COMP_STATUS_MASK, DSA_COMP_SUCCESS,
    DSA_OPCODE_BATCH, DSA_OPCODE_CFLUSH, DSA_OPCODE_COMPARE, DSA_OPCODE_COMPVAL,
    DSA_OPCODE_COPY_CRC, DSA_OPCODE_CRCGEN, DSA_OPCODE_DUALCAST, DSA_OPCODE_MEMFILL,
    DSA_OPCODE_MEMMOVE, DSA_OPCODE_NOOP, DsaCompletionRecord, DsaHwDesc, IDXD_OP_FLAG_CC,
    IDXD_OP_FLAG_CRAV, IDXD_OP_FLAG_RCR, idxd_uapi, reset_completion,
};
use std::mem::{align_of, offset_of, size_of};
use std::ptr;

#[test]
fn generated_dsa_descriptor_layout_matches_linux_uapi_contract() {
    assert_eq!(
        size_of::<idxd_uapi::dsa_hw_desc>(),
        64,
        "bindgen dsa_hw_desc size drifted from the 64-byte hardware descriptor ABI"
    );
    assert_eq!(
        align_of::<idxd_uapi::dsa_hw_desc>(),
        1,
        "bindgen should preserve linux/idxd.h packed descriptor alignment"
    );
    assert_eq!(
        offset_of!(idxd_uapi::dsa_hw_desc, completion_addr),
        8,
        "completion_addr offset drifted in generated dsa_hw_desc"
    );
    assert_eq!(
        offset_of!(idxd_uapi::dsa_hw_desc, __bindgen_anon_1),
        16,
        "memmove src union offset drifted in generated dsa_hw_desc"
    );
    assert_eq!(
        offset_of!(idxd_uapi::dsa_hw_desc, __bindgen_anon_2),
        24,
        "memmove dst union offset drifted in generated dsa_hw_desc"
    );
    assert_eq!(
        offset_of!(idxd_uapi::dsa_hw_desc, __bindgen_anon_3),
        32,
        "memmove transfer-size union offset drifted in generated dsa_hw_desc"
    );
    assert_eq!(
        offset_of!(idxd_uapi::dsa_hw_desc, int_handle),
        36,
        "int_handle offset drifted in generated dsa_hw_desc"
    );
    assert_eq!(
        offset_of!(idxd_uapi::dsa_hw_desc, __bindgen_anon_4),
        40,
        "operation-specific union offset drifted in generated dsa_hw_desc"
    );
}

#[test]
fn generated_dsa_completion_layout_matches_linux_uapi_contract() {
    assert_eq!(
        size_of::<idxd_uapi::dsa_completion_record>(),
        32,
        "bindgen dsa_completion_record size drifted from the 32-byte hardware completion ABI"
    );
    assert_eq!(
        align_of::<idxd_uapi::dsa_completion_record>(),
        1,
        "bindgen should preserve linux/idxd.h packed completion alignment"
    );
    assert_eq!(
        offset_of!(idxd_uapi::dsa_completion_record, status),
        0,
        "completion status offset drifted in generated dsa_completion_record"
    );
    assert_eq!(
        offset_of!(idxd_uapi::dsa_completion_record, __bindgen_anon_1),
        1,
        "completion result union offset drifted in generated dsa_completion_record"
    );
    assert_eq!(
        offset_of!(idxd_uapi::dsa_completion_record, bytes_completed),
        4,
        "bytes_completed offset drifted in generated dsa_completion_record"
    );
    assert_eq!(
        offset_of!(idxd_uapi::dsa_completion_record, fault_addr),
        8,
        "fault_addr offset drifted in generated dsa_completion_record"
    );
    assert_eq!(
        offset_of!(idxd_uapi::dsa_completion_record, __bindgen_anon_2),
        16,
        "completion crc/operation-specific union offset drifted in generated dsa_completion_record"
    );
}

#[test]
fn public_dsa_helpers_preserve_hardware_submission_alignment() {
    assert_eq!(
        size_of::<DsaHwDesc>(),
        size_of::<idxd_uapi::dsa_hw_desc>(),
        "DsaHwDesc wrapper must not change descriptor size"
    );
    assert_eq!(
        align_of::<DsaHwDesc>(),
        64,
        "DsaHwDesc wrapper must restore MOVDIR64B 64-byte descriptor alignment"
    );
    assert_eq!(
        size_of::<DsaCompletionRecord>(),
        size_of::<idxd_uapi::dsa_completion_record>(),
        "DsaCompletionRecord wrapper must not change completion size"
    );
    assert_eq!(
        align_of::<DsaCompletionRecord>(),
        32,
        "DsaCompletionRecord wrapper must restore 32-byte completion alignment"
    );
}

#[test]
fn public_dsa_constants_are_sourced_from_generated_uapi() {
    assert_eq!(
        DSA_OPCODE_NOOP,
        idxd_uapi::dsa_opcode::DSA_OPCODE_NOOP as u8
    );
    assert_eq!(
        DSA_OPCODE_BATCH,
        idxd_uapi::dsa_opcode::DSA_OPCODE_BATCH as u8
    );
    assert_eq!(
        DSA_OPCODE_MEMMOVE,
        idxd_uapi::dsa_opcode::DSA_OPCODE_MEMMOVE as u8
    );
    assert_eq!(
        DSA_OPCODE_MEMFILL,
        idxd_uapi::dsa_opcode::DSA_OPCODE_MEMFILL as u8
    );
    assert_eq!(
        DSA_OPCODE_COMPARE,
        idxd_uapi::dsa_opcode::DSA_OPCODE_COMPARE as u8
    );
    assert_eq!(
        DSA_OPCODE_COMPVAL,
        idxd_uapi::dsa_opcode::DSA_OPCODE_COMPVAL as u8
    );
    assert_eq!(
        DSA_OPCODE_DUALCAST,
        idxd_uapi::dsa_opcode::DSA_OPCODE_DUALCAST as u8
    );
    assert_eq!(
        DSA_OPCODE_CRCGEN,
        idxd_uapi::dsa_opcode::DSA_OPCODE_CRCGEN as u8
    );
    assert_eq!(
        DSA_OPCODE_COPY_CRC,
        idxd_uapi::dsa_opcode::DSA_OPCODE_COPY_CRC as u8
    );
    assert_eq!(
        DSA_OPCODE_CFLUSH,
        idxd_uapi::dsa_opcode::DSA_OPCODE_CFLUSH as u8
    );

    assert_eq!(IDXD_OP_FLAG_CRAV, idxd_uapi::IDXD_OP_FLAG_CRAV);
    assert_eq!(IDXD_OP_FLAG_RCR, idxd_uapi::IDXD_OP_FLAG_RCR);
    assert_eq!(IDXD_OP_FLAG_CC, idxd_uapi::IDXD_OP_FLAG_CC);

    assert_eq!(
        DSA_COMP_NONE,
        idxd_uapi::dsa_completion_status::DSA_COMP_NONE as u8
    );
    assert_eq!(
        DSA_COMP_SUCCESS,
        idxd_uapi::dsa_completion_status::DSA_COMP_SUCCESS as u8
    );
    assert_eq!(
        DSA_COMP_PAGE_FAULT_NOBOF,
        idxd_uapi::dsa_completion_status::DSA_COMP_PAGE_FAULT_NOBOF as u8
    );
    assert_eq!(DSA_COMP_STATUS_MASK, idxd_uapi::DSA_COMP_STATUS_MASK as u8);
}

#[test]
fn completion_accessors_read_generated_packed_fields() {
    let mut completion = DsaCompletionRecord::default();
    let base = (&mut completion as *mut DsaCompletionRecord).cast::<u8>();

    // SAFETY: The wrapper is a repr(C) single-field wrapper around the generated
    // packed completion record with a larger alignment. These writes target the
    // generated field offsets guarded above and use unaligned stores for packed
    // multi-byte fields, matching the raw ABI contract without requiring hardware.
    unsafe {
        ptr::write(
            base.add(offset_of!(idxd_uapi::dsa_completion_record, status)),
            0x81,
        );
        ptr::write(
            base.add(offset_of!(
                idxd_uapi::dsa_completion_record,
                __bindgen_anon_1
            )),
            0x7e,
        );
        ptr::write_unaligned(
            base.add(offset_of!(
                idxd_uapi::dsa_completion_record,
                bytes_completed
            ))
            .cast::<u32>(),
            0x1122_3344,
        );
        ptr::write_unaligned(
            base.add(offset_of!(idxd_uapi::dsa_completion_record, fault_addr))
                .cast::<u64>(),
            0x1122_3344_5566_7788,
        );
        ptr::write_unaligned(
            base.add(offset_of!(
                idxd_uapi::dsa_completion_record,
                __bindgen_anon_2
            ))
            .cast::<u64>(),
            0x8877_6655_4433_2211,
        );
    }

    assert_eq!(completion.status(), 0x81);
    assert_eq!(completion.result(), 0x7e);
    assert_eq!(completion.bytes_completed(), 0x1122_3344);
    assert_eq!(completion.fault_addr(), 0x1122_3344_5566_7788);
    assert_eq!(completion.crc_value(), 0x8877_6655_4433_2211);
}

#[test]
fn reset_completion_clears_record_without_changing_wrapper_contract() {
    let mut completion = DsaCompletionRecord::default();
    let bytes = (&mut completion as *mut DsaCompletionRecord).cast::<u8>();

    // SAFETY: The byte pattern only makes the host-free record non-zero so the
    // reset helper's raw zeroing effect is observable; no hardware observes it.
    unsafe {
        ptr::write_bytes(bytes, 0xa5, size_of::<DsaCompletionRecord>());
    }
    assert_ne!(completion.status(), DSA_COMP_NONE);

    reset_completion(&mut completion);

    let bytes = (&completion as *const DsaCompletionRecord).cast::<u8>();
    for index in 0..size_of::<DsaCompletionRecord>() {
        // SAFETY: The wrapper is initialized, and reading its object
        // representation as bytes is valid for the reset contract check.
        let byte = unsafe { ptr::read(bytes.add(index)) };
        assert_eq!(byte, 0, "reset_completion left byte {index} non-zero");
    }
    assert_eq!(completion.status(), DSA_COMP_NONE);
    assert_eq!(size_of::<DsaCompletionRecord>(), 32);
    assert_eq!(align_of::<DsaCompletionRecord>(), 32);
}

#[test]
fn memmove_helper_writes_generated_opcode_flags_and_fields() {
    let src = [0x5a_u8; 17];
    let mut dst = [0_u8; 17];
    let mut desc = DsaHwDesc::default();
    let mut completion = DsaCompletionRecord::default();

    desc.fill_memmove(src.as_ptr(), dst.as_mut_ptr(), src.len() as u32);
    desc.set_completion(&mut completion);

    assert_eq!(
        desc.opcode(),
        DSA_OPCODE_MEMMOVE,
        "fill_memmove must write the generated DSA memmove opcode"
    );
    assert_eq!(
        desc.flags(),
        IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | IDXD_OP_FLAG_CC,
        "fill_memmove must request completion records, valid completion address, and cache control"
    );
    assert_eq!(
        desc.src_addr(),
        src.as_ptr() as u64,
        "fill_memmove must write src_addr through the generated descriptor union"
    );
    assert_eq!(
        desc.dst_addr(),
        dst.as_mut_ptr() as u64,
        "fill_memmove must write dst_addr through the generated descriptor union"
    );
    assert_eq!(
        desc.xfer_size(),
        src.len() as u32,
        "fill_memmove must write xfer_size through the generated descriptor union"
    );
    assert_eq!(
        desc.completion_addr(),
        (&mut completion as *mut DsaCompletionRecord) as u64,
        "set_completion must write the generated completion_addr field"
    );
}
