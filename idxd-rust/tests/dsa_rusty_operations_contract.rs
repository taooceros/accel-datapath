use idxd_rust::{
    DsaDifCheck, DsaDifInsert, DsaDifUpdate, DsaFlag, DsaHwDesc, DsaOpcode,
    default_completion_flags,
};

fn assert_opcode(desc: &DsaHwDesc, opcode: DsaOpcode) {
    assert_eq!(desc.opcode(), opcode.as_u8());
}

fn assert_default_completion(desc: &DsaHwDesc) {
    assert_eq!(desc.flags(), default_completion_flags().bits());
}

#[test]
fn rusty_dsa_operation_helpers_fill_all_supported_opcodes() {
    let flags = DsaFlag::RequestCompletionRecord | DsaFlag::CompletionRecordAddressValid;

    let mut desc = DsaHwDesc::default();
    desc.noop(flags);
    assert_opcode(&desc, DsaOpcode::Noop);
    assert_eq!(desc.flags(), flags.bits());

    let sub_desc = [DsaHwDesc::default(), DsaHwDesc::default()];
    desc.batch(&sub_desc, flags);
    assert_opcode(&desc, DsaOpcode::Batch);
    assert_eq!(desc.src_addr(), sub_desc.as_ptr() as u64);
    assert_eq!(desc.desc_count(), sub_desc.len() as u32);

    desc.drain(flags);
    assert_opcode(&desc, DsaOpcode::Drain);
    assert_eq!(desc.flags(), flags.bits());

    let src = [0x11_u8; 64];
    let src2 = [0x22_u8; 64];
    let mut dst = [0_u8; 128];
    let mut dst2 = [0_u8; 128];
    let pattern = 0x1111_1111_1111_1111;

    desc.memmove(&src, &mut dst);
    assert_opcode(&desc, DsaOpcode::Memmove);
    assert_default_completion(&desc);
    assert_eq!(desc.src_addr(), src.as_ptr() as u64);
    assert_eq!(desc.dst_addr(), dst.as_mut_ptr() as u64);
    assert_eq!(desc.xfer_size(), src.len() as u32);

    desc.memfill(pattern, &mut dst);
    assert_opcode(&desc, DsaOpcode::Memfill);
    assert_eq!(desc.src_addr(), pattern);
    assert_eq!(desc.xfer_size(), dst.len() as u32);

    desc.compare(&src, &src2);
    assert_opcode(&desc, DsaOpcode::Compare);
    assert_eq!(desc.xfer_size(), src.len() as u32);

    desc.compare_value(&src, pattern);
    assert_opcode(&desc, DsaOpcode::CompareValue);
    assert_eq!(desc.dst_addr(), pattern);

    let mut delta = [0_u8; 256];
    desc.create_delta(&src, &src2, &mut delta, 0xff);
    assert_opcode(&desc, DsaOpcode::CreateDelta);
    assert_eq!(desc.xfer_size(), src.len() as u32);

    desc.apply_delta(&delta, &mut dst);
    assert_opcode(&desc, DsaOpcode::ApplyDelta);

    desc.dualcast(&src, &mut dst, &mut dst2);
    assert_opcode(&desc, DsaOpcode::Dualcast);
    assert_eq!(desc.xfer_size(), src.len() as u32);

    desc.crc_gen(&src, 0x1234_5678);
    assert_opcode(&desc, DsaOpcode::CrcGen);
    assert_eq!(desc.xfer_size(), src.len() as u32);

    desc.copy_crc(&src, &mut dst, 0x1234_5678);
    assert_opcode(&desc, DsaOpcode::CopyCrc);
    assert_eq!(desc.xfer_size(), src.len() as u32);

    desc.dif_check(&src, DsaDifCheck::default());
    assert_opcode(&desc, DsaOpcode::DifCheck);

    desc.dif_insert(&src, &mut dst, DsaDifInsert::default());
    assert_opcode(&desc, DsaOpcode::DifInsert);

    desc.dif_strip(&src, &mut dst, DsaDifCheck::default());
    assert_opcode(&desc, DsaOpcode::DifStrip);

    desc.dif_update(&src, &mut dst, DsaDifUpdate::default());
    assert_opcode(&desc, DsaOpcode::DifUpdate);

    desc.cache_flush(&src);
    assert_opcode(&desc, DsaOpcode::CacheFlush);
    assert_eq!(desc.dst_addr(), src.as_ptr() as u64);
    assert_eq!(desc.xfer_size(), src.len() as u32);
}
