fn main() {
    println!("cargo:rerun-if-changed=native/signalsmith_wrapper.cpp");
    println!("cargo:rerun-if-changed=native/world_wrapper.cpp");
    println!("cargo:rerun-if-changed=vendor/signalsmith/include/signalsmith-stretch.h");
    println!("cargo:rerun-if-changed=vendor/signalsmith/include/signalsmith-linear/fft.h");
    println!("cargo:rerun-if-changed=vendor/signalsmith/include/signalsmith-linear/stft.h");
    cc::Build::new()
        .cpp(true)
        .std("c++14")
        .warnings(false)
        .include("vendor/signalsmith/include")
        .file("native/signalsmith_wrapper.cpp")
        .compile("mam-signalsmith-stretch");
    let world_sources = [
        "vendor/world/src/cheaptrick.cpp",
        "vendor/world/src/common.cpp",
        "vendor/world/src/d4c.cpp",
        "vendor/world/src/fft.cpp",
        "vendor/world/src/harvest.cpp",
        "vendor/world/src/matlabfunctions.cpp",
        "vendor/world/src/synthesis.cpp",
    ];
    for source in world_sources {
        println!("cargo:rerun-if-changed={source}");
    }
    println!("cargo:rerun-if-changed=vendor/world/src/world");
    cc::Build::new()
        .cpp(true)
        .std("c++14")
        .warnings(false)
        .include("vendor/world/src")
        .files(world_sources)
        .file("native/world_wrapper.cpp")
        .compile("mam-world-reference");
    tauri_build::build()
}
