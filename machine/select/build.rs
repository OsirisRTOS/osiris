

fn main() {
    // Pass linker script to top level
    if let Ok(linker_script) = std::env::var("DEP_HALARM_LINKER_SCRIPT") {
        println!("cargo:linker-script={linker_script}");
    } else {
        println!("cargo:warning=LD_SCRIPT_PATH environment variable not set.");
    }
}