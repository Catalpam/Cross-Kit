#![cfg_attr(not(feature = "binding-generator"), allow(dead_code))]

use std::env;

#[cfg(feature = "binding-generator")]
use camino::Utf8PathBuf;
#[cfg(feature = "binding-generator")]
use uniffi_bindgen::EmptyCrateConfigSupplier;
#[cfg(feature = "binding-generator")]
use uniffi_bindgen::bindings::KotlinBindingGenerator;
#[cfg(feature = "binding-generator")]
use uniffi_bindgen::library_mode;

#[cfg(feature = "binding-generator")]
fn main() {
    let mut args = env::args().skip(1);
    let Some(lib_path) = args.next() else {
        eprintln!(
            "usage: cargo run --features binding-generator --bin uniffi-bindgen -- <library-path> <out-dir>"
        );
        std::process::exit(1);
    };
    let Some(out_dir) = args.next() else {
        eprintln!(
            "usage: cargo run --features binding-generator --bin uniffi-bindgen -- <library-path> <out-dir>"
        );
        std::process::exit(1);
    };

    let lib_path = Utf8PathBuf::from(lib_path);
    let out_dir = Utf8PathBuf::from(out_dir);

    library_mode::generate_bindings(
        lib_path.as_ref(),
        None,
        &KotlinBindingGenerator,
        &EmptyCrateConfigSupplier,
        None,
        out_dir.as_ref(),
        false,
    )
    .expect("failed to generate Kotlin bindings");
}

#[cfg(not(feature = "binding-generator"))]
fn main() {
    eprintln!("Enable the `binding-generator` feature to run this helper binary.");
    std::process::exit(1);
}
