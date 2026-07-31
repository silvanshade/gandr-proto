//! The versioned render-bus frame + delta wire schema.
//!
//! These types realize the render-bus protocol of the inspection-protocol
//! proposal (the inspection-protocol proposal §4–§5): the
//! frame envelope, the machine-state projection a renderer paints, the
//! incremental delta form, and the scalar machine summary. They are the
//! serialized image of the in-process worker→present seam ([`crate::present`])
//! — the same projection, now crossing a socket instead of an `mpsc` channel.
//!
//! # Versioning discipline (proposal Axis C)
//!
//! Every document-scoped message carries `(doc_uri, doc_version)`: `doc_uri`
//! identifies the document, `doc_version` is the LSP `textDocument` version the
//! projection describes. A renderer paints a frame only while its `doc_version`
//! matches the buffer it shows, and requests a [`FrameBody::Resync`] otherwise;
//! the server coalesces to the latest version per document under edit pressure.
//! [`RenderFrame::schema_version`] is the wire-format version — the bus
//! analogue of the pipeline `Report`'s `schema_version` — and a consumer checks
//! it before decoding.
//!
//! # Session-typeable shape (proposal Axis E)
//!
//! The message set is the projection of a binary session — `?Attach . !Hello .
//! μ X. ( !Frame . X ⊕ !Delta . X & ?Resync . X & ?Detach . end )`. When A4
//! sessions land (`type-system.md` §7–§8) the bus server becomes a typed
//! endpoint and today's JSON frames are that protocol's messages, with no
//! consumer break.
//!
//! # `report` — the present projection, not the pipeline `Report`
//!
//! [`FrameBody::Frame`] carries a [`ReportView`] — the [`crate::present`]
//! projection (highlight/mark spans, diagnostic and goal cards) — rather than a
//! re-export of `gandr_pipeline::diag::Report`. This keeps the crate a true
//! leaf (no pipeline dependency) and is exactly the renderer-facing shape the
//! `gandr-tui` worker already produces; the proposal's §4 sketch wrote `report:
//! Report` generically, and §1 resolves it as "the same projection" the present
//! seam carries. The structured `gandr_pipeline::diag::Report` remains the
//! Channel-1 (LSP) and agent-JSON surface.

use core::fmt;

use crate::present::DiagCard;
use crate::present::GoalCard;
use crate::present::HlSpan;
use crate::present::MarkSpan;

/// The render-bus wire schema version number.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "codecs", serde(transparent))]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct WireSchemaVersion(u32);

impl From<u32> for WireSchemaVersion
{
    #[inline]
    fn from(version: u32) -> Self
    {
        Self(version)
    }
}

impl From<WireSchemaVersion> for u32
{
    #[inline]
    fn from(version: WireSchemaVersion) -> Self
    {
        version.0
    }
}

/// An LSP `textDocument` version carried by document-scoped render frames.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "codecs", serde(transparent))]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct DocVersion(i32);

impl From<i32> for DocVersion
{
    #[inline]
    fn from(version: i32) -> Self
    {
        Self(version)
    }
}

impl From<DocVersion> for i32
{
    #[inline]
    fn from(version: DocVersion) -> Self
    {
        version.0
    }
}
/// A borrowed LSP document URI carried by document-scoped render frames.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct DocumentUri<'uri>(&'uri str);

impl AsRef<str> for DocumentUri<'_>
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        self.0
    }
}

impl fmt::Display for DocumentUri<'_>
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        self.0.fmt(f)
    }
}

/// The render-bus wire schema version.
///
/// Bumped whenever a wire field is renamed, removed, or changes meaning
/// (additive fields do not require a bump). A consumer checks
/// [`RenderFrame::schema_version`] against this before decoding a stream.
///
/// This is distinct from the pipeline `Report`'s own `schema_version`
/// (currently `2`): that versions the diagnostics/goals/marks JSON envelope on
/// Channel 1; this versions the render-bus envelope.
pub const WIRE_SCHEMA_VERSION: WireSchemaVersion = WireSchemaVersion(1);

/// One bus message: the frame envelope of proposal §4.
///
/// `doc_uri` and `doc_version` are the per-document routing keys; they are
/// `Some` for document-scoped bodies ([`FrameBody::Frame`],
/// [`FrameBody::Delta`], [`FrameBody::Resync`]) and `None` for
/// connection-scoped bodies ([`FrameBody::Hello`], [`FrameBody::Detach`]),
/// which are not about one document. Construct through the body-specific
/// constructors, which maintain that invariant.
#[cfg_attr(feature = "codecs", derive(serde::Serialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderFrame
{
    /// The wire schema version ([`WIRE_SCHEMA_VERSION`]); checked before
    /// decoding.
    schema_version: WireSchemaVersion,
    /// The LSP document URI this message concerns; `Some` for document-scoped
    /// bodies, `None` for connection-scoped ones.
    doc_uri: Option<String>,
    /// The LSP `textDocument` version this message concerns; `Some`/`None` in
    /// lockstep with [`Self::doc_uri`].
    doc_version: Option<DocVersion>,
    /// The message payload.
    body: FrameBody,
}

/// The serde decode image of a [`RenderFrame`]: the same fields with no
/// invariants attached, so deserialization hands [`RenderFrame::from_wire`]
/// raw input to validate before a frame exists.
#[cfg(feature = "codecs")]
#[derive(serde::Deserialize)]
struct RenderFrameWire
{
    /// The decoded schema version; checked against [`WIRE_SCHEMA_VERSION`].
    schema_version: WireSchemaVersion,
    /// The decoded document-URI routing key.
    doc_uri: Option<String>,
    /// The decoded document-version routing key; must be present or absent in
    /// lockstep with [`Self::doc_uri`].
    doc_version: Option<DocVersion>,
    /// The decoded payload; its variant determines which routing keys are
    /// lawful.
    body: FrameBody,
}

#[cfg(feature = "codecs")]
impl<'de> serde::Deserialize<'de> for RenderFrame
{
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = RenderFrameWire::deserialize(deserializer)?;
        Self::from_wire(wire)
    }
}

impl RenderFrame
{
    /// The wire schema version carried by this frame.
    ///
    /// # Contract
    /// - ensures: returns [`WIRE_SCHEMA_VERSION`] for frames built through this
    ///   API or decoded from a valid wire image.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn schema_version(&self) -> WireSchemaVersion
    {
        self.schema_version
    }

    /// The LSP document URI this frame concerns, if document-scoped.
    ///
    /// # Contract
    /// - ensures: returns `Some` exactly for document-scoped bodies.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn doc_uri(&self) -> Option<DocumentUri<'_>>
    {
        self.doc_uri.as_deref().map(DocumentUri)
    }

    /// The LSP document version this frame concerns, if document-scoped.
    ///
    /// # Contract
    /// - ensures: returns `Some` exactly for document-scoped bodies.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn doc_version(&self) -> Option<DocVersion>
    {
        self.doc_version
    }

    /// The frame payload.
    ///
    /// # Contract
    /// - ensures: exposes the body read-only; construct replacement frames
    ///   through the named constructors.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn body(&self) -> &FrameBody
    {
        &self.body
    }

    /// Validates a freshly decoded [`RenderFrameWire`] into a [`RenderFrame`].
    ///
    /// # Contract
    /// - ensures: the schema version is [`WIRE_SCHEMA_VERSION`]; `doc_uri` and
    ///   `doc_version` are both present or both absent; and the routing keys
    ///   match the body's scope — connection-scoped bodies
    ///   ([`FrameBody::Hello`], [`FrameBody::Detach`]) carry none,
    ///   document-scoped bodies ([`FrameBody::Frame`], [`FrameBody::Delta`],
    ///   [`FrameBody::Resync`]) carry both.
    /// - fails: surfaces a serde error naming the broken invariant.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns `E::custom` when the schema version differs from
    /// [`WIRE_SCHEMA_VERSION`], when `doc_uri` and `doc_version` are not
    /// present in lockstep, or when the routing keys do not match the body's
    /// scope.
    #[cfg(feature = "codecs")]
    fn from_wire<E>(wire: RenderFrameWire) -> Result<Self, E>
    where
        E: serde::de::Error,
    {
        if wire.schema_version != WIRE_SCHEMA_VERSION {
            return Err(E::custom(
                "render frame schema_version does not match the supported wire schema",
            ));
        }

        let has_document_keys = match (&wire.doc_uri, wire.doc_version) {
            | (&Some(_), Some(_)) => true,
            | (&None, None) => false,
            | _ => {
                return Err(E::custom(
                    "render frame doc_uri and doc_version must be present or absent in lockstep",
                ));
            },
        };

        match (&wire.body, has_document_keys) {
            | (&FrameBody::Hello { .. } | &FrameBody::Detach, false)
            | (&FrameBody::Frame { .. } | &FrameBody::Delta { .. } | &FrameBody::Resync, true) => {
            },
            | (&FrameBody::Hello { .. }, true) => {
                return Err(E::custom(
                    "hello render frames must not carry document routing keys",
                ));
            },
            | (&FrameBody::Detach, true) => {
                return Err(E::custom(
                    "detach render frames must not carry document routing keys",
                ));
            },
            | (&FrameBody::Frame { .. }, false) => {
                return Err(E::custom(
                    "frame render frames require document routing keys",
                ));
            },
            | (&FrameBody::Delta { .. }, false) => {
                return Err(E::custom(
                    "delta render frames require document routing keys",
                ));
            },
            | (&FrameBody::Resync, false) => {
                return Err(E::custom(
                    "resync render frames require document routing keys",
                ));
            },
        }

        Ok(Self {
            schema_version: wire.schema_version,
            doc_uri: wire.doc_uri,
            doc_version: wire.doc_version,
            body: wire.body,
        })
    }

    /// The server→client attach greeting: the tracked document set and the
    /// server's capabilities. Connection-scoped, so the routing keys are
    /// absent.
    ///
    /// # Contract
    /// - ensures: `schema_version` is [`WIRE_SCHEMA_VERSION`]; `doc_uri` and
    ///   `doc_version` are `None`; the body is [`FrameBody::Hello`].
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn hello(
        docs: Vec<DocId>,
        caps: ServerCaps,
    ) -> Self
    {
        Self {
            schema_version: WIRE_SCHEMA_VERSION,
            doc_uri: None,
            doc_version: None,
            body: FrameBody::Hello { docs, caps },
        }
    }

    /// A full projection for one document version: the present-projection
    /// [`ReportView`] plus the [`MachineView`] machine state.
    ///
    /// # Contract
    /// - ensures: `schema_version` is [`WIRE_SCHEMA_VERSION`]; the routing keys
    ///   are `Some(doc_uri)` / `Some(doc_version)`; the body is
    ///   [`FrameBody::Frame`].
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn frame(
        doc_uri: String,
        doc_version: DocVersion,
        report: ReportView,
        machine: MachineView,
    ) -> Self
    {
        Self {
            schema_version: WIRE_SCHEMA_VERSION,
            doc_uri: Some(doc_uri),
            doc_version: Some(doc_version),
            body: FrameBody::Frame { report, machine },
        }
    }

    /// An incremental patch producing `doc_version` from an acknowledged
    /// `base_version` (proposal Axis B; the growth path).
    ///
    /// # Contract
    /// - ensures: `schema_version` is [`WIRE_SCHEMA_VERSION`]; the routing keys
    ///   are `Some`; the body is [`FrameBody::Delta`] carrying `base_version`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn delta(
        doc_uri: String,
        doc_version: DocVersion,
        base_version: DocVersion,
        changed: Vec<NodeDelta>,
    ) -> Self
    {
        Self {
            schema_version: WIRE_SCHEMA_VERSION,
            doc_uri: Some(doc_uri),
            doc_version: Some(doc_version),
            body: FrameBody::Delta {
                base_version,
                changed,
            },
        }
    }

    /// A client→server request to resend a full [`FrameBody::Frame`] for
    /// `doc_version` (the client fell behind).
    ///
    /// # Contract
    /// - ensures: `schema_version` is [`WIRE_SCHEMA_VERSION`]; the routing keys
    ///   are `Some`; the body is [`FrameBody::Resync`].
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn resync(
        doc_uri: String,
        doc_version: DocVersion,
    ) -> Self
    {
        Self {
            schema_version: WIRE_SCHEMA_VERSION,
            doc_uri: Some(doc_uri),
            doc_version: Some(doc_version),
            body: FrameBody::Resync,
        }
    }

    /// An either-direction connection close. Connection-scoped, so the routing
    /// keys are absent.
    ///
    /// # Contract
    /// - ensures: `schema_version` is [`WIRE_SCHEMA_VERSION`]; the routing keys
    ///   are `None`; the body is [`FrameBody::Detach`].
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn detach() -> Self
    {
        Self {
            schema_version: WIRE_SCHEMA_VERSION,
            doc_uri: None,
            doc_version: None,
            body: FrameBody::Detach,
        }
    }
}

/// The payload of a [`RenderFrame`] — the message set of proposal §4.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "codecs",
    serde(tag = "kind", content = "data", rename_all = "snake_case")
)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrameBody
{
    /// server→client on attach: the tracked document set and the server's
    /// capabilities.
    Hello
    {
        /// The documents the server tracks, with their current versions.
        docs: Vec<DocId>,
        /// What the server can stream (deltas, obligations, sessions).
        caps: ServerCaps,
    },
    /// A full projection for one document version (the MVP always sends whole
    /// frames).
    Frame
    {
        /// The present-projection diagnostics/goals/marks/highlights.
        report: ReportView,
        /// The reified machine-state projection.
        machine: MachineView,
    },
    /// A node-id-keyed patch against a prior frame (the growth path; obligation
    /// deltas ride here once the reserved slot populates).
    Delta
    {
        /// The acknowledged version this patch applies against.
        base_version: DocVersion,
        /// The changed derivation nodes.
        changed: Vec<NodeDelta>,
    },
    /// client→server: "I fell behind, resend a full [`Self::Frame`]" for the
    /// envelope's `doc_version`.
    Resync,
    /// Either direction: close the stream.
    Detach,
}

/// A tracked document's identity: its URI and current version.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DocId
{
    /// The LSP document URI.
    pub uri: String,
    /// The LSP `textDocument` version.
    pub version: DocVersion,
}

impl DocId
{
    /// A document id from its URI and version.
    ///
    /// # Contract
    /// - ensures: fields are stored verbatim.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        uri: String,
        version: DocVersion,
    ) -> Self
    {
        Self { uri, version }
    }
}

/// Whether the server may send incremental derivation deltas.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "codecs", serde(transparent))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct DeltaStreaming(bool);

impl From<bool> for DeltaStreaming
{
    #[inline]
    fn from(enabled: bool) -> Self
    {
        Self(enabled)
    }
}

impl From<DeltaStreaming> for bool
{
    #[inline]
    fn from(enabled: DeltaStreaming) -> Self
    {
        enabled.0
    }
}

/// Whether obligation deltas are populated in render-bus messages.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "codecs", serde(transparent))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct ObligationDeltas(bool);

impl From<bool> for ObligationDeltas
{
    #[inline]
    fn from(enabled: bool) -> Self
    {
        Self(enabled)
    }
}

impl From<ObligationDeltas> for bool
{
    #[inline]
    fn from(enabled: ObligationDeltas) -> Self
    {
        enabled.0
    }
}

/// Whether derivation nodes carry session protocol badges.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "codecs", serde(transparent))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct SessionBadges(bool);

impl From<bool> for SessionBadges
{
    #[inline]
    fn from(enabled: bool) -> Self
    {
        Self(enabled)
    }
}

impl From<SessionBadges> for bool
{
    #[inline]
    fn from(enabled: SessionBadges) -> Self
    {
        enabled.0
    }
}

/// What a bus server can stream, advertised in [`FrameBody::Hello`].
///
/// The MVP posture ([`Self::mvp`]) is whole-frame only: no deltas, no
/// obligations, no session badges. Each flag graduates independently as its
/// backing machinery lands (deltas/obligations, A4 for
/// sessions).
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServerCaps
{
    /// The wire schema version the server speaks ([`WIRE_SCHEMA_VERSION`]).
    pub schema_version: WireSchemaVersion,
    /// Whether the server may send [`FrameBody::Delta`] frames (proposal Axis
    /// B; `false` in the MVP).
    pub deltas: DeltaStreaming,
    /// Whether obligation deltas are populated (the reserved slot; `false`
    /// today).
    pub obligations: ObligationDeltas,
    /// Whether derivation nodes carry session [`ProtocolBadge`]s (A4; `false`
    /// today).
    pub sessions: SessionBadges,
}

impl ServerCaps
{
    /// Server capabilities from their parts.
    ///
    /// # Contract
    /// - ensures: fields are stored verbatim.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        deltas: DeltaStreaming,
        obligations: ObligationDeltas,
        sessions: SessionBadges,
    ) -> Self
    {
        Self {
            schema_version: WIRE_SCHEMA_VERSION,
            deltas,
            obligations,
            sessions,
        }
    }

    /// The Wave-1 MVP posture: whole frames only.
    ///
    /// # Contract
    /// - ensures: `schema_version` is [`WIRE_SCHEMA_VERSION`]; every capability
    ///   flag is `false`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn mvp() -> Self
    {
        Self::new(false.into(), false.into(), false.into())
    }
}

/// The present-projection view carried by a [`FrameBody::Frame`]: the
/// renderer-facing diagnostics, goals, marks, and highlights.
///
/// This is the serialized image of the fields the `gandr-tui` worker hands its
/// renderer (a [`crate::present::PreviewFrame`] minus its cursor-local and
/// generation fields, which are Channel-1 / in-process concerns). Obligations
/// are reserved: they arrive via [`FrameBody::Delta`] once the pipeline's
/// reserved slot populates.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReportView
{
    /// Syntax-highlight spans over the document.
    pub highlights: Vec<HlSpan>,
    /// Total-marking spans (error underlines + hole tints).
    pub marks: Vec<MarkSpan>,
    /// Fail-fast diagnostics.
    pub diagnostics: Vec<DiagCard>,
    /// Hole goals, in source order.
    pub goals: Vec<GoalCard>,
}

/// The reified machine-state projection carried by a [`FrameBody::Frame`]
/// (proposal §4; `typing-machine.md` §3/§5/§7).
///
/// It is the derivation forest, the control register, the frame-stack summary,
/// and the scalar digest — everything a renderer needs to paint the typing
/// machine, projected to plain strings and ids (never the `Rc`-based semantic
/// values).
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MachineView
{
    /// The derivation forest roots (`typing-machine.md` §7), context-deltas not
    /// snapshots.
    pub derivation: Vec<DerivationNode>,
    /// The control register (`typing-machine.md` §3.1).
    pub control: ControlView,
    /// The frame stack, top first (`typing-machine.md` §3.3), as a summary.
    pub frames: Vec<FrameSummary>,
    /// The scalar digest (also the `gandr/machineSummary` payload).
    pub summary: MachineSummary,
}

/// One derivation-tree node (`typing-machine.md` §7 `derivation_node`),
/// projected to plain data.
///
/// Stores context *deltas* (the bindings this node added), not snapshots; a
/// renderer reconstructs any node's full context by folding deltas along the
/// path from the root. The `id` is the machine's `step_id`, stable enough to
/// key a [`NodeDelta`] against. Construct via [`Default`] and set fields, or
/// (in the bus server) the pipeline projection builder.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DerivationNode
{
    /// The node id (the machine `step_id`), the delta key.
    pub id: NodeId,
    /// The rule name (`"Abs⇓"`, `"∪E"`, `"Acquire"`, …).
    pub rule: String,
    /// The typed sub-expression, rendered.
    pub expr: String,
    /// The direction this node was typed in.
    pub direction: DirView,
    /// The CBPV layer (value or computation).
    pub layer: LayerView,
    /// The world badge, when worlds are active (`typing-machine.md` §3.3).
    pub world: Option<String>,
    /// The bindings this node added to the context (a delta, not a snapshot).
    pub ctx_delta: Vec<CtxBinding>,
    /// The endpoints consumed below this node (linear resources).
    pub linear_in: Vec<String>,
    /// The node's resulting type, rendered, when it produced one.
    pub result: Option<String>,
    /// The sub-derivations (the tree mirrors the stack discipline).
    pub children: Vec<Self>,
    /// The session before/after badge, when this is a session frame.
    pub protocol: Option<ProtocolBadge>,
    /// The grade, rendered, when the node is graded.
    pub grade: Option<String>,
}

/// A machine monotone step counter value used as a derivation-node key.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "codecs", serde(transparent))]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct NodeStep(u64);

impl From<u64> for NodeStep
{
    #[inline]
    fn from(step: u64) -> Self
    {
        Self(step)
    }
}

impl From<NodeStep> for u64
{
    #[inline]
    fn from(step: NodeStep) -> Self
    {
        step.0
    }
}

/// A derivation node's id — the machine's monotone `step_id`
/// (`typing-machine.md` §3.2 / §7). The key a [`NodeDelta`] patches against.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct NodeId
{
    /// The step counter value.
    pub step: NodeStep,
}

impl NodeId
{
    /// A node id from its step counter.
    ///
    /// # Contract
    /// - ensures: the field is stored verbatim.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(step: NodeStep) -> Self
    {
        Self { step }
    }
}

/// One typing-context hypothesis `name : type`, rendered (the delta entries of
/// [`DerivationNode::ctx_delta`]).
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CtxBinding
{
    /// The bound name.
    pub name: String,
    /// The bound type, rendered.
    pub ty: String,
}

impl CtxBinding
{
    /// A context binding from its name and rendered type.
    ///
    /// # Contract
    /// - ensures: fields are stored verbatim.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        name: String,
        ty: String,
    ) -> Self
    {
        Self { name, ty }
    }
}

/// The direction a node was typed in (`typing-machine.md` §3.1 `dir`).
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "codecs",
    serde(tag = "kind", content = "data", rename_all = "snake_case")
)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum DirView
{
    /// Inference position (`Infer`): the type is synthesized.
    #[default]
    Infer,
    /// Checking position (`Check of ty`): the term checks against `expected`.
    Check
    {
        /// The expected type, rendered.
        expected: String,
    },
}

/// The CBPV layer of a node (`typing-machine.md` §3.1 `layer`).
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "codecs", serde(rename_all = "snake_case"))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LayerView
{
    /// A value (`Val`): values *are*.
    #[default]
    Value,
    /// A computation (`Comp`): computations *do*.
    Computation,
}

/// The control register (`typing-machine.md` §3.1 `control`), projected.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "codecs",
    serde(tag = "kind", content = "data", rename_all = "snake_case")
)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum ControlView
{
    /// `Descend`: about to type a sub-expression.
    Descend
    {
        /// The sub-expression, rendered.
        expr: String,
        /// The direction it will be typed in.
        dir: DirView,
        /// The CBPV layer.
        layer: LayerView,
    },
    /// `Return`: a sub-expression's type propagating up.
    Return
    {
        /// The returning type, rendered.
        ty: String,
    },
    /// The projection's idle sentinel: the machine is at rest (not mid-step)
    /// for the frame emitted. Not one of the machine's own `control`
    /// states.
    #[default]
    Done,
}

/// One frame-stack entry, summarized (`typing-machine.md` §3.3). Mirrors the
/// pipeline `ContextFrame` shape the failure chain already uses.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FrameSummary
{
    /// The frame constructor name (`"KAppFn"`, `"KBind"`, …), for stable
    /// machine parsing.
    pub role: String,
    /// A prose description of the pending obligation.
    pub prose: String,
    /// The frame's binder name, for binder-introducing frames.
    pub binder: Option<String>,
}

impl FrameSummary
{
    /// A frame summary from its parts.
    ///
    /// # Contract
    /// - ensures: fields are stored verbatim.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        role: String,
        prose: String,
        binder: Option<String>,
    ) -> Self
    {
        Self {
            role,
            prose,
            binder,
        }
    }
}

/// A session before/after badge (`typing-machine.md` §7 `protocol`): the
/// endpoint's session type across a session-frame boundary.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProtocolBadge
{
    /// The endpoint (channel), rendered.
    pub channel: String,
    /// The session type before this node, rendered.
    pub before: String,
    /// The session type after this node, rendered.
    pub after: String,
}

impl ProtocolBadge
{
    /// A protocol badge from its parts.
    ///
    /// # Contract
    /// - ensures: fields are stored verbatim.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        channel: String,
        before: String,
        after: String,
    ) -> Self
    {
        Self {
            channel,
            before,
            after,
        }
    }
}

/// Monotone machine steps consumed by a checking run.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "codecs", serde(transparent))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct MachineStepCount(u64);

impl From<u64> for MachineStepCount
{
    #[inline]
    fn from(steps: u64) -> Self
    {
        Self(steps)
    }
}

impl From<MachineStepCount> for u64
{
    #[inline]
    fn from(steps: MachineStepCount) -> Self
    {
        steps.0
    }
}

/// Current frame-stack depth in the typing machine.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "codecs", serde(transparent))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct FrameStackDepth(u32);

impl From<u32> for FrameStackDepth
{
    #[inline]
    fn from(depth: u32) -> Self
    {
        Self(depth)
    }
}

impl From<FrameStackDepth> for u32
{
    #[inline]
    fn from(depth: FrameStackDepth) -> Self
    {
        depth.0
    }
}

/// Outstanding obligation count in the machine summary.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "codecs", serde(transparent))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct OutstandingObligationCount(u32);

impl From<u32> for OutstandingObligationCount
{
    #[inline]
    fn from(count: u32) -> Self
    {
        Self(count)
    }
}

impl From<OutstandingObligationCount> for u32
{
    #[inline]
    fn from(count: OutstandingObligationCount) -> Self
    {
        count.0
    }
}

/// Solver-trail depth in the machine summary.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "codecs", serde(transparent))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct SolverTrailDepth(u32);

impl From<u32> for SolverTrailDepth
{
    #[inline]
    fn from(depth: u32) -> Self
    {
        Self(depth)
    }
}

impl From<SolverTrailDepth> for u32
{
    #[inline]
    fn from(depth: SolverTrailDepth) -> Self
    {
        depth.0
    }
}

/// The scalar machine digest (`typing-machine.md` §3/§5): the
/// `gandr/machineSummary` payload (proposal §3) and a [`MachineView`] field.
///
/// Cheap enough for a status hover; the full state (frames, derivation) is
/// bus-only.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MachineSummary
{
    /// The monotone step counter (`typing-machine.md` §3.2 `steps`).
    pub steps: MachineStepCount,
    /// The frame-stack depth (`typing-machine.md` §3.2 `stack` length).
    pub frame_depth: FrameStackDepth,
    /// The outstanding-obligation count (the reserved slot; `0` today).
    pub obligations: OutstandingObligationCount,
    /// The solver trail depth (`typing-machine.md` §5 `trail` length).
    pub trail_depth: SolverTrailDepth,
}

/// One node-keyed patch in a [`FrameBody::Delta`] (proposal Axis B; the growth
/// path).
///
/// The MVP never emits deltas; this shape is the transparent scaling lever the
/// same frame envelope carries once the incremental machinery lands. Obligation
/// deltas are a future additive variant (the enum is `non_exhaustive`).
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "codecs",
    serde(tag = "kind", content = "data", rename_all = "snake_case")
)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeDelta
{
    /// Replace the subtree rooted at `id` with `node`.
    Updated
    {
        /// The node being replaced.
        id: NodeId,
        /// Its new subtree.
        node: DerivationNode,
    },
    /// Prune the subtree rooted at `id`.
    Removed
    {
        /// The node being pruned.
        id: NodeId,
    },
    /// Insert `node` as a new child of `parent`.
    Inserted
    {
        /// The parent to insert under.
        parent: NodeId,
        /// The inserted subtree.
        node: DerivationNode,
    },
}

#[cfg(test)]
mod tests
{
    use super::ControlView;
    use super::CtxBinding;
    use super::DirView;
    use super::DocId;
    use super::DocVersion;
    use super::FrameBody;
    use super::FrameSummary;
    use super::LayerView;
    use super::MachineView;
    #[cfg(feature = "codecs")]
    use super::NodeDelta;
    #[cfg(feature = "codecs")]
    use super::NodeId;
    #[cfg(feature = "codecs")]
    use super::NodeStep;
    use super::ProtocolBadge;
    use super::RenderFrame;
    use super::ReportView;
    use super::ServerCaps;
    use super::WIRE_SCHEMA_VERSION;

    #[test]
    fn projection_part_constructors_store_fields_verbatim()
    {
        // Distinct field values so a field-swap in any constructor is caught.
        let binding = CtxBinding::new("x".to_owned(), "Nat".to_owned());
        assert_eq!("x", binding.name);
        assert_eq!("Nat", binding.ty);

        let summary = FrameSummary::new(
            "KAppFn".to_owned(),
            "applying f".to_owned(),
            Some("y".to_owned()),
        );
        assert_eq!("KAppFn", summary.role);
        assert_eq!("applying f", summary.prose);
        assert_eq!(Some("y"), summary.binder.as_deref());

        let badge = ProtocolBadge::new("c".to_owned(), "?Nat.end".to_owned(), "end".to_owned());
        assert_eq!("c", badge.channel);
        assert_eq!("?Nat.end", badge.before);
        assert_eq!("end", badge.after);
    }

    #[test]
    fn document_scoped_constructors_populate_routing_keys()
    {
        let frame = RenderFrame::frame(
            "file:///a.gandr".to_owned(),
            DocVersion::from(7_i32),
            ReportView::default(),
            MachineView::default(),
        );
        assert_eq!(WIRE_SCHEMA_VERSION, frame.schema_version());
        let doc_uri = frame
            .doc_uri()
            .expect("document-scoped frame carries a URI");
        assert_eq!("file:///a.gandr", doc_uri.as_ref());
        assert_eq!(Some(DocVersion::from(7_i32)), frame.doc_version());
        assert!(matches!(frame.body(), FrameBody::Frame { .. }));
    }

    #[test]
    fn connection_scoped_constructors_omit_routing_keys()
    {
        let hello = RenderFrame::hello(
            vec![DocId::new(
                "file:///a.gandr".to_owned(),
                DocVersion::from(1_i32),
            )],
            ServerCaps::mvp(),
        );
        assert_eq!(None, hello.doc_uri());
        assert_eq!(None, hello.doc_version());
        let detach = RenderFrame::detach();
        assert_eq!(None, detach.doc_uri());
        assert_eq!(None, detach.doc_version());
        assert!(matches!(detach.body(), FrameBody::Detach));
    }

    #[test]
    fn mvp_capabilities_are_whole_frame_only()
    {
        let caps = ServerCaps::mvp();
        assert_eq!(WIRE_SCHEMA_VERSION, caps.schema_version);
        assert!(!bool::from(caps.deltas));
        assert!(!bool::from(caps.obligations));
        assert!(!bool::from(caps.sessions));
    }

    #[test]
    fn control_and_direction_defaults_are_the_projection_idle_states()
    {
        assert_eq!(ControlView::Done, ControlView::default());
        assert_eq!(DirView::Infer, DirView::default());
        assert_eq!(LayerView::Value, LayerView::default());
    }

    #[cfg(feature = "codecs")]
    #[test]
    fn every_body_variant_round_trips_through_json()
    {
        let frames = [
            RenderFrame::hello(
                vec![DocId::new(
                    "file:///a.gandr".to_owned(),
                    DocVersion::from(1_i32),
                )],
                ServerCaps::mvp(),
            ),
            RenderFrame::frame(
                "file:///a.gandr".to_owned(),
                DocVersion::from(2_i32),
                ReportView::default(),
                MachineView::default(),
            ),
            RenderFrame::delta(
                "file:///a.gandr".to_owned(),
                DocVersion::from(3_i32),
                DocVersion::from(2_i32),
                vec![NodeDelta::Removed {
                    id: NodeId::new(NodeStep::from(41)),
                }],
            ),
            RenderFrame::resync("file:///a.gandr".to_owned(), DocVersion::from(2_i32)),
            RenderFrame::detach(),
        ];
        for frame in frames {
            let json = serde_json::to_string(&frame).expect("serialize");
            let back: RenderFrame = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(frame, back, "round-trip preserves the frame");
        }
    }

    #[cfg(feature = "codecs")]
    #[test]
    fn deserialize_rejects_doc_uri_doc_version_lockstep_violations()
    {
        let mut missing_version = serde_json::to_value(RenderFrame::resync(
            "file:///a.gandr".to_owned(),
            DocVersion::from(2_i32),
        ))
        .expect("serialize resync");
        missing_version["doc_version"] = serde_json::Value::Null;
        let missing_version_err = serde_json::from_value::<RenderFrame>(missing_version)
            .expect_err("doc_uri without doc_version is invalid");
        assert!(
            missing_version_err.to_string().contains("lockstep"),
            "error names the lockstep invariant: {missing_version_err}"
        );

        let mut missing_uri = serde_json::to_value(RenderFrame::resync(
            "file:///a.gandr".to_owned(),
            DocVersion::from(2_i32),
        ))
        .expect("serialize resync");
        missing_uri["doc_uri"] = serde_json::Value::Null;
        let missing_uri_err = serde_json::from_value::<RenderFrame>(missing_uri)
            .expect_err("doc_version without doc_uri is invalid");
        assert!(
            missing_uri_err.to_string().contains("lockstep"),
            "error names the lockstep invariant: {missing_uri_err}"
        );
    }

    #[cfg(feature = "codecs")]
    #[test]
    fn deserialize_rejects_body_routing_mismatches()
    {
        let mut connection_scoped_with_document_keys =
            serde_json::to_value(RenderFrame::hello(Vec::new(), ServerCaps::mvp()))
                .expect("serialize hello");
        connection_scoped_with_document_keys["doc_uri"] =
            serde_json::Value::String("file:///a.gandr".to_owned());
        connection_scoped_with_document_keys["doc_version"] =
            serde_json::Value::Number(serde_json::Number::from(2_i32));
        let connection_scoped_err =
            serde_json::from_value::<RenderFrame>(connection_scoped_with_document_keys)
                .expect_err("hello with document routing is invalid");
        assert!(
            connection_scoped_err
                .to_string()
                .contains("hello render frames"),
            "error names the body/routing mismatch: {connection_scoped_err}"
        );

        let mut document_scoped_without_document_keys = serde_json::to_value(RenderFrame::frame(
            "file:///a.gandr".to_owned(),
            DocVersion::from(2_i32),
            ReportView::default(),
            MachineView::default(),
        ))
        .expect("serialize frame");
        document_scoped_without_document_keys["doc_uri"] = serde_json::Value::Null;
        document_scoped_without_document_keys["doc_version"] = serde_json::Value::Null;
        let document_scoped_err =
            serde_json::from_value::<RenderFrame>(document_scoped_without_document_keys)
                .expect_err("frame without document routing is invalid");
        assert!(
            document_scoped_err
                .to_string()
                .contains("frame render frames"),
            "error names the body/routing mismatch: {document_scoped_err}"
        );
    }

    #[cfg(feature = "codecs")]
    #[test]
    fn frame_body_is_adjacently_tagged_on_the_wire()
    {
        let resync = RenderFrame::resync("file:///a.gandr".to_owned(), DocVersion::from(5_i32));
        let json = serde_json::to_value(&resync).expect("serialize");
        assert_eq!("resync", json["body"]["kind"]);
        assert_eq!(u32::from(WIRE_SCHEMA_VERSION), json["schema_version"]);
        assert_eq!(5_i32, json["doc_version"]);
    }
}
