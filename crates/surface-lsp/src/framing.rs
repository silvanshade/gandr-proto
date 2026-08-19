//! Content-Length framing for the LSP base protocol.
//!
//! One message is an ASCII header block (`Name: value` lines, terminated by
//! an extra `\r\n`) followed by `Content-Length` bytes of UTF-8 JSON.

use alloc::string::String;
use std::io::BufRead;
use std::io::Write;

use crate::boundary::ContentLength;
use crate::boundary::FrameBytes;
use crate::boundary::FramePayload;
use crate::boundary::HeaderText;

/// Defensive upper bound on a message's content length (64 MiB).
const MAX_CONTENT_LENGTH: usize = 0x0400_0000;

/// A framing-layer failure.
///
/// Framing errors desynchronize the stream. Drivers treat them as fatal for
/// the connection. JSON content that fails to parse is not a framing error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FramingError
{
    /// The stream ended in the middle of a message.
    #[error("unexpected end of stream")]
    UnexpectedEof,
    /// A header line was missing its colon.
    #[error("malformed header")]
    MalformedHeader,
    /// No Content-Length header was present.
    #[error("missing Content-Length")]
    MissingContentLength,
    /// Content-Length was not a positive integer inside the cap.
    #[error("invalid Content-Length")]
    InvalidContentLength,
    /// An underlying IO failure.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Reads one framed message, returning its content bytes.
///
/// # Contract
/// - ensures: `Ok(Some(bytes))` holds exactly the declared content part;
///   `Ok(None)` reports a clean end of stream at a message boundary.
/// - fails: [`FramingError`] on malformed headers, a missing, invalid, or
///   oversized `Content-Length`, an EOF inside a message, or an IO failure.
/// - panics: none.
///
/// # Errors
///
/// See the contract's fails clause.
#[inline]
pub fn read_message<R>(reader: &mut R) -> Result<Option<FrameBytes>, FramingError>
where
    R: BufRead,
{
    let mut content_length = None;
    let mut saw_header = false;
    loop {
        let mut line = String::new();
        let count = reader.read_line(&mut line)?;
        if count == 0 {
            if saw_header {
                return Err(FramingError::UnexpectedEof);
            }
            return Ok(None);
        }
        saw_header = true;
        let trimmed = trim_crlf(HeaderText::from(line.as_str()));
        if trimmed.0.is_empty() {
            break;
        }
        let Some((name, value)) = split_header(trimmed)
        else {
            return Err(FramingError::MalformedHeader);
        };
        if name.0.eq_ignore_ascii_case("content-length") {
            content_length = Some(parse_content_length(HeaderText::from(value.0.trim()))?);
        }
    }
    let Some(length) = content_length
    else {
        return Err(FramingError::MissingContentLength);
    };
    let mut payload = vec![0_u8; usize::from(length)];
    reader.read_exact(payload.as_mut_slice())?;
    Ok(Some(FrameBytes::from(payload)))
}

/// Writes one framed message without flushing.
///
/// # Contract
/// - ensures: emits `Content-Length: <len>\r\n\r\n` followed by `payload`.
/// - fails: propagates the underlying IO failure.
/// - panics: none.
///
/// # Errors
///
/// Propagates the underlying [`std::io::Error`].
#[inline]
pub fn write_message<W>(
    writer: &mut W,
    payload: FramePayload<'_>,
) -> Result<(), std::io::Error>
where
    W: Write,
{
    write!(writer, "Content-Length: {}\r\n\r\n", payload.0.len())?;
    writer.write_all(payload.0)
}

/// Strip a single trailing `\n` and an optional preceding `\r`.
fn trim_crlf(line: HeaderText<'_>) -> HeaderText<'_>
{
    let without_lf = line.0.strip_suffix('\n').unwrap_or(line.0);
    HeaderText::from(without_lf.strip_suffix('\r').unwrap_or(without_lf))
}

/// Split `Name: value`, requiring a colon.
fn split_header(line: HeaderText<'_>) -> Option<(HeaderText<'_>, HeaderText<'_>)>
{
    let (name, rest) = line.0.split_once(':')?;
    Some((HeaderText::from(name), HeaderText::from(rest)))
}

/// Parse a decimal content length inside the defensive cap.
fn parse_content_length(text: HeaderText<'_>) -> Result<ContentLength, FramingError>
{
    let Ok(length) = text.0.parse::<usize>()
    else {
        return Err(FramingError::InvalidContentLength);
    };
    if length == 0 || length > MAX_CONTENT_LENGTH {
        return Err(FramingError::InvalidContentLength);
    }
    Ok(ContentLength::from(length))
}

#[cfg(test)]
mod tests
{
    use super::read_message;
    use super::write_message;
    use crate::boundary::FramePayload;

    #[test]
    fn a_round_trip_preserves_the_payload()
    {
        let mut buffer = Vec::new();
        write_message(
            &mut buffer,
            FramePayload::from(br#"{"jsonrpc":"2.0"}"#.as_slice()),
        )
        .expect("write");
        let mut cursor = std::io::Cursor::new(buffer);
        let payload = read_message(&mut cursor).expect("read").expect("a message");
        assert_eq!(br#"{"jsonrpc":"2.0"}"#, payload.as_ref());
    }

    #[test]
    fn eof_at_a_boundary_is_clean()
    {
        let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
        assert_eq!(None, read_message(&mut cursor).expect("eof"));
    }
}
