use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "macos" {
        let stub = root.join("native_stubs.c");
        println!("cargo:rerun-if-changed={}", stub.display());
        cc::Build::new()
            .file(&stub)
            .flag_if_supported("-Wall")
            .flag_if_supported("-Wextra")
            .compile("xcw_native_bridge");
        return;
    }

    let cli = root.join("native");
    let native = cli.join("bridge");

    let files = [
        cli.join("DFPrivateSimulatorDisplayBridge.m"),
        cli.join("XCWProcessRunner.m"),
        cli.join("XCWPrivateSimulatorBooter.m"),
        cli.join("XCWAccessibilityBridge.m"),
        cli.join("XCWSimctl.m"),
        native.join("XCWNativeBridge.m"),
    ];

    let mut build = cc::Build::new();
    build
        .files(files.iter())
        .include(&cli)
        .include(&native)
        .flag("-fobjc-arc")
        .flag("-fmodules")
        .flag_if_supported("-Wall")
        .flag_if_supported("-Wextra");

    for file in &files {
        println!("cargo:rerun-if-changed={}", file.display());
    }
    for header in [
        cli.join("DFPrivateSimulatorDisplayBridge.h"),
        cli.join("XCWPrivateSimulatorBooter.h"),
        cli.join("XCWAccessibilityBridge.h"),
        cli.join("XCWSimctl.h"),
        native.join("XCWNativeBridge.h"),
    ] {
        println!("cargo:rerun-if-changed={}", header.display());
    }

    build.compile("xcw_native_bridge");

    for framework in [
        "Foundation",
        "Accelerate",
        "AppKit",
        "CoreImage",
        "CoreGraphics",
        "CoreVideo",
        "ImageIO",
        "QuartzCore",
    ] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}
