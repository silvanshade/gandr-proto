//! The language-server state machine.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use std::io::Write as _;

use serde_json::Value;
use serde_json::json;

use crate::analysis::Analysis;
use crate::framing::FramingError;
use crate::framing::read_message;
use crate::framing::write_message;
use crate::position::PositionEncoding;
use crate::protocol::ClientCapabilities;
use crate::protocol::DidChangeTextDocumentParams;
use crate::protocol::DidCloseTextDocumentParams;
use crate::protocol::DidOpenTextDocumentParams;
use crate::protocol::DocumentUri;
use crate::protocol::InitializeParams;
use crate::protocol::PublishDiagnosticsParams;
use crate::protocol::SemanticTokensParams;
use crate::protocol::TextDocumentPositionParams;
use crate::rpc::INVALID_PARAMS;
use crate::rpc::INVALID_REQUEST;
use crate::rpc::Id;
use crate::rpc::Incoming;
use crate::rpc::METHOD_NOT_FOUND;
use crate::rpc::SERVER_NOT_INITIALIZED;
use crate::rpc::notification;
use crate::rpc::parse_incoming;
use crate::rpc::response_error;
use crate::rpc::response_ok;
use crate::tokens::TOKEN_MODIFIERS;
use crate::tokens::TOKEN_TYPES;

/// Lifecycle of the server.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase
{
    /// Waiting for `initialize`.
    Start,
    /// Serving documents.
    Ready,
    /// `shutdown` received; only `exit` remains.
    Shutdown,
}

/// Whether the stdio loop should keep reading.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LoopControl
{
    /// Read the next message.
    Continue,
    /// Leave the loop.
    Stop,
}

/// The language-server state.
pub struct Server
{
    /// Lifecycle phase.
    phase: Phase,
    /// Negotiated position encoding.
    encoding: PositionEncoding,
    /// Open documents by URI.
    documents: BTreeMap<DocumentUri, String>,
}

impl Server
{
    /// A server that has not yet been initialized.
    ///
    /// # Contract
    /// - ensures: the server is in the start phase with UTF-16 encoding.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self {
            phase: Phase::Start,
            encoding: PositionEncoding::Utf16,
            documents: BTreeMap::new(),
        }
    }

    /// Handle one content payload and return the messages to send.
    ///
    /// # Contract
    /// - ensures: returns zero or more JSON-RPC messages. `exit` is reported by
    ///   [`HandleOutcome::should_stop`].
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn handle_payload(
        &mut self,
        payload: crate::boundary::FramePayload<'_>,
    ) -> HandleOutcome
    {
        match parse_incoming(payload) {
            | Ok(incoming) => self.handle_incoming(incoming),
            | Err(error) => HandleOutcome::continue_with(Vec::from([error])),
        }
    }

    /// Dispatch a classified incoming message.
    fn handle_incoming(
        &mut self,
        incoming: Incoming,
    ) -> HandleOutcome
    {
        match incoming {
            | Incoming::ClientResponse => HandleOutcome::continue_with(Vec::new()),
            | Incoming::Notification { method, params } => {
                self.handle_notification(crate::boundary::MethodName::from(method.as_str()), params)
            },
            | Incoming::Request { id, method, params } => self.handle_request(
                &id,
                crate::boundary::MethodName::from(method.as_str()),
                params,
            ),
        }
    }

    /// Handle a notification.
    fn handle_notification(
        &mut self,
        method: crate::boundary::MethodName<'_>,
        params: Value,
    ) -> HandleOutcome
    {
        match method.0 {
            | "exit" => HandleOutcome::stop(Vec::new()),
            | "textDocument/didOpen" => self.did_open(params),
            | "textDocument/didChange" => self.did_change(params),
            | "textDocument/didClose" => self.did_close(params),
            | _ => HandleOutcome::continue_with(Vec::new()),
        }
    }

    /// Handle a request.
    fn handle_request(
        &mut self,
        id: &Id,
        method: crate::boundary::MethodName<'_>,
        params: Value,
    ) -> HandleOutcome
    {
        if method.0 == "initialize" {
            return self.initialize(id, params);
        }
        if self.phase == Phase::Start {
            return HandleOutcome::continue_with(Vec::from([response_error(
                Some(id),
                SERVER_NOT_INITIALIZED,
                crate::boundary::ErrorText::from("server not initialized"),
            )]));
        }
        if self.phase == Phase::Shutdown && method.0 != "shutdown" {
            return HandleOutcome::continue_with(Vec::from([response_error(
                Some(id),
                INVALID_REQUEST,
                crate::boundary::ErrorText::from("server is shut down"),
            )]));
        }
        match method.0 {
            | "shutdown" => {
                self.phase = Phase::Shutdown;
                HandleOutcome::continue_with(Vec::from([response_ok(id, &Value::Null)]))
            },
            | "textDocument/semanticTokens/full" => self.semantic_tokens(id, params),
            | "textDocument/hover" => self.hover(id, params),
            | "textDocument/completion" => self.completion(id, params),
            | _ => HandleOutcome::continue_with(Vec::from([response_error(
                Some(id),
                METHOD_NOT_FOUND,
                crate::boundary::ErrorText::from("method not found"),
            )])),
        }
    }

    /// Complete the initialize handshake.
    fn initialize(
        &mut self,
        id: &Id,
        params: Value,
    ) -> HandleOutcome
    {
        if self.phase != Phase::Start {
            return HandleOutcome::continue_with(Vec::from([response_error(
                Some(id),
                INVALID_REQUEST,
                crate::boundary::ErrorText::from("initialize already received"),
            )]));
        }
        let parsed = serde_json::from_value::<InitializeParams>(params).unwrap_or_default();
        self.encoding = negotiate_encoding(&parsed.capabilities);
        self.phase = Phase::Ready;
        HandleOutcome::continue_with(Vec::from([response_ok(
            id,
            &initialize_result(self.encoding),
        )]))
    }

    /// Record an opened document and publish diagnostics.
    fn did_open(
        &mut self,
        params: Value,
    ) -> HandleOutcome
    {
        let Ok(params) = serde_json::from_value::<DidOpenTextDocumentParams>(params)
        else {
            return HandleOutcome::continue_with(Vec::new());
        };
        self.documents
            .insert(params.text_document.uri.clone(), params.text_document.text);
        self.publish(&params.text_document.uri)
    }

    /// Apply a full-document change and publish diagnostics.
    fn did_change(
        &mut self,
        params: Value,
    ) -> HandleOutcome
    {
        let Ok(params) = serde_json::from_value::<DidChangeTextDocumentParams>(params)
        else {
            return HandleOutcome::continue_with(Vec::new());
        };
        let Some(change) = params.content_changes.last()
        else {
            return HandleOutcome::continue_with(Vec::new());
        };
        self.documents
            .insert(params.text_document.uri.clone(), change.text.clone());
        self.publish(&params.text_document.uri)
    }

    /// Forget a closed document and clear its diagnostics.
    fn did_close(
        &mut self,
        params: Value,
    ) -> HandleOutcome
    {
        let Ok(params) = serde_json::from_value::<DidCloseTextDocumentParams>(params)
        else {
            return HandleOutcome::continue_with(Vec::new());
        };
        self.documents.remove(&params.text_document.uri);
        HandleOutcome::continue_with(Vec::from([notification(
            crate::boundary::MethodName::from("textDocument/publishDiagnostics"),
            &json!(PublishDiagnosticsParams {
                uri: params.text_document.uri,
                diagnostics: Vec::new(),
            }),
        )]))
    }

    /// Publish diagnostics for `uri`.
    fn publish(
        &self,
        uri: &DocumentUri,
    ) -> HandleOutcome
    {
        let Some(text) = self.documents.get(uri)
        else {
            return HandleOutcome::continue_with(Vec::new());
        };
        let analysis = Analysis::check(text.clone());
        HandleOutcome::continue_with(Vec::from([notification(
            crate::boundary::MethodName::from("textDocument/publishDiagnostics"),
            &json!(PublishDiagnosticsParams {
                uri: uri.clone(),
                diagnostics: analysis.diagnostics(self.encoding),
            }),
        )]))
    }

    /// Answer `textDocument/semanticTokens/full`.
    fn semantic_tokens(
        &self,
        id: &Id,
        params: Value,
    ) -> HandleOutcome
    {
        let Ok(params) = serde_json::from_value::<SemanticTokensParams>(params)
        else {
            return HandleOutcome::continue_with(Vec::from([response_error(
                Some(id),
                INVALID_PARAMS,
                crate::boundary::ErrorText::from("invalid params"),
            )]));
        };
        let Some(text) = self.documents.get(&params.text_document.uri)
        else {
            return HandleOutcome::continue_with(Vec::from([response_ok(id, &json!(null))]));
        };
        let tokens = Analysis::check(text.clone()).semantic_tokens(self.encoding);
        HandleOutcome::continue_with(Vec::from([response_ok(id, &json!(tokens))]))
    }

    /// Answer `textDocument/hover`.
    fn hover(
        &self,
        id: &Id,
        params: Value,
    ) -> HandleOutcome
    {
        let Ok(params) = serde_json::from_value::<TextDocumentPositionParams>(params)
        else {
            return HandleOutcome::continue_with(Vec::from([response_error(
                Some(id),
                INVALID_PARAMS,
                crate::boundary::ErrorText::from("invalid params"),
            )]));
        };
        let Some(text) = self.documents.get(&params.text_document.uri)
        else {
            return HandleOutcome::continue_with(Vec::from([response_ok(id, &json!(null))]));
        };
        let hover = Analysis::check(text.clone()).hover(params.position, self.encoding);
        HandleOutcome::continue_with(Vec::from([response_ok(id, &json!(hover))]))
    }

    /// Answer `textDocument/completion`.
    fn completion(
        &self,
        id: &Id,
        params: Value,
    ) -> HandleOutcome
    {
        let Ok(params) = serde_json::from_value::<TextDocumentPositionParams>(params)
        else {
            return HandleOutcome::continue_with(Vec::from([response_error(
                Some(id),
                INVALID_PARAMS,
                crate::boundary::ErrorText::from("invalid params"),
            )]));
        };
        let Some(text) = self.documents.get(&params.text_document.uri)
        else {
            return HandleOutcome::continue_with(Vec::from([response_ok(id, &json!([]))]));
        };
        let items = Analysis::check(text.clone()).completions(params.position, self.encoding);
        HandleOutcome::continue_with(Vec::from([response_ok(id, &json!(items))]))
    }
}

impl Default for Server
{
    #[inline]
    fn default() -> Self
    {
        Self::new()
    }
}

/// Outcome of handling one payload.
pub struct HandleOutcome
{
    /// Messages to write.
    pub messages: Vec<Value>,
    /// Whether the stdio loop should stop.
    stop: LoopControl,
}

impl HandleOutcome
{
    /// Continue after writing `messages`.
    fn continue_with(messages: Vec<Value>) -> Self
    {
        Self {
            messages,
            stop: LoopControl::Continue,
        }
    }

    /// Stop after writing `messages`.
    fn stop(messages: Vec<Value>) -> Self
    {
        Self {
            messages,
            stop: LoopControl::Stop,
        }
    }

    /// Whether the driver should leave the stdio loop.
    ///
    /// # Contract
    /// - ensures: true only after `exit`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn should_stop(&self) -> crate::boundary::ShouldStop
    {
        crate::boundary::ShouldStop::from(self.stop == LoopControl::Stop)
    }
}

/// The initialize result this face advertises, including the token legend.
///
/// # Contract
/// - ensures: the JSON is the `initialize` result body for UTF-16.
/// - panics: none.
#[inline]
#[must_use]
pub fn advertised_capabilities() -> Value
{
    initialize_result(PositionEncoding::Utf16)
}

/// Pretty-printed [`advertised_capabilities`] for the CLI smoke path.
///
/// # Contract
/// - ensures: returns pretty JSON ending in a newline; `{}` only if
///   serialization fails, which these value types cannot do.
/// - panics: none.
#[inline]
#[must_use]
pub fn advertised_capabilities_text() -> String
{
    let body = serde_json::to_string_pretty(&advertised_capabilities())
        .unwrap_or_else(|_| String::from("{}"));
    format!("{body}\n")
}

/// Run the synchronous stdio language server.
///
/// # Contract
/// - ensures: reads framed messages from stdin and writes responses to stdout
///   until `exit` or a clean EOF.
/// - fails: a framing error ends the session.
/// - panics: none.
///
/// # Errors
///
/// Returns [`FramingError`] when the stream desynchronizes.
#[inline]
pub fn run_stdio() -> Result<(), FramingError>
{
    let mut server = Server::new();
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    let mut stdout = std::io::stdout();
    loop {
        let Some(payload) = read_message(&mut reader)?
        else {
            return Ok(());
        };
        let outcome = server.handle_payload(crate::boundary::FramePayload::from(payload.as_ref()));
        for message in &outcome.messages {
            let encoded = serde_json::to_vec(message).unwrap_or_else(|_| Vec::from(b"{}"));
            write_message(
                &mut stdout,
                crate::boundary::FramePayload::from(encoded.as_slice()),
            )?;
        }
        stdout.flush()?;
        if bool::from(outcome.should_stop()) {
            return Ok(());
        }
    }
}

/// Negotiate UTF-8 when the client lists it, otherwise UTF-16.
fn negotiate_encoding(capabilities: &ClientCapabilities) -> PositionEncoding
{
    let Some(general) = capabilities.general.as_ref()
    else {
        return PositionEncoding::Utf16;
    };
    let Some(encodings) = general.position_encodings.as_ref()
    else {
        return PositionEncoding::Utf16;
    };
    if encodings.iter().any(|encoding| encoding == "utf-8") {
        PositionEncoding::Utf8
    }
    else {
        PositionEncoding::Utf16
    }
}

/// The initialize result for `encoding`.
fn initialize_result(encoding: PositionEncoding) -> Value
{
    let encoding_name = match encoding {
        | PositionEncoding::Utf8 => "utf-8",
        | PositionEncoding::Utf16 => "utf-16",
    };
    json!({
        "capabilities": {
            "positionEncoding": encoding_name,
            "textDocumentSync": 1,
            "hoverProvider": true,
            "completionProvider": { "triggerCharacters": ["."] },
            "semanticTokensProvider": {
                "legend": {
                    "tokenTypes": TOKEN_TYPES,
                    "tokenModifiers": TOKEN_MODIFIERS
                },
                "full": true,
                "range": false
            }
        },
        "serverInfo": { "name": "gandr-lsp", "version": "0.0.0" }
    })
}

#[cfg(test)]
mod tests
{
    use serde_json::Value;

    use super::Server;
    use super::advertised_capabilities;

    #[test]
    fn initialize_advertises_the_token_legend()
    {
        let mut server = Server::new();
        let outcome = server.handle_payload(crate::boundary::FramePayload::from(
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#.as_slice(),
        ));
        let Some(message) = outcome.messages.first()
        else {
            panic!("initialize must answer");
        };
        let types = message
            .pointer("/result/capabilities/semanticTokensProvider/legend/tokenTypes")
            .and_then(Value::as_array)
            .expect("legend");
        assert_eq!(Some("keyword"), types.first().and_then(Value::as_str));
        assert_eq!(advertised_capabilities(), message["result"]);
    }
}
