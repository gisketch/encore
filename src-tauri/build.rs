fn main() {
    #[cfg(target_os = "macos")]
    {
        cc::Build::new()
            .file("src/encoder/macos_writer.m")
            .flag("-fobjc-arc")
            .compile("encore_video_writer");
        println!("cargo:rustc-link-lib=framework=AVFoundation");
        println!("cargo:rustc-link-lib=framework=CoreMedia");
        println!("cargo:rustc-link-lib=framework=CoreVideo");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=VideoToolbox");
    }
    tauri_build::build()
}
