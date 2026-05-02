use std::mem::{align_of, size_of};

use idxd_rust::{
    DsaDifCheck, DsaDifInsert, DsaDifUpdate, DsaFlag, DsaFlags, DsaHwDesc, DsaOpcode,
    default_completion_flags,
};

fn assert_opcode(desc: &DsaHwDesc, opcode: DsaOpcode) {
    assert_eq!(desc.opcode(), opcode.as_u8());
}

fn assert_flags(desc: &DsaHwDesc, flags: DsaFlags) {
    assert_eq!(desc.flags(), flags.bits() & 0x00ff_ffff);
}

fn read_u8(bytes: &[u8; 24], offset: usize) -> u8 {
    bytes[offset]
}

fn read_u16(bytes: &[u8; 24], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}

fn read_u32(bytes: &[u8; 24], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn read_u64(bytes: &[u8; 24], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}

#[test]
fn dsa_descriptor_wrapper_preserves_descriptor_alignment() {
    assert_eq!(size_of::<DsaHwDesc>(), 64);
    assert_eq!(align_of::<DsaHwDesc>(), 64);
}

#[test]
fn noop_sets_opcode_and_supplied_flags() {
    let mut desc = DsaHwDesc::default();
    let flags = DsaFlag::Fence | DsaFlag::DrainStatus;

    desc.fill_noop(flags);

    assert_opcode(&desc, DsaOpcode::Noop);
    assert_flags(&desc, flags);
}

#[test]
fn batch_sets_descriptor_list_and_count() {
    let list = [DsaHwDesc::default(), DsaHwDesc::default()];
    let mut desc = DsaHwDesc::default();
    let flags = DsaFlag::RequestCompletionRecord | DsaFlag::CompletionRecordAddressValid;

    desc.fill_batch(list.as_ptr(), 2, flags);

    assert_opcode(&desc, DsaOpcode::Batch);
    assert_flags(&desc, flags);
    assert_eq!(desc.src_addr(), list.as_ptr() as u64);
    assert_eq!(desc.desc_count(), 2);
}

#[test]
fn drain_sets_opcode_and_supplied_flags() {
    let mut desc = DsaHwDesc::default();
    let flags = DsaFlag::DrainReadback | DsaFlag::DrainStatus;

    desc.fill_drain(flags);

    assert_opcode(&desc, DsaOpcode::Drain);
    assert_flags(&desc, flags);
}

#[test]
fn memmove_sets_source_destination_and_size() {
    let src = [1_u8; 8];
    let mut dst = [0_u8; 8];
    let mut desc = DsaHwDesc::default();

    desc.fill_memmove(src.as_ptr(), dst.as_mut_ptr(), 8);

    assert_opcode(&desc, DsaOpcode::Memmove);
    assert_flags(&desc, default_completion_flags());
    assert_eq!(desc.src_addr(), src.as_ptr() as u64);
    assert_eq!(desc.dst_addr(), dst.as_mut_ptr() as u64);
    assert_eq!(desc.xfer_size(), 8);
}

#[test]
fn memfill_sets_pattern_destination_and_size() {
    let mut dst = [0_u8; 16];
    let mut desc = DsaHwDesc::default();
    let pattern = 0x1122_3344_5566_7788;

    desc.fill_memfill(pattern, dst.as_mut_ptr(), 16);

    assert_opcode(&desc, DsaOpcode::Memfill);
    assert_flags(&desc, default_completion_flags());
    assert_eq!(desc.src_addr(), pattern);
    assert_eq!(desc.dst_addr(), dst.as_mut_ptr() as u64);
    assert_eq!(desc.xfer_size(), 16);
}

#[test]
fn compare_sets_both_sources_and_size() {
    let src1 = [1_u8; 4];
    let src2 = [2_u8; 4];
    let mut desc = DsaHwDesc::default();

    desc.fill_compare(src1.as_ptr(), src2.as_ptr(), 4);

    assert_opcode(&desc, DsaOpcode::Compare);
    assert_flags(
        &desc,
        DsaFlag::RequestCompletionRecord | DsaFlag::CompletionRecordAddressValid,
    );
    assert_eq!(desc.src_addr(), src1.as_ptr() as u64);
    assert_eq!(desc.dst_addr(), src2.as_ptr() as u64);
    assert_eq!(desc.xfer_size(), 4);
}

#[test]
fn compare_value_sets_source_pattern_and_size() {
    let src = [3_u8; 4];
    let mut desc = DsaHwDesc::default();
    let pattern = 0xaabb_ccdd_eeff_0011;

    desc.fill_compare_value(src.as_ptr(), pattern, 4);

    assert_opcode(&desc, DsaOpcode::CompareValue);
    assert_flags(
        &desc,
        DsaFlag::RequestCompletionRecord | DsaFlag::CompletionRecordAddressValid,
    );
    assert_eq!(desc.src_addr(), src.as_ptr() as u64);
    assert_eq!(desc.dst_addr(), pattern);
    assert_eq!(desc.xfer_size(), 4);
}

#[test]
fn create_delta_sets_sources_size_and_delta_fields() {
    let src1 = [1_u8; 32];
    let src2 = [2_u8; 32];
    let mut delta = [0_u8; 64];
    let mut desc = DsaHwDesc::default();

    desc.fill_create_delta(
        src1.as_ptr(),
        src2.as_ptr(),
        32,
        delta.as_mut_ptr(),
        64,
        0x5a,
    );

    let op = desc.op_specific();
    assert_opcode(&desc, DsaOpcode::CreateDelta);
    assert_flags(&desc, default_completion_flags());
    assert_eq!(desc.src_addr(), src1.as_ptr() as u64);
    assert_eq!(desc.dst_addr(), src2.as_ptr() as u64);
    assert_eq!(desc.xfer_size(), 32);
    assert_eq!(read_u64(&op, 0), delta.as_mut_ptr() as u64);
    assert_eq!(read_u32(&op, 8), 64);
    assert_eq!(read_u8(&op, 16), 0x5a);
}

#[test]
fn apply_delta_sets_delta_destination_and_record_size() {
    let delta = [1_u8; 64];
    let mut dst = [0_u8; 64];
    let mut desc = DsaHwDesc::default();

    desc.fill_apply_delta(delta.as_ptr(), dst.as_mut_ptr(), 64);

    let op = desc.op_specific();
    assert_opcode(&desc, DsaOpcode::ApplyDelta);
    assert_flags(&desc, default_completion_flags());
    assert_eq!(desc.src_addr(), delta.as_ptr() as u64);
    assert_eq!(desc.dst_addr(), dst.as_mut_ptr() as u64);
    assert_eq!(read_u32(&op, 0), 64);
}

#[test]
fn dualcast_sets_source_two_destinations_and_size() {
    let src = [1_u8; 64];
    let mut dst1 = [0_u8; 64];
    let mut dst2 = [0_u8; 64];
    let mut desc = DsaHwDesc::default();

    desc.fill_dualcast(src.as_ptr(), dst1.as_mut_ptr(), dst2.as_mut_ptr(), 64);

    let op = desc.op_specific();
    assert_opcode(&desc, DsaOpcode::Dualcast);
    assert_flags(&desc, default_completion_flags());
    assert_eq!(desc.src_addr(), src.as_ptr() as u64);
    assert_eq!(desc.dst_addr(), dst1.as_mut_ptr() as u64);
    assert_eq!(desc.xfer_size(), 64);
    assert_eq!(read_u64(&op, 0), dst2.as_mut_ptr() as u64);
}

#[test]
fn crc_gen_sets_source_size_seed_and_seed_address() {
    let src = [1_u8; 64];
    let mut desc = DsaHwDesc::default();

    desc.fill_crc_gen(src.as_ptr(), 64, 0xfeed_beef, 0x1122_3344_5566_7788);

    let op = desc.op_specific();
    assert_opcode(&desc, DsaOpcode::CrcGen);
    assert_flags(
        &desc,
        DsaFlag::RequestCompletionRecord | DsaFlag::CompletionRecordAddressValid,
    );
    assert_eq!(desc.src_addr(), src.as_ptr() as u64);
    assert_eq!(desc.xfer_size(), 64);
    assert_eq!(read_u32(&op, 0), 0xfeed_beef);
    assert_eq!(read_u64(&op, 8), 0x1122_3344_5566_7788);
}

#[test]
fn copy_crc_sets_copy_fields_and_crc_fields() {
    let src = [1_u8; 32];
    let mut dst = [0_u8; 32];
    let mut desc = DsaHwDesc::default();

    desc.fill_copy_crc(
        src.as_ptr(),
        dst.as_mut_ptr(),
        32,
        0x1234_5678,
        0x8877_6655_4433_2211,
    );

    let op = desc.op_specific();
    assert_opcode(&desc, DsaOpcode::CopyCrc);
    assert_flags(
        &desc,
        DsaFlag::RequestCompletionRecord | DsaFlag::CompletionRecordAddressValid,
    );
    assert_eq!(desc.src_addr(), src.as_ptr() as u64);
    assert_eq!(desc.dst_addr(), dst.as_mut_ptr() as u64);
    assert_eq!(desc.xfer_size(), 32);
    assert_eq!(read_u32(&op, 0), 0x1234_5678);
    assert_eq!(read_u64(&op, 8), 0x8877_6655_4433_2211);
}

#[test]
fn dif_check_sets_source_size_and_check_fields() {
    let src = [1_u8; 128];
    let mut desc = DsaHwDesc::default();
    let dif = DsaDifCheck {
        src_dif_flags: 0x11,
        flags: 0x22,
        ref_tag_seed: 0x3344_5566,
        app_tag_mask: 0x7788,
        app_tag_seed: 0x99aa,
    };

    desc.fill_dif_check(src.as_ptr(), 128, dif);

    let op = desc.op_specific();
    assert_opcode(&desc, DsaOpcode::DifCheck);
    assert_flags(&desc, default_completion_flags());
    assert_eq!(desc.src_addr(), src.as_ptr() as u64);
    assert_eq!(desc.xfer_size(), 128);
    assert_eq!(read_u8(&op, 0), dif.src_dif_flags);
    assert_eq!(read_u8(&op, 2), dif.flags);
    assert_eq!(read_u32(&op, 8), dif.ref_tag_seed);
    assert_eq!(read_u16(&op, 12), dif.app_tag_mask);
    assert_eq!(read_u16(&op, 14), dif.app_tag_seed);
}

#[test]
fn dif_insert_sets_copy_fields_and_insert_fields() {
    let src = [1_u8; 128];
    let mut dst = [0_u8; 128];
    let mut desc = DsaHwDesc::default();
    let dif = DsaDifInsert {
        dest_dif_flags: 0x12,
        flags: 0x23,
        ref_tag_seed: 0x3456_789a,
        app_tag_mask: 0xbcde,
        app_tag_seed: 0xf012,
    };

    desc.fill_dif_insert(src.as_ptr(), dst.as_mut_ptr(), 128, dif);

    let op = desc.op_specific();
    assert_opcode(&desc, DsaOpcode::DifInsert);
    assert_flags(&desc, default_completion_flags());
    assert_eq!(desc.src_addr(), src.as_ptr() as u64);
    assert_eq!(desc.dst_addr(), dst.as_mut_ptr() as u64);
    assert_eq!(desc.xfer_size(), 128);
    assert_eq!(read_u8(&op, 1), dif.dest_dif_flags);
    assert_eq!(read_u8(&op, 2), dif.flags);
    assert_eq!(read_u32(&op, 16), dif.ref_tag_seed);
    assert_eq!(read_u16(&op, 20), dif.app_tag_mask);
    assert_eq!(read_u16(&op, 22), dif.app_tag_seed);
}

#[test]
fn dif_strip_sets_copy_fields_and_check_strip_fields() {
    let src = [1_u8; 128];
    let mut dst = [0_u8; 128];
    let mut desc = DsaHwDesc::default();
    let dif = DsaDifCheck {
        src_dif_flags: 0x31,
        flags: 0x42,
        ref_tag_seed: 0x5364_7586,
        app_tag_mask: 0x97a8,
        app_tag_seed: 0xb9ca,
    };

    desc.fill_dif_strip(src.as_ptr(), dst.as_mut_ptr(), 128, dif);

    let op = desc.op_specific();
    assert_opcode(&desc, DsaOpcode::DifStrip);
    assert_flags(&desc, default_completion_flags());
    assert_eq!(desc.src_addr(), src.as_ptr() as u64);
    assert_eq!(desc.dst_addr(), dst.as_mut_ptr() as u64);
    assert_eq!(desc.xfer_size(), 128);
    assert_eq!(read_u8(&op, 0), dif.src_dif_flags);
    assert_eq!(read_u8(&op, 2), dif.flags);
    assert_eq!(read_u32(&op, 8), dif.ref_tag_seed);
    assert_eq!(read_u16(&op, 12), dif.app_tag_mask);
    assert_eq!(read_u16(&op, 14), dif.app_tag_seed);
}

#[test]
fn dif_update_sets_copy_fields_and_update_fields() {
    let src = [1_u8; 128];
    let mut dst = [0_u8; 128];
    let mut desc = DsaHwDesc::default();
    let dif = DsaDifUpdate {
        src_flags: 0x01,
        dest_flags: 0x02,
        flags: 0x03,
        src_ref_tag_seed: 0x1122_3344,
        src_app_tag_mask: 0x5566,
        src_app_tag_seed: 0x7788,
        dest_ref_tag_seed: 0x99aa_bbcc,
        dest_app_tag_mask: 0xddee,
        dest_app_tag_seed: 0xff00,
    };

    desc.fill_dif_update(src.as_ptr(), dst.as_mut_ptr(), 128, dif);

    let op = desc.op_specific();
    assert_opcode(&desc, DsaOpcode::DifUpdate);
    assert_flags(&desc, default_completion_flags());
    assert_eq!(desc.src_addr(), src.as_ptr() as u64);
    assert_eq!(desc.dst_addr(), dst.as_mut_ptr() as u64);
    assert_eq!(desc.xfer_size(), 128);
    assert_eq!(read_u8(&op, 0), dif.src_flags);
    assert_eq!(read_u8(&op, 1), dif.dest_flags);
    assert_eq!(read_u8(&op, 2), dif.flags);
    assert_eq!(read_u32(&op, 8), dif.src_ref_tag_seed);
    assert_eq!(read_u16(&op, 12), dif.src_app_tag_mask);
    assert_eq!(read_u16(&op, 14), dif.src_app_tag_seed);
    assert_eq!(read_u32(&op, 16), dif.dest_ref_tag_seed);
    assert_eq!(read_u16(&op, 20), dif.dest_app_tag_mask);
    assert_eq!(read_u16(&op, 22), dif.dest_app_tag_seed);
}

#[test]
fn cache_flush_sets_address_and_size() {
    let src = [1_u8; 64];
    let mut desc = DsaHwDesc::default();

    desc.fill_cache_flush(src.as_ptr(), 64);

    assert_opcode(&desc, DsaOpcode::CacheFlush);
    assert_flags(
        &desc,
        DsaFlag::RequestCompletionRecord | DsaFlag::CompletionRecordAddressValid,
    );
    assert_eq!(desc.dst_addr(), src.as_ptr() as u64);
    assert_eq!(desc.xfer_size(), 64);
}
