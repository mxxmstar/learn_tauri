fn main() {
    tauri_build::build();

    cc::Build::new()
        .files(&[
            "native/bcm_common.c",
            "native/rpc_connect.c",
            "native/bcm_dmon.c",
            "native/bcm_config.c",
        ])
        .include("native/include")
        .compile("bcmcore");

    println!("cargo:rerun-if-changed=native/");
}
