//! Typed access to the Hayagriva bibliography used by validation and rendering.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::string::String;
#[cfg(test)]
use alloc::vec::Vec;
use std::path::Path;

use yaml_rust2::Yaml;
use yaml_rust2::YamlLoader;

use crate::DocError;
use crate::model::CiteKey;

/// The parsed citation register, keyed by the stable corpus cite key.
#[repr(transparent)]
#[derive(Clone, Debug, Default)]
pub struct Bibliography
{
    /// Typed reference records under their stable cite keys.
    entries: BTreeMap<String, Reference>,
}

impl Bibliography
{
    /// Look up one cited reference.
    #[inline]
    #[must_use]
    pub(crate) fn get(
        &self,
        cite: &CiteKey,
    ) -> Option<&Reference>
    {
        self.entries.get(&cite.key)
    }

    /// Copy the stable keys for document-class validation.
    #[inline]
    #[must_use]
    pub(crate) fn key_set(&self) -> BTreeSet<String>
    {
        self.entries.keys().cloned().collect()
    }

    /// Build a minimal key-only bibliography for validation tests.
    #[cfg(test)]
    pub(crate) fn from_keys(keys: Vec<String>) -> Self
    {
        let entries = keys
            .into_iter()
            .map(|key| {
                let title = key.clone();
                (key, Reference {
                    author: None,
                    title,
                    venue: None,
                    date: None,
                    locator: None,
                })
            })
            .collect();
        Self { entries }
    }

    /// Parse an in-memory bibliography fixture.
    #[cfg(test)]
    pub(crate) fn parse_source(source: &str) -> Result<Self, DocError>
    {
        parse(Path::new("mem:refs.yml"), BibliographySource(source))
    }
}

/// Bibliographic fields rendered for one cited work.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Reference
{
    /// Author display text, when the register supplies it.
    author: Option<String>,
    /// Work title.
    title: String,
    /// Proceedings, institution, publisher, or genre display text.
    venue: Option<String>,
    /// Publication year or date display text.
    date: Option<String>,
    /// Preferred resolvable locator, when the register supplies one.
    locator: Option<ReferenceLocator>,
}

impl Reference
{
    /// Author display text, when present.
    #[inline]
    #[must_use]
    pub(crate) const fn author(&self) -> Option<&String>
    {
        self.author.as_ref()
    }

    /// Work title.
    #[inline]
    #[must_use]
    pub(crate) const fn title(&self) -> &String
    {
        &self.title
    }

    /// Venue display text, when present.
    #[inline]
    #[must_use]
    pub(crate) const fn venue(&self) -> Option<&String>
    {
        self.venue.as_ref()
    }

    /// Publication date display text, when present.
    #[inline]
    #[must_use]
    pub(crate) const fn date(&self) -> Option<&String>
    {
        self.date.as_ref()
    }

    /// Preferred resolvable locator, when present.
    #[inline]
    #[must_use]
    pub(crate) const fn locator(&self) -> Option<&ReferenceLocator>
    {
        self.locator.as_ref()
    }
}

/// Preferred outbound locator for a bibliographic entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReferenceLocator
{
    /// Digital Object Identifier.
    Doi(String),
    /// arXiv identifier.
    Arxiv(String),
    /// Stable source URL.
    Url(String),
}

impl ReferenceLocator
{
    /// Render the locator as an absolute hyperlink target.
    #[inline]
    #[must_use]
    pub(crate) fn href(&self) -> String
    {
        match *self {
            | Self::Doi(ref value) => format!("https://doi.org/{value}"),
            | Self::Arxiv(ref value) => format!("https://arxiv.org/abs/{value}"),
            | Self::Url(ref value) => value.clone(),
        }
    }

    /// Render the locator's concise reader-facing label.
    #[inline]
    #[must_use]
    pub(crate) fn label(&self) -> String
    {
        match *self {
            | Self::Doi(ref value) => format!("doi:{value}"),
            | Self::Arxiv(ref value) => format!("arXiv:{value}"),
            | Self::Url(_) => String::from("source"),
        }
    }
}

/// Borrowed source text for a bibliography parse.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct BibliographySource<'source>(&'source str);

/// Fields consumed from each Hayagriva record.
#[derive(Clone, Copy)]
enum BibliographyField
{
    /// Work title.
    Title,
    /// Author display text.
    Author,
    /// Publication date.
    Date,
    /// Parent Hayagriva record.
    Parent,
    /// Institution responsible for the work.
    Organization,
    /// Book publisher.
    Publisher,
    /// Work genre, used as a final venue fallback.
    Genre,
    /// DOI or arXiv identifier map.
    SerialNumber,
    /// Digital Object Identifier.
    Doi,
    /// arXiv identifier.
    Arxiv,
    /// Stable source URL.
    Url,
}

/// Load and type the Hayagriva bibliography.
///
/// # Contract
/// - requires: `path` names a Hayagriva YAML mapping whose string keys identify
///   reference records and whose records carry string titles.
/// - ensures: each string-keyed record is available under the same stable key;
///   locator precedence is DOI, then arXiv, then URL.
/// - provides: one typed bibliography shared by validation and HTML rendering.
/// - fails: returns [`DocError::Io`] for unreadable input and
///   [`DocError::Yaml`] for malformed YAML or a string-keyed record without a
///   string title.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — records distinguishing DOI, arXiv, URL, missing
///   optional metadata, and a missing title distinguish every parse decision.
/// - witness: `bibliography::tests::reference_metadata_and_locator_precedence_are_typed`
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

/// Parse already-read bibliography source into typed reference records.
fn parse(
    path: &Path,
    source: BibliographySource<'_>,
) -> Result<Bibliography, DocError>
{
    let documents = YamlLoader::load_from_str(source.0).map_err(|error| DocError::Yaml {
        path: path.to_path_buf(),
        detail: error.to_string(),
    })?;
    let mut entries = BTreeMap::new();
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
            let key = key.to_owned();
            let reference = parse_reference(path, &key, record)?;
            let _previous = entries.insert(key, reference);
        }
    }
    Ok(Bibliography { entries })
}

/// Type one Hayagriva record and select its display metadata.
fn parse_reference(
    path: &Path,
    key: &String,
    record: &Yaml,
) -> Result<Reference, DocError>
{
    let title = text_field(record, BibliographyField::Title).ok_or_else(|| DocError::Yaml {
        path: path.to_path_buf(),
        detail: format!("reference '{key}' is missing a string title"),
    })?;
    let author = text_field(record, BibliographyField::Author);
    let date = text_field(record, BibliographyField::Date);
    let parent = field(record, BibliographyField::Parent);
    let venue = parent
        .and_then(|value| text_field(value, BibliographyField::Title))
        .or_else(|| text_field(record, BibliographyField::Organization))
        .or_else(|| text_field(record, BibliographyField::Publisher))
        .or_else(|| text_field(record, BibliographyField::Genre));
    let serial_number = field(record, BibliographyField::SerialNumber);
    let locator = serial_number
        .and_then(|value| text_field(value, BibliographyField::Doi))
        .map(ReferenceLocator::Doi)
        .or_else(|| {
            serial_number
                .and_then(|value| text_field(value, BibliographyField::Arxiv))
                .map(ReferenceLocator::Arxiv)
        })
        .or_else(|| text_field(record, BibliographyField::Url).map(ReferenceLocator::Url));
    Ok(Reference {
        author,
        title,
        venue,
        date,
        locator,
    })
}

/// Read one named field from a YAML mapping.
fn field(
    node: &Yaml,
    requested: BibliographyField,
) -> Option<&Yaml>
{
    let name = match requested {
        | BibliographyField::Title => "title",
        | BibliographyField::Author => "author",
        | BibliographyField::Date => "date",
        | BibliographyField::Parent => "parent",
        | BibliographyField::Organization => "organization",
        | BibliographyField::Publisher => "publisher",
        | BibliographyField::Genre => "genre",
        | BibliographyField::SerialNumber => "serial-number",
        | BibliographyField::Doi => "doi",
        | BibliographyField::Arxiv => "arxiv",
        | BibliographyField::Url => "url",
    };
    let mapping = node.as_hash()?;
    mapping
        .iter()
        .find_map(|(key, value)| (key.as_str() == Some(name)).then_some(value))
}

/// Copy a string or integer YAML scalar into display text.
fn text_field(
    node: &Yaml,
    requested: BibliographyField,
) -> Option<String>
{
    let value = field(node, requested)?;
    match *value {
        | Yaml::String(ref text) => Some(text.clone()),
        | Yaml::Integer(number) => Some(number.to_string()),
        | _ => None,
    }
}

#[cfg(test)]
mod tests
{
    use alloc::string::String;

    use super::Bibliography;
    use super::Reference;
    use super::ReferenceLocator;
    use crate::DocError;
    use crate::model::CiteKey;

    /// Reference metadata and DOI/arXiv/URL priority are typed exactly.
    #[test]
    fn reference_metadata_and_locator_precedence_are_typed() -> Result<(), DocError>
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
    arxiv: "2401.00001"
  url: https://example.test/fallback
A:
  type: article
  title: Archive
  serial-number:
    arxiv: "2402.00002"
U:
  type: thesis
  title: Repository Copy
  organization: Example University
  url: https://example.test/thesis
M:
  type: misc
  title: Metadata Minimum
"#,
        )?;
        let doi = bibliography.get(&CiteKey {
            key: "D".to_owned(),
        });
        assert_eq!(
            doi.and_then(Reference::author).map(String::as_str),
            Some("Ada & Bob"),
        );
        assert_eq!(
            doi.and_then(Reference::venue).map(String::as_str),
            Some("Venue"),
        );
        assert_eq!(
            doi.and_then(Reference::date).map(String::as_str),
            Some("2024"),
        );
        assert!(matches!(
            doi.and_then(Reference::locator),
            Some(ReferenceLocator::Doi(value)) if value == "10.1000/example"
        ));

        let arxiv = bibliography.get(&CiteKey {
            key: "A".to_owned(),
        });
        assert!(matches!(
            arxiv.and_then(Reference::locator),
            Some(ReferenceLocator::Arxiv(value)) if value == "2402.00002"
        ));

        let url = bibliography.get(&CiteKey {
            key: "U".to_owned(),
        });
        assert!(matches!(
            url.and_then(Reference::locator),
            Some(ReferenceLocator::Url(value)) if value == "https://example.test/thesis"
        ));

        let minimum = bibliography.get(&CiteKey {
            key: "M".to_owned(),
        });
        assert!(minimum.and_then(Reference::author).is_none());
        assert!(minimum.and_then(Reference::venue).is_none());
        assert!(minimum.and_then(Reference::date).is_none());
        assert!(minimum.and_then(Reference::locator).is_none());
        Ok(())
    }

    /// A string-keyed record without a title is rejected.
    #[test]
    fn missing_title_is_rejected()
    {
        let result = Bibliography::parse_source("BROKEN:\n  type: article\n  author: Nobody\n");
        assert!(matches!(
            result,
            Err(DocError::Yaml { ref detail, .. })
                if detail == "reference 'BROKEN' is missing a string title"
        ));
    }
}
