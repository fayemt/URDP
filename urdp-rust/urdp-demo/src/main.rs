use clap::{Arg, Command};
use urdp_control::{CodexDescriptor, CodexOffer, to_cbor};

fn main() {
    let matches = Command::new("urdp-demo")
        .about("URDP demo tool")
        .arg(
            Arg::new("list")
                .long("list")
                .help("List codex descriptors"),
        )
        .get_matches();

    if matches.contains_id("list") {
        // Print a sample codex descriptor in CBOR form
        let desc = CodexDescriptor {
            codex_id: [0u8; 32],
            name: "Example Codex".into(),
            semver: "v0.1".into(),
            vendor_id: "example".into(),
            domains: vec!["text/plain".into()],
            exp_bpb: Some(100),
            exp_decode_us: Some(500),
            pack_size_bytes: Some(1024),
        };
        let offer = CodexOffer {
            offer_id: 1,
            codex_list: vec![desc],
        };
        let cbor = to_cbor(&offer).unwrap();
        println!("CBOR (hex): {}", hex::encode(cbor));
    }
}
