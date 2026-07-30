//! The lexicon freshness gate: the committed `GF` lexicon modules match a
//! regeneration over the `.gfd` corpus (the derived-file pattern — regenerate,
//! never hand-edit).

use core::error::Error;
use std::path::Path;

use crate::common::repo_root;

/// Shared result type for the lexicon witnesses.
type TestResult<T = ()> = Result<T, Box<dyn Error>>;

#[cfg(test)]
mod tests
{
    use super::*;

    /// The lexicon generated from the `.gfd` corpus agrees with the committed
    /// modules (the freshness gate).
    #[test]
    fn generated_lexicon_is_fresh() -> TestResult
    {
        let root = repo_root();
        let pgf = root.join("target/gf/GandrDocsLex.pgf");
        if !pgf.exists() {
            eprintln!("skip: GF environment unprovisioned");
            return Ok(());
        }
        let runtime = gandr_workflow_grammatical_framework::rt::PyPgf::new(
            &pgf,
            &gandr_workflow_grammatical_framework::rt::LanguageName::new("GandrDocsLexHtml"),
        )?;
        let corpus_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
        let refs = root.join("docs/spec/refs.yml");
        let lexicon = gandr_workflow_docs::lexicon::generate(&runtime, &corpus_dir, &refs)?;
        let grammar = Path::new(env!("CARGO_MANIFEST_DIR")).join("grammar");
        for (name, rendered) in [
            ("GandrDocsLex.gf", lexicon.render_abstract()),
            ("GandrDocsLexHtml.gf", lexicon.render_concrete()),
        ] {
            let committed = std::fs::read_to_string(grammar.join(name))?;
            assert_eq!(committed, rendered, "{name} is stale; run the lexicon lane");
        }
        Ok(())
    }
}
