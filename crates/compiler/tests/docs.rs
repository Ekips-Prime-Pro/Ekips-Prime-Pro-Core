//! Checks that the wiki in `docs/` is complete in both languages.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn docs_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs")
}

/// All Markdown files below `dir`, relative to `dir`.
fn markdown_files(dir: &Path) -> BTreeSet<PathBuf> {
    let mut out = BTreeSet::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current).expect("readable docs directory") {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "md") {
                out.insert(path.strip_prefix(dir).expect("prefix").to_path_buf());
            }
        }
    }
    out
}

#[test]
fn every_page_exists_in_english_and_german() {
    let en = markdown_files(&docs_dir().join("en"));
    let de = markdown_files(&docs_dir().join("de"));
    assert!(!en.is_empty(), "docs/en is empty");
    let only_en: Vec<_> = en.difference(&de).collect();
    let only_de: Vec<_> = de.difference(&en).collect();
    assert!(
        only_en.is_empty() && only_de.is_empty(),
        "pages missing a translation: only in en: {only_en:?}, only in de: {only_de:?}"
    );
}

#[test]
fn every_page_is_linked_from_the_index() {
    let index = std::fs::read_to_string(docs_dir().join("README.md")).expect("docs/README.md");
    for page in markdown_files(&docs_dir().join("en")) {
        let page = page.to_string_lossy().replace('\\', "/");
        for lang in ["en", "de"] {
            let link = format!("({lang}/{page})");
            assert!(index.contains(&link), "docs/README.md does not link {link}");
        }
    }
}
