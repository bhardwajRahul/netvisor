#[cfg(test)]
mod dependency_tests {
    #[test]
    fn ensure_no_openssl_dependencies() {
        // This test ensures we don't accidentally pull in OpenSSL
        // Run: cargo tree -i openssl-sys --target all

        let output = std::process::Command::new("cargo")
            .args(["tree", "-i", "openssl-sys", "--target", "all"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("Failed to run cargo tree");

        let stdout = String::from_utf8_lossy(&output.stdout);

        // If openssl-sys is not in the dependency tree, cargo tree returns empty with a warning
        // If it IS in the tree, it will show the dependency chain
        assert!(
            stdout.contains("warning: nothing to print") || stdout.is_empty(),
            "Found OpenSSL dependencies! This breaks musl cross-compilation.\n\
             Run 'cargo tree -i openssl-sys --target all' to see the dependency chain.\n\
             Make sure all dependencies use 'default-features = false' with 'rustls-tls' features.\n\
             Output:\n{}",
            stdout
        );
    }

    #[test]
    fn test_no_aws_lc_rs_dependency() {
        // Ensures aws-lc-rs is NOT in the dependency tree.
        //
        // Most dependencies use `ring` as the rustls crypto provider. If any dependency
        // adds `aws-lc-rs`, rustls 0.23+ will have both providers available and cannot
        // auto-determine which to use, causing a runtime panic:
        //   "Could not automatically determine the process-level CryptoProvider"
        //
        // Known culprit: metrics-exporter-prometheus's `push-gateway` feature explicitly
        // enables `hyper-rustls/aws-lc-rs`. Fix by disabling default features.
        let output = std::process::Command::new("cargo")
            .args(["tree", "-i", "aws-lc-rs", "--target", "all"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("Failed to run cargo tree");

        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(
            stdout.contains("warning: nothing to print") || stdout.is_empty(),
            "Found aws-lc-rs in dependencies! This causes rustls crypto provider conflicts.\n\
             Run 'cargo tree -i aws-lc-rs --target all' to see the dependency chain.\n\
             Output:\n{}",
            stdout
        );
    }
}
