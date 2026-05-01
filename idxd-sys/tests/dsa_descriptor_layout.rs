use idxd_sys::idxd_uapi;
use std::mem::{align_of, offset_of, size_of};

#[test]
fn generated_dsa_descriptor_layout_matches_linux_uapi_contract() {
    assert_eq!(size_of::<idxd_uapi::dsa_hw_desc>(), 64);
    assert_eq!(align_of::<idxd_uapi::dsa_hw_desc>(), 1);
    assert_eq!(offset_of!(idxd_uapi::dsa_hw_desc, completion_addr), 8);
    assert_eq!(offset_of!(idxd_uapi::dsa_hw_desc, __bindgen_anon_1), 16);
    assert_eq!(offset_of!(idxd_uapi::dsa_hw_desc, __bindgen_anon_2), 24);
    assert_eq!(offset_of!(idxd_uapi::dsa_hw_desc, __bindgen_anon_3), 32);
    assert_eq!(offset_of!(idxd_uapi::dsa_hw_desc, int_handle), 36);
    assert_eq!(offset_of!(idxd_uapi::dsa_hw_desc, __bindgen_anon_4), 40);
}

#[test]
fn generated_dsa_completion_layout_matches_linux_uapi_contract() {
    assert_eq!(size_of::<idxd_uapi::dsa_completion_record>(), 32);
    assert_eq!(align_of::<idxd_uapi::dsa_completion_record>(), 1);
    assert_eq!(offset_of!(idxd_uapi::dsa_completion_record, status), 0);
    assert_eq!(
        offset_of!(idxd_uapi::dsa_completion_record, __bindgen_anon_1),
        1
    );
    assert_eq!(
        offset_of!(idxd_uapi::dsa_completion_record, bytes_completed),
        4
    );
    assert_eq!(offset_of!(idxd_uapi::dsa_completion_record, fault_addr), 8);
    assert_eq!(
        offset_of!(idxd_uapi::dsa_completion_record, __bindgen_anon_2),
        16
    );
}

#[test]
fn generated_dsa_values_remain_available_through_bindgen_module() {
    assert_eq!(idxd_uapi::dsa_opcode::DSA_OPCODE_MEMMOVE as u8, 3);
    assert_eq!(idxd_uapi::dsa_completion_status::DSA_COMP_NONE as u8, 0);
    assert_eq!(idxd_uapi::dsa_completion_status::DSA_COMP_SUCCESS as u8, 1);
    assert_eq!(idxd_uapi::DSA_COMP_STATUS_MASK as u8, 0x7f);
}
