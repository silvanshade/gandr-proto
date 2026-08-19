//! The JSON-RPC 2.0 envelope subset LSP rides on.

use alloc::string::String;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;

/// JSON-RPC: invalid JSON was received.
pub const PARSE_ERROR: RpcErrorCode = RpcErrorCode(-32700);
/// JSON-RPC: the message is not a valid request object.
pub const INVALID_REQUEST: RpcErrorCode = RpcErrorCode(-32600);
/// JSON-RPC: the request method is not served.
pub const METHOD_NOT_FOUND: RpcErrorCode = RpcErrorCode(-32601);
/// JSON-RPC: the params do not match the method.
pub const INVALID_PARAMS: RpcErrorCode = RpcErrorCode(-32602);
/// LSP: a request arrived before `initialize`.
pub const SERVER_NOT_INITIALIZED: RpcErrorCode = RpcErrorCode(-32002);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct RpcErrorCode(i64);

/// A JSON-RPC message id: an integer or a string, echoed back verbatim.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum Id
{
    /// A numeric id.
    Number(i64),
    /// A string id.
    String(String),
}

/// One classified incoming message.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Incoming
{
    /// A request that expects a response.
    Request
    {
        /// Request id.
        id: Id,
        /// Method name.
        method: String,
        /// Params object, or null.
        params: Value,
    },
    /// A notification with no response.
    Notification
    {
        /// Method name.
        method: String,
        /// Params object, or null.
        params: Value,
    },
    /// A client response to a server request; this face ignores them.
    ClientResponse,
}

/// Classifies one content payload, or produces the error response to send.
///
/// # Contract
/// - ensures: `Ok` classifies requests, notifications, and client responses;
///   `Err` carries a complete JSON-RPC error response for unparseable JSON,
///   batch arrays, and non-request objects.
/// - panics: none.
///
/// # Errors
///
/// The `Err` payload is the response to send, not a failure to handle.
#[inline]
pub fn parse_incoming(payload: crate::boundary::FramePayload<'_>) -> Result<Incoming, Value>
{
    let parsed: Value = match serde_json::from_slice(payload.as_ref()) {
        | Ok(value) => value,
        | Err(_) => {
            return Err(response_error(
                None,
                PARSE_ERROR,
                crate::boundary::ErrorText::from("Parse error"),
            ));
        },
    };
    if parsed.is_array() {
        return Err(response_error(
            None,
            INVALID_REQUEST,
            crate::boundary::ErrorText::from("LSP forbids JSON-RPC batches"),
        ));
    }
    let Some(object) = parsed.as_object()
    else {
        return Err(response_error(
            None,
            INVALID_REQUEST,
            crate::boundary::ErrorText::from("not a request object"),
        ));
    };
    if object.contains_key("result") || object.contains_key("error") {
        return Ok(Incoming::ClientResponse);
    }
    let Some(method) = object.get("method").and_then(Value::as_str)
    else {
        return Err(response_error(
            None,
            INVALID_REQUEST,
            crate::boundary::ErrorText::from("missing method"),
        ));
    };
    let params = object.get("params").cloned().unwrap_or(Value::Null);
    match object.get("id") {
        | Some(id_value) => match serde_json::from_value::<Id>(id_value.clone()) {
            | Ok(id) => Ok(Incoming::Request {
                id,
                method: String::from(method),
                params,
            }),
            | Err(_) => Err(response_error(
                None,
                INVALID_REQUEST,
                crate::boundary::ErrorText::from("invalid id"),
            )),
        },
        | None => Ok(Incoming::Notification {
            method: String::from(method),
            params,
        }),
    }
}

/// Builds a success response.
///
/// # Contract
/// - ensures: the response carries `result` and echoes `id`.
/// - panics: none.
#[inline]
#[must_use]
pub fn response_ok(
    id: &Id,
    result: &Value,
) -> Value
{
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// Builds an error response.
///
/// # Contract
/// - ensures: the response carries `error {code, message}`; `id` is null when
///   the offending id could not be determined.
/// - panics: none.
#[inline]
#[must_use]
pub fn response_error(
    id: Option<&Id>,
    code: RpcErrorCode,
    message: crate::boundary::ErrorText<'_>,
) -> Value
{
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message.0 }
    })
}

/// Builds a server-to-client notification.
///
/// # Contract
/// - ensures: the message carries `method` and `params` and no `id`.
/// - panics: none.
#[inline]
#[must_use]
pub fn notification(
    method: crate::boundary::MethodName<'_>,
    params: &Value,
) -> Value
{
    json!({ "jsonrpc": "2.0", "method": method.0, "params": params })
}

#[cfg(test)]
mod tests
{
    use super::Incoming;
    use super::parse_incoming;

    #[test]
    fn a_request_is_classified()
    {
        let incoming = parse_incoming(crate::boundary::FramePayload::from(
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#.as_slice(),
        ))
        .expect("valid request");
        assert!(matches!(incoming, Incoming::Request { method, .. } if method == "initialize"));
    }

    #[test]
    fn a_batch_is_rejected()
    {
        assert!(parse_incoming(crate::boundary::FramePayload::from(b"[]".as_slice())).is_err());
    }
}
