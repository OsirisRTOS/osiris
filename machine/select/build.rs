use cfg_aliases::cfg_aliases;

fn main() {
    // Pass linker script to top level
    if let Ok(linker_script) = std::env::var("DEP_HALARM_LINKER_SCRIPT") {
        println!("cargo:linker-script={linker_script}");
    } else {
        println!("cargo:warning=LD_SCRIPT_PATH environment variable not set.");
    }

    cfg_aliases! {
        freestanding: { all(not(test), not(doctest), not(doc), not(kani), any(target_os = "none", target_os = "unknown")) },
    }
}
