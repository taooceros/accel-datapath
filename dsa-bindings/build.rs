use std::env;
use std::path::PathBuf;

fn main() {
    let header = env::var("IDXD_HEADER").unwrap_or_else(|_| "/usr/include/linux/idxd.h".into());

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=IDXD_HEADER");
    println!("cargo:rerun-if-env-changed=BINDGEN_EXTRA_CLANG_ARGS");
    println!("cargo:rerun-if-changed={header}");

    let out_path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let bindings = bindgen::Builder::default()
        .header(header)
        .allowlist_type("iax_hw_desc")
        .allowlist_type("iax_completion_record")
        .allowlist_type("iax_raw_desc")
        .allowlist_type("iax_raw_completion_record")
        .allowlist_type("iax_opcode")
        .allowlist_type("iax_completion_status")
        .allowlist_var("IDXD_OP_FLAG_.*")
        .rustified_enum("iax_opcode")
        .rustified_enum("iax_completion_status")
        .layout_tests(false)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("failed to generate idxd IAX bindings from linux/idxd.h");

    bindings
        .write_to_file(out_path.join("idxd_iax_bindings.rs"))
        .expect("failed to write idxd IAX bindings");
}
