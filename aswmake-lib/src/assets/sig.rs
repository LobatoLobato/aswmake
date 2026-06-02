use std::sync::LazyLock;

pub static SIG_FILE: LazyLock<tempfile::NamedTempFile> = LazyLock::new(|| {
    let sig = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/pak.sig"));
    let sig_file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(sig_file.path(), sig).unwrap(); 
    
    sig_file
});
