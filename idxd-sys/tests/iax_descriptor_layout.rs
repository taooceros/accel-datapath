use idxd_sys::idxd_uapi;
use std::mem::{align_of, offset_of, size_of};

const IAX_CRC64_FLAGS_OFFSET: usize = 38;
const IAX_CRC64_POLY_OFFSET: usize = 56;
const IAX_CRC64_RESULT_OFFSET: usize = 32;

#[test]
fn generated_iax_descriptor_layout_matches_linux_uapi_contract() {
    assert_eq!(size_of::<idxd_uapi::iax_hw_desc>(), 64);
    assert_eq!(align_of::<idxd_uapi::iax_hw_desc>(), 1);
    assert_eq!(offset_of!(idxd_uapi::iax_hw_desc, completion_addr), 8);
    assert_eq!(offset_of!(idxd_uapi::iax_hw_desc, src1_addr), 16);
    assert_eq!(offset_of!(idxd_uapi::iax_hw_desc, dst_addr), 24);
    assert_eq!(offset_of!(idxd_uapi::iax_hw_desc, src1_size), 32);
    assert_eq!(offset_of!(idxd_uapi::iax_hw_desc, int_handle), 36);
    assert_eq!(
        offset_of!(idxd_uapi::iax_hw_desc, __bindgen_anon_1),
        IAX_CRC64_FLAGS_OFFSET
    );
    assert_eq!(offset_of!(idxd_uapi::iax_hw_desc, src2_addr), 40);
    assert_eq!(offset_of!(idxd_uapi::iax_hw_desc, max_dst_size), 48);
    assert_eq!(offset_of!(idxd_uapi::iax_hw_desc, src2_size), 52);
    assert_eq!(
        offset_of!(idxd_uapi::iax_hw_desc, filter_flags),
        IAX_CRC64_POLY_OFFSET
    );
    assert_eq!(offset_of!(idxd_uapi::iax_hw_desc, num_inputs), 60);
}

#[test]
fn generated_iax_completion_layout_matches_linux_uapi_contract() {
    assert_eq!(size_of::<idxd_uapi::iax_completion_record>(), 64);
    assert_eq!(align_of::<idxd_uapi::iax_completion_record>(), 1);
    assert_eq!(offset_of!(idxd_uapi::iax_completion_record, status), 0);
    assert_eq!(offset_of!(idxd_uapi::iax_completion_record, error_code), 1);
    assert_eq!(
        offset_of!(idxd_uapi::iax_completion_record, bytes_completed),
        4
    );
    assert_eq!(offset_of!(idxd_uapi::iax_completion_record, fault_addr), 8);
    assert_eq!(
        offset_of!(idxd_uapi::iax_completion_record, invalid_flags),
        16
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_completion_record, output_size),
        24
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_completion_record, output_bits),
        28
    );
    assert_eq!(
        offset_of!(idxd_uapi::iax_completion_record, rsvd5),
        IAX_CRC64_RESULT_OFFSET
    );
}

#[test]
fn generated_iax_values_remain_available_through_bindgen_module() {
    assert_eq!(idxd_uapi::iax_opcode::IAX_OPCODE_NOOP as u8, 0);
    assert_eq!(idxd_uapi::iax_completion_status::IAX_COMP_NONE as u8, 0);
    assert_eq!(idxd_uapi::iax_completion_status::IAX_COMP_SUCCESS as u8, 1);
    assert_eq!(idxd_uapi::DSA_COMP_STATUS_MASK as u8, 0x7f);
}
