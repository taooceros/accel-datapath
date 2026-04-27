use idxd_sys::{
    DSA_OPCODE_MEMMOVE, DsaCompletionRecord, DsaHwDesc, IDXD_OP_FLAG_CC, IDXD_OP_FLAG_CRAV,
    IDXD_OP_FLAG_RCR, idxd_uapi,
};
use std::mem::{align_of, offset_of, size_of};

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
