#![cfg_attr(target_os = "none", no_std)]
#![cfg(not(target_os = "none"))]

mod codegen;
pub mod ir;
mod parser;
mod ldgen;

use std::path::Path;

use crate::ir::DeviceTree;

pub fn parse_dts(dts_path: &Path, include_dirs: &[&Path]) -> Result<DeviceTree, String> {
    let dtb = parser::dts_to_dtb(dts_path, include_dirs)?;
    parser::dtb_to_devicetree(&dtb)
}

pub fn generate_rust(dt: &DeviceTree) -> String {
    codegen::generate_rust(dt)
}

pub fn generate_ld(dt: &DeviceTree) -> Result<String, String> {
    ldgen::generate_ld(dt)
}
