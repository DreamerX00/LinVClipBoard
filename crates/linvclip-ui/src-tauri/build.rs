fn main() {
    // The KLIPY API key is no longer compiled in. The app downloads
    // `gif-provider.json` from the repository at runtime (see src/gif.rs), so a
    // rotated key never requires rebuilding or re-releasing.
    tauri_build::build()
}
