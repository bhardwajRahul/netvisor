fn main() {
    // Vendored data refreshes live in scripts/refresh-vendored-data.sh
    // (run by `make refresh-vendored-data` and the release workflow).
    println!("cargo:rerun-if-changed=assets/oui.csv");
    println!("cargo:rerun-if-changed=assets/domain-classification");
}
