use crate::file_refs::{find_files, is_image_file, read_file_ref};

#[test]
fn find_files_finds_rust_sources() {
    let files = find_files("*.rs", ".", 10);
    assert!(!files.is_empty());
    assert!(files.iter().all(|f| f.ends_with(".rs")));
}

#[test]
fn find_files_respects_limit() {
    let files = find_files("*.rs", ".", 3);
    assert!(files.len() <= 3);
}

#[test]
fn find_files_empty_pattern() {
    let files = find_files("", ".", 10);
    assert!(!files.is_empty());
}

#[test]
fn is_image_file_detects_png() {
    assert!(is_image_file("photo.png"));
    assert!(is_image_file("image.jpg"));
    assert!(is_image_file("pic.jpeg"));
    assert!(is_image_file("anim.gif"));
    assert!(is_image_file("icon.webp"));
}

#[test]
fn is_image_file_rejects_text() {
    assert!(!is_image_file("main.rs"));
    assert!(!is_image_file("README.md"));
}

#[test]
fn read_file_ref_reads_text() {
    let result = read_file_ref("Cargo.toml");
    assert!(result.is_ok());
    let content = result.unwrap();
    assert!(!content.text.is_empty());
    assert!(!content.is_image);
}
