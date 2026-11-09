use std::{env, path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-env-changed=CUDA_HOME");
    println!("cargo:rerun-if-changed=native");
    if env::var_os("CARGO_FEATURE_CUDA").is_none() {
        return;
    }
    let toolkit = env::var("CUDA_HOME").unwrap_or_else(|_| "/usr/local/cuda".into());
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("native");
    let ok = Command::new("cmake")
        .args(["-S", "native", "-B"])
        .arg(&output)
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .arg(format!("-DCMAKE_CUDA_COMPILER={toolkit}/bin/nvcc"))
        .status()
        .expect("CMake is required for the CUDA feature");
    assert!(ok.success(), "CUDA C++ configuration failed");
    assert!(
        Command::new("cmake")
            .arg("--build")
            .arg(&output)
            .args(["--parallel", "2"])
            .status()
            .unwrap()
            .success(),
        "CUDA C++ build failed"
    );
    println!("cargo:rustc-link-search=native={}", output.display());
    println!("cargo:rustc-link-search=native={toolkit}/lib64");
    for lib in [
        "static=claw_lm_cuda",
        "dylib=cudart",
        "dylib=cublas",
        "dylib=stdc++",
    ] {
        println!("cargo:rustc-link-lib={lib}");
    }
    println!("cargo:rustc-link-arg=-Wl,-rpath,{toolkit}/lib64");
}
