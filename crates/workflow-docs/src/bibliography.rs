//! Typed access to the Hayagriva bibliography read for citation resolution.

use alloc::collections::BTreeSet;
use alloc::string::String;
use std::path::Path;

use yaml_rust2::Yaml;
use yaml_rust2::YamlLoader;

use crate::DocError;

/// The parsed citation register: the stable keys a `cite` may resolve to.
#[repr(transparent)]
#[derive(Clone, Debug, Default)]
pub struct Bibliography
{
    /// Stable cite keys, one per well-formed register record.
    keys: BTreeSet<String>,
}

impl Bibliography
{
    /// Copy the stable keys for document-class validation.
    #[inline]
    #[must_use]
    pub fn key_set(&self) -> BTreeSet<String>
    {
        self.keys.clone()
    }

    /// Parse an in-memory bibliography fixture.
    #[cfg(test)]
    pub(crate) fn parse_source(source: BibliographySource<'_>) -> Result<Self, DocError>
    {
        parse(Path::new("mem:bibliography.yml"), source)
    }
}

/// Borrowed source text for a bibliography parse.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub(crate) struct BibliographySource<'source>(&'source str);

impl<'source> From<&'source str> for BibliographySource<'source>
{
    #[inline]
    fn from(source: &'source str) -> Self
    {
        Self(source)
    }
}

/// Load and type the Hayagriva bibliography.
///
/// # Contract
/// - requires: `path` names a Hayagriva YAML mapping whose string keys identify
///   reference records and whose records carry string titles.
/// - ensures: each string-keyed record contributes its key to the register.
/// - provides: the resolvable key set the document-class validator cites
///   against.
/// - fails: returns [`DocError::Io`] for unreadable input and
///   [`DocError::Yaml`] for malformed YAML or a string-keyed record without a
///   string title.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — records with and without a title distinguish
///   every parse decision.
/// - witness: `bibliography::tests::well_formed_records_contribute_their_keys`
/// - witness: `bibliography::tests::missing_title_is_rejected`
///
/// # Errors
/// Returns [`DocError`] when the file cannot be read or typed as a
/// bibliography.
#[inline]
pub fn load(path: &Path) -> Result<Bibliography, DocError>
{
    let text = std::fs::read_to_string(path).map_err(|source| DocError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let bibliography = parse(path, BibliographySource(&text))?;
    Ok(bibliography)
}

/// Parse already-read bibliography source into the stable key set.
fn parse(
    path: &Path,
    source: BibliographySource<'_>,
) -> Result<Bibliography, DocError>
{
    let documents = YamlLoader::load_from_str(source.0).map_err(|error| DocError::Yaml {
        path: path.to_path_buf(),
        detail: error.to_string(),
    })?;
    let mut keys = BTreeSet::new();
    for document in &documents {
        let Some(mapping) = document.as_hash()
        else {
            continue;
        };
        for (raw_key, record) in mapping {
            let Some(key) = raw_key.as_str()
            else {
                continue;
            };
            if title(record).is_none() {
                return Err(DocError::Yaml {
                    path: path.to_path_buf(),
                    detail: format!("reference '{key}' is missing a string title"),
                });
            }
            let _fresh = keys.insert(key.to_owned());
        }
    }
    Ok(Bibliography { keys })
}

/// Copy a record's title when it is a string or integer scalar.
fn title(record: &Yaml) -> Option<String>
{
    let mapping = record.as_hash()?;
    let value = mapping
        .iter()
        .find_map(|(key, value)| (key.as_str() == Some("title")).then_some(value))?;
    match *value {
        | Yaml::String(ref text) => Some(text.clone()),
        | Yaml::Integer(number) => Some(number.to_string()),
        | _ => None,
    }
}

#[cfg(test)]
mod tests
{
    use super::Bibliography;
    use crate::DocError;

    /// Every well-formed record contributes its stable key, whatever metadata
    /// it carries beyond the title.
    #[test]
    fn well_formed_records_contribute_their_keys() -> Result<(), DocError>
    {
        let bibliography = Bibliography::parse_source(
            r#"D:
  type: article
  title: "Typed & Linked"
  author: Ada & Bob
  date: 2024
  parent:
    type: proceedings
    title: Venue
  serial-number:
    doi: 10.1000/example
M:
  type: misc
  title: Metadata Minimum
"#
            .into(),
        )?;
        let keys = bibliography.key_set();
        assert!(keys.contains("D"));
        assert!(keys.contains("M"));
        assert_eq!(keys.len(), 2);
        Ok(())
    }

    /// A string-keyed record without a title is rejected.
    #[test]
    fn missing_title_is_rejected()
    {
        let result =
            Bibliography::parse_source("BROKEN:\n  type: article\n  author: Nobody\n".into());
        assert!(matches!(
            result,
            Err(DocError::Yaml { ref detail, .. })
                if detail == "reference 'BROKEN' is missing a string title"
        ));
    }
}
