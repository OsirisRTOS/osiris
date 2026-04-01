mod codegen;
mod ir;
mod parser;

use std::path::Path;

pub fn run(dts_path: &Path, include_dirs: &[&Path], out_path: &Path) -> Result<(), String> {
    let dtb = parser::dts_to_dtb(dts_path, include_dirs)?;
    let dt = parser::dtb_to_devicetree(&dtb)?;
    let src = codegen::generate_rust(&dt);
    std::fs::write(out_path, src)
        .map_err(|e| format!("dtgen: failed to write {}: {e}", out_path.display()))
}
