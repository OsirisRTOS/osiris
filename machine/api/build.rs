fn main() {
    println!("cargo::rerun-if-env-changed=OSIRIS_METRICS");
    if std::env::var("OSIRIS_METRICS").map_or(false, |v| v == "true" || v == "1") {
        println!("cargo::rustc-cfg=metrics");
    }
}
