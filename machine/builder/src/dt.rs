use std::{
    path::{Path, PathBuf},
    process::Command,
};

// Device Tree ----------------------------------------------------------------

pub fn check_dts() -> Option<PathBuf> {
    let dts = std::env::var("DTS_PATH").ok()?;
    let dts_path = Path::new(&dts);
    println!("cargo::rerun-if-changed={}", dts_path.display());
    Some(dts_path.to_path_buf())
}

/// Returns the compatible (vendor, name) tuple for the SoC node in the device tree.
pub fn soc(dt: &dtgen::ir::DeviceTree) -> Vec<(&str, &str)> {
    let soc_node = dt
        .nodes
        .iter()
        .find(|n| n.name == "soc")
        .expect("Device tree must have a soc node");

    soc_node
        .compatible
        .iter()
        .filter_map(|s| {
            let parts: Vec<&str> = s.split(',').collect();
            if parts.len() == 2 {
                Some((parts[0], parts[1]))
            } else {
                None
            }
        })
        .collect()
}

pub fn generate_device_tree(
    dt: &dtgen::ir::DeviceTree,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let rust = dtgen::generate_rust(dt);
    std::fs::write(out.join("device_tree.rs"), rust)?;

    let ld = dtgen::generate_ld(dt).map_err(|e| format!("linker script generation failed: {e}"))?;
    std::fs::write(out.join("prelude.ld"), ld)?;
    println!("cargo::rustc-link-search=native={}", out.display());
    Ok(())
}

pub fn build_device_tree(dts: &Path) -> Result<dtgen::ir::DeviceTree, Box<dyn std::error::Error>> {
    let include_paths = include_paths()?;
    let include_refs: Vec<&Path> = include_paths.iter().map(PathBuf::as_path).collect();

    for path in &include_paths {
        if !path.exists() {
            println!("cargo:warning=MISSING INCLUDE PATH: {:?}", path);
        }
    }

    Ok(dtgen::parse_dts(&dts, &include_refs)?)
}

fn include_paths() -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-env-changed=ZEPHYR_BASE");
    println!("cargo::rerun-if-env-changed=ZEPHYR_MODULES");
    println!("cargo::rerun-if-env-changed=OUT_DIR");

    let out = PathBuf::from(std::env::var("OUT_DIR")?);
    let zephyr = zephyr_base(&out)?;
    let mut paths = vec![
        zephyr.join("include"),
        zephyr.join("dts"),
        zephyr.join("dts/common"),
        zephyr.join("dts/arm"),
        zephyr.join("boards"),
    ];

    for module in zephyr_modules(&out)? {
        paths.push(module.join("dts"));
    }

    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn zephyr_base(out: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(path) = std::env::var("ZEPHYR_BASE") {
        return Ok(PathBuf::from(path));
    }

    const ZEPHYR_REVISION: &str = "v4.3.0";
    const ZEPHYR_URL: &str = "https://github.com/zephyrproject-rtos/zephyr";

    let path = out.join("zephyr");
    clone_if_missing(
        ZEPHYR_URL,
        ZEPHYR_REVISION,
        &path,
        &["include", "dts", "boards"],
    )?;
    Ok(path)
}

fn zephyr_modules(out: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    if let Ok(modules) = std::env::var("ZEPHYR_MODULES") {
        return Ok(std::env::split_paths(&modules).collect());
    }

    // TODO: Make generic.
    const HAL_STM32_REVISION: &str = "286dd285b5bb4fddafdfff27b5405264e5a61bfe";
    const HAL_STM32_URL: &str = "https://github.com/zephyrproject-rtos/hal_stm32";

    let hal_stm32 = out.join("hal_stm32");
    clone_if_missing(HAL_STM32_URL, HAL_STM32_REVISION, &hal_stm32, &["dts"])?;
    Ok(vec![hal_stm32])
}

fn clone_if_missing(
    url: &str,
    revision: &str,
    path: &Path,
    sparse_paths: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    if path.exists() {
        return Ok(());
    }

    run(Command::new("git").args(["init"]).arg(path))?;
    run(Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["remote", "add", "origin", url]))?;
    run(Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["sparse-checkout", "init", "--cone"]))?;

    let mut sparse_checkout = Command::new("git");
    sparse_checkout
        .arg("-C")
        .arg(path)
        .args(["sparse-checkout", "set"])
        .args(sparse_paths);
    run(&mut sparse_checkout)?;

    run(Command::new("git").arg("-C").arg(path).args([
        "fetch",
        "--depth=1",
        "--filter=blob:none",
        "origin",
        revision,
    ]))?;
    run(Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["checkout", "--detach", "FETCH_HEAD"]))?;
    Ok(())
}

fn run(command: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command failed with status {status}").into())
    }
}
