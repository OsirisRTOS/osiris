fn main() {
    println!("cargo::rerun-if-env-changed=OSIRIS_DEBUG_METRICS");
    if std::env::var("OSIRIS_DEBUG_METRICS").map_or(false, |v| v == "true" || v == "1") {
        println!("cargo::rustc-cfg=osiris_metrics");
    }
}
