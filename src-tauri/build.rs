fn main() {
    println!("cargo:rerun-if-changed=native/signalsmith_wrapper.cpp");
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
    tauri_build::build()
}
