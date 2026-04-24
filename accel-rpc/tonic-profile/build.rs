fn main() {
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("failed to locate vendored protoc");
    unsafe {
        std::env::set_var("PROTOC", protoc);
    }
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .codec_path("crate::custom_codec::ProfileCodec")
        .compile_protos(&["proto/profile.proto"], &["proto"])
        .expect("failed to compile profile proto");
}
