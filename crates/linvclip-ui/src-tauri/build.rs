fn main() {
    // ── Embed obfuscated KLIPY API key at compile time ──
    //
    // Key sources, in order of precedence:
    //   1. `KLIPY_API_KEY` environment variable — what CI provides (GitHub
    //      Actions secret, see CONTRIBUTING.md).
    //   2. `klipy.key` next to this file (gitignored) — local-dev fallback.
    //
    // The bytes are XOR-scrambled so the key never appears as plaintext in
    // the binary or in version control. When neither source yields a key the
    // build still succeeds (GIF search is simply disabled at runtime and the
    // UI shows a friendly message), but we warn loudly so a keyless release
    // artifact is never produced by accident.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let key_path = std::path::Path::new(&manifest_dir).join("klipy.key");

    println!("cargo:rerun-if-env-changed=KLIPY_API_KEY");
    println!("cargo:rerun-if-changed={}", key_path.display());

    let from_env = std::env::var("KLIPY_API_KEY")
        .ok()
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty());

    let (raw_key, source) = match from_env {
        Some(k) => (k, "KLIPY_API_KEY env var"),
        None if key_path.exists() => (
            std::fs::read_to_string(&key_path)
                .unwrap_or_default()
                .trim()
                .to_string(),
            "klipy.key",
        ),
        None => (String::new(), "none"),
    };

    if raw_key.is_empty() {
        println!(
            "cargo:warning=KLIPY API key not found (KLIPY_API_KEY env var unset and {} missing or empty). \
             GIF search will be disabled in this build. See CONTRIBUTING.md.",
            key_path.display()
        );
    } else {
        // Never print the key itself — only where it came from.
        println!("cargo:warning=KLIPY API key embedded from {}.", source);
    }

    let xor_pad: &[u8] = b"LvCb2026xKm9";
    let obfuscated: Vec<u8> = raw_key
        .bytes()
        .enumerate()
        .map(|(i, b)| b ^ xor_pad[i % xor_pad.len()])
        .collect();

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest = std::path::Path::new(&out_dir).join("klipy_key.rs");
    std::fs::write(
        &dest,
        format!(
            "const KLIPY_KEY_XOR_PAD: &[u8] = b\"LvCb2026xKm9\";\nconst KLIPY_KEY_BYTES: &[u8] = &{:?};\n",
            obfuscated
        ),
    )
    .expect("Failed to write klipy_key.rs");

    tauri_build::build()
}
