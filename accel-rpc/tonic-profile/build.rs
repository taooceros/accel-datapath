fn main() {
    let manifest_dir = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by Cargo"),
    );
    let proto_file = manifest_dir.join("proto/profile.proto");
    let proto_dir = manifest_dir.join("proto");
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("failed to locate vendored protoc");
    unsafe {
        std::env::set_var("PROTOC", protoc);
    }
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .codec_path("crate::custom_codec::ProfileCodec")
        .compile_protos(&[proto_file], &[proto_dir])
        .expect("failed to compile profile proto");
}
