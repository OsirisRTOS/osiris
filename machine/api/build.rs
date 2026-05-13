fn main() {
    println!("cargo::rerun-if-env-changed=METRICS");
    if std::env::var("METRICS").map_or(false, |v| v == "true" || v == "1") {
        println!("cargo::rustc-cfg=metrics");
    }
}
