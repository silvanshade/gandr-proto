//! Decoding operation payloads and encoding operation replies over the public
//! [`Value`] host-seam API (ADR-35 D4).
//!
//! The host sees only [`Value`] through the seam's `as_str` / `as_int` /
//! `as_list` / `as_record` decoders (never the `#[non_exhaustive]` variants),
//! and builds replies only through [`Value`]'s constructors. Every decode is
//! total: a shape mismatch is a [`ShellError::Payload`], never a panic.

use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_checker::boundary::EffectSignatureName;
use gandr_core_checker::boundary::FieldName;
use gandr_core_checker::boundary::OperationName;
use gandr_core_checker::host as sig;
use gandr_core_checker::syntax::Value;

use crate::boundary::FileKind;
use crate::boundary::FileSizeBytes;
use crate::boundary::ProcessExitCode;
use crate::boundary::ShellErrorDetail;
use crate::boundary::StandardError;
use crate::boundary::StandardOutput;
use crate::error::ShellError;

/// The stdio disposition of an `Exec::exec` spawn (ADR-74 D4).
///
/// The mode is the consumption context carried in the `Exec` payload: a
/// consumed result captures, a discarded REPL command line inherits.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum SpawnMode
{
    /// Capture the child's stdout/stderr into the typed reply (the contract for
    /// consumed results — splices, bound values, scripts). The drift-safe
    /// default: a payload with no `mode` field is captured.
    #[default]
    Captured,
    /// Let the child inherit the parent's stdin/stdout/stderr, so an
    /// interactive program behaves. The reply's captured strings are then
    /// empty.
    Inherit,
}

/// A decoded `Exec::exec` command: a program, its argument vector, and its
/// spawn mode.
///
/// argv is passed to [`std::process::Command`] element-wise (no shell
/// interpolation), so a value with shell metacharacters is an inert argument,
/// not an injection vector.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Command
{
    /// The program to run (resolved against `PATH` by the OS).
    pub program: String,
    /// The arguments, passed verbatim as separate argv entries.
    pub args: Vec<String>,
    /// The stdio disposition (ADR-74 D4): captured (default) or inherit.
    pub mode: SpawnMode,
}

/// Decodes a bare string payload (e.g. an `Fs::read` path).
///
/// # Errors
/// [`ShellError::Payload`] if the annotation-stripped payload is not a string.
#[inline]
pub fn decode_str<'sig, 'op, S, O>(
    sig: S,
    op: O,
    payload: &Value,
) -> Result<String, ShellError>
where
    S: Into<EffectSignatureName<'sig>>,
    O: Into<OperationName<'op>>,
{
    let sig = sig.into();
    let op = op.into();
    payload
        .as_str()
        .map(|text| text.as_ref().to_owned())
        .ok_or_else(|| payload_error(sig, op, "expected a string payload"))
}

/// Decodes a bare integer payload (e.g. a `Proc::exit` code).
///
/// # Errors
/// [`ShellError::Payload`] if the annotation-stripped payload is not an
/// integer.
#[inline]
pub fn decode_int<'sig, 'op, S, O>(
    sig: S,
    op: O,
    payload: &Value,
) -> Result<ProcessExitCode, ShellError>
where
    S: Into<EffectSignatureName<'sig>>,
    O: Into<OperationName<'op>>,
{
    let sig = sig.into();
    let op = op.into();
    payload
        .as_int()
        .map(i64::from)
        .map(ProcessExitCode::from)
        .ok_or_else(|| payload_error(sig, op, "expected an integer payload"))
}

/// Decodes an `Exec::exec` command payload `{program, args, mode}`.
///
/// The `mode` field (ADR-74 D4) is optional: a payload with no `mode` decodes
/// as [`SpawnMode::Captured`] (the drift-safe default), so a hand-built
/// `{program, args}` perform still captures. An unrecognized `mode` string is a
/// [`ShellError::Payload`] — the mode vocabulary is closed.
///
/// # Errors
/// [`ShellError::Payload`] if the payload is not a record, `program` is not a
/// string, `args` is not a list, any `args` element is not a string, or `mode`
/// is present but not one of the known mode strings.
#[inline]
pub fn decode_command<'sig, 'op, S, O>(
    sig: S,
    op: O,
    payload: &Value,
) -> Result<Command, ShellError>
where
    S: Into<EffectSignatureName<'sig>>,
    O: Into<OperationName<'op>>,
{
    let sig = sig.into();
    let op = op.into();
    let fields = payload
        .as_record()
        .ok_or_else(|| payload_error(sig, op, "expected a record {program, args, mode}"))?;
    let program = record_str(fields, sig, op, sig::FIELD_PROGRAM.into())?;
    let args_value = fields
        .get(sig::FIELD_ARGS)
        .ok_or_else(|| payload_error(sig, op, "missing field `args`"))?;
    let raw_args = args_value
        .as_list()
        .ok_or_else(|| payload_error(sig, op, "field `args` must be a list"))?;
    let mut args = Vec::with_capacity(raw_args.len());
    for element in raw_args {
        let arg = element
            .as_str()
            .ok_or_else(|| payload_error(sig, op, "every `args` element must be a string"))?;
        args.push(arg.as_ref().to_owned());
    }
    // The `mode` field is optional (a bare `{program, args}` perform stays
    // captured); when present it must be a known mode string.
    let mode = match fields.get(sig::FIELD_MODE) {
        | None => SpawnMode::Captured,
        | Some(value) => match value.as_str() {
            | Some(mode) if mode.as_ref() == sig::MODE_CAPTURED => SpawnMode::Captured,
            | Some(mode) if mode.as_ref() == sig::MODE_INHERIT => SpawnMode::Inherit,
            | _ => {
                return Err(payload_error(
                    sig,
                    op,
                    "field `mode` must be \"captured\" or \"inherit\"",
                ));
            },
        },
    };
    Ok(Command {
        program,
        args,
        mode,
    })
}

/// Decodes an `Fs::write` payload `{path, contents}`.
///
/// # Errors
/// [`ShellError::Payload`] if the payload is not a record with string `path`
/// and `contents` fields.
#[inline]
pub fn decode_write<'sig, 'op, S, O>(
    sig: S,
    op: O,
    payload: &Value,
) -> Result<(String, String), ShellError>
where
    S: Into<EffectSignatureName<'sig>>,
    O: Into<OperationName<'op>>,
{
    let sig = sig.into();
    let op = op.into();
    let fields = payload
        .as_record()
        .ok_or_else(|| payload_error(sig, op, "expected a record {path, contents}"))?;
    let path = record_str(fields, sig, op, sig::FIELD_PATH.into())?;
    let contents = record_str(fields, sig, op, sig::FIELD_CONTENTS.into())?;
    Ok((path, contents))
}

/// Reads a required string field of a record payload.
#[inline]
fn record_str(
    fields: &alloc::collections::BTreeMap<String, alloc::rc::Rc<Value>>,
    sig: EffectSignatureName<'_>,
    op: OperationName<'_>,
    label: FieldName<'_>,
) -> Result<String, ShellError>
{
    fields
        .get(label.as_ref())
        .and_then(|value| value.as_str())
        .map(|text| text.as_ref().to_owned())
        .ok_or_else(|| payload_error(sig, op, "missing or non-string field"))
}

/// Builds a [`ShellError::Payload`] for `sig::op`.
#[inline]
fn payload_error<'detail, D>(
    sig: EffectSignatureName<'_>,
    op: OperationName<'_>,
    detail: D,
) -> ShellError
where
    D: Into<ShellErrorDetail<'detail>>,
{
    let detail = detail.into();
    ShellError::Payload {
        sig: sig.as_ref().to_owned(),
        op: op.as_ref().to_owned(),
        detail: detail.as_ref().to_owned(),
    }
}

/// Encodes an `Exec::exec` reply record `{stdout, stderr, exit_code}`.
#[inline]
#[must_use]
pub fn encode_exec_result<'stdout, 'stderr, S, E, C>(
    stdout: S,
    stderr: E,
    exit_code: C,
) -> Value
where
    S: Into<StandardOutput<'stdout>>,
    E: Into<StandardError<'stderr>>,
    C: Into<ProcessExitCode>,
{
    let stdout = stdout.into();
    let stderr = stderr.into();
    let exit_code = exit_code.into();
    Value::record([
        (sig::FIELD_STDOUT.to_owned(), Value::string(stdout.as_ref())),
        (sig::FIELD_STDERR.to_owned(), Value::string(stderr.as_ref())),
        (
            sig::FIELD_EXIT_CODE.to_owned(),
            Value::int(i64::from(exit_code)),
        ),
    ])
}

/// Encodes an `Fs::stat` reply record `{kind, size}`.
#[inline]
#[must_use]
pub fn encode_stat<'kind, K, Z>(
    kind: K,
    size: Z,
) -> Value
where
    K: Into<FileKind<'kind>>,
    Z: Into<FileSizeBytes>,
{
    let kind = kind.into();
    let size = size.into();
    Value::record([
        (sig::FIELD_KIND.to_owned(), Value::string(kind.as_ref())),
        (sig::FIELD_SIZE.to_owned(), Value::int(i64::from(size))),
    ])
}

/// Encodes a list of strings as a `List String` value.
#[inline]
#[must_use]
pub fn encode_str_list(items: &[String]) -> Value
{
    Value::list(items.iter().map(Value::string).collect())
}
