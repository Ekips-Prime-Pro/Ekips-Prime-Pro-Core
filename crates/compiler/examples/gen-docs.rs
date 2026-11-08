//! Regenerates `docs/en/stdlib.md` and `docs/de/stdlib.md` from the
//! standard library table.
//!
//! ```text
//! cargo run -p ekips-compiler --example gen-docs
//! ```

use ekips_compiler::Lang;
use ekips_compiler::stdlib::reference_markdown;

fn main() -> std::io::Result<()> {
    let docs = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
    for (lang, dir) in [(Lang::En, "en"), (Lang::De, "de")] {
        let path = docs.join(dir).join("stdlib.md");
        std::fs::write(&path, reference_markdown(lang))?;
        println!("wrote {}", path.display());
    }
    Ok(())
}
