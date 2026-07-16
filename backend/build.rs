fn main() {
    // Vendored data refreshes live in scripts/refresh-vendored-data.sh
    // (run by `make refresh-vendored-data` and the release workflow).
    println!("cargo:rerun-if-changed=assets/oui.csv");
    println!("cargo:rerun-if-changed=assets/domain-classification");

    // Windows: delay-load the Npcap runtime (packet.dll) for the `daemon` binary. pnet links
    // Packet.lib, which makes packet.dll a load-time import — so without Npcap installed the exe
    // fails to even start (0xC0000135). Delay-loading defers that load to the first packet-capture
    // call, so the daemon runs on a bare host and only needs Npcap when npcap-based ARP is actually
    // used; the default scan path uses the SendARP fallback and never touches packet.dll.
    // (wpcap.dll is not imported, so it needs no delay-load entry.)
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-arg-bin=daemon=/DELAYLOAD:packet.dll");
        println!("cargo:rustc-link-arg-bin=daemon=delayimp.lib");
    }
}
