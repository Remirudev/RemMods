use std::path::PathBuf;
use std::process::ExitCode;

use specta_typescript::Typescript;

const HEADER: &str = "// @ts-nocheck\n/* eslint-disable */";

fn main() -> ExitCode {
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("src")
        .join("lib")
        .join("ipc")
        .join("bindings.ts");
    let exporter = Typescript::default().header(HEADER);
    match remmods_lib::specta::builder().export(exporter, &target) {
        Ok(()) => {
            println!("wrote {}", target.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("exporting bindings to {} failed: {error}", target.display());
            ExitCode::FAILURE
        }
    }
}
