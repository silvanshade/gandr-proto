use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_term::boundary::GradeBound;
use gandr_core_term::classifier::Classifier;
use gandr_core_term::classifier::SortExpr;
use gandr_core_term::classifier::SortParam;
use gandr_core_term::effect::EffectRow;
use gandr_core_term::error::TypeError;
use gandr_core_term::error::text;
use gandr_core_term::grade::Grade;
use gandr_core_term::prim::NativePrim;
use gandr_core_term::static_term::FamilyApp;
use gandr_core_term::static_term::StaticArg;
use gandr_core_term::static_term::StaticBinder;
use gandr_core_term::static_term::StaticNeutral;
use gandr_core_term::static_term::StaticTerm;
use gandr_core_term::static_term::StaticVar;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::NumLit;
use gandr_core_term::syntax::Side;
use gandr_core_term::syntax::Term;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::CompType;
use gandr_core_term::types::Ty;
use gandr_core_term::types::ValueType;
use gandr_kernel_strata::Level;
use gandr_kernel_strata::LevelConstant;
use gandr_kernel_strata::LevelVar;
use gandr_kernel_strata::LevelVarIndex;

use crate::checkpoint::Checkpoints;
use crate::checkpoint::ItemCheckpoint;
use crate::checkpoint::ItemTyping;
use crate::footprint::Footprint;
use crate::persistence::CheckpointStoreError;
use crate::persistence::UnsupportedPersistence;
use crate::region::Program;

/// Magic and version prefix for canonical program encodings.
const PROGRAM_MAGIC: &[u8; 8] = b"GPRG\0\0\0\x03";
/// Magic and version prefix for canonical checkpoint encodings.
const CHECKPOINT_MAGIC: &[u8; 8] = b"GCP\0\0\0\0\x03";
/// The incremental codec's independent decode-time cap for variable offsets.
///
/// This format has its own identity and does not reuse the kernel export
/// constant. Real universe levels use offsets `0` or `1`; larger values are
/// well-formed but intentionally non-round-tripping until strata exposes an
/// O(1) offset constructor.
const MAX_DECODED_LEVEL_OFFSET: u64 = 4096;
/// Declares the stable one-byte tags in the persistence grammar.
macro_rules! tags {
    ($($name:ident = $value:literal),+ $(,)?) => {
        $(
            #[doc = concat!("Stable persistence tag `", stringify!($name), "`.")]
            const $name: u8 = $value;
        )+
    };
}

tags! {
    PROGRAM = 1,
    CHECKPOINTS = 2,
    ITEM_CHECKPOINT = 3,
    FOOTPRINT = 4,
    TYPING_DEFINITION = 5,
    TYPING_EXPRESSION = 6,
    TYPING_ERROR = 7,
    TYPING_HOLEY = 8,
    ERROR_TYPE_MISMATCH = 9,
    ERROR_SHAPE_MISMATCH = 10,
    ERROR_STUCK = 11,
    ERROR_UNBOUND = 12,
    ERROR_GRADE = 13,
    TERM_VALUE = 14,
    TERM_COMP = 15,
    TY_VALUE = 16,
    TY_COMP = 17,
    VALUE_VAR = 18,
    VALUE_UNIT = 19,
    VALUE_INT = 20,
    VALUE_STR = 21,
    VALUE_NUM = 22,
    VALUE_PAIR = 23,
    VALUE_INJ = 24,
    VALUE_LIST = 25,
    VALUE_RECORD = 26,
    VALUE_THUNK = 27,
    VALUE_ANNOT = 28,
    VALUE_HOLE = 29,
    VALUE_HERE = 30,
    VT_ATOM = 31,
    VT_UNIT = 32,
    VT_PROD = 33,
    VT_SUM = 34,
    VT_LIST = 35,
    VT_RECORD = 36,
    VT_THUNK = 37,
    VT_STK = 38,
    VT_PATH = 39,
    VT_UNIVERSE = 40,
    VT_SIGMA = 41,
    VT_UNKNOWN = 42,
    CT_F = 43,
    CT_ARROW = 44,
    CT_WITH = 45,
    CT_UNKNOWN = 46,
    COMP_ABS = 47,
    COMP_APP = 48,
    COMP_RET = 49,
    COMP_BIND = 50,
    COMP_FORCE = 51,
    COMP_CASE = 52,
    COMP_DATA_CASE = 53,
    COMP_LIST_CASE = 54,
    COMP_SPLIT = 55,
    COMP_RECORD_PROJ = 56,
    COMP_WITH = 57,
    COMP_PRJ = 58,
    COMP_DUP = 59,
    COMP_DROP = 60,
    COMP_RESUME = 61,
    COMP_RESET = 62,
    COMP_SHIFT = 63,
    COMP_HOLE = 64,
    COMP_NATIVE = 65,
    COMP_WALK = 66,
    COMP_FIX = 67,
    VALUE_RUN = 68,
    // CT_PI is the dependent function type, and it is deliberately appended
    // here rather than folded into CT_ARROW: a non-dependent arrow keeps the
    // exact bytes it encoded to before `Π` existed, so every checkpoint an
    // earlier build wrote still decodes and still round-trips canonically.
    // The numbers follow whatever the chain already assigned — a tag is a wire
    // identity, so renumbering an existing one would invalidate every artifact
    // that carries it. This version's family payload is a typed static carrier.
    CT_PI = 69,
    VT_FAMILY = 70,
    CT_FAMILY = 71,
    STATIC_ARG_LEVEL = 72,
    STATIC_ARG_SORT = 73,
    STATIC_ARG_TYPE = 74,
    STATIC_ARG_VALUE = 75,
    STATIC_NEUTRAL_HEAD = 77,
    STATIC_NEUTRAL_APP = 78,
    STATIC_TERM_VAR = 79,
    STATIC_TERM_UNIVERSE = 80,
    STATIC_TERM_QUOTE = 81,
    STATIC_TERM_PI = 82,
    STATIC_TERM_LAM = 83,
    STATIC_TERM_APP = 84,
    STATIC_TERM_NEUTRAL = 85,
}
/// Encodes a lowered program using the canonical persistence grammar.
pub(super) fn encode_program(program: &Program) -> Result<Vec<u8>, CheckpointStoreError>
{
    Encoder::new(PROGRAM_MAGIC).encode(Work::Program(program))
}

/// Encodes a complete checkpoint set using the canonical persistence grammar.
pub(super) fn encode_checkpoints(checkpoints: &Checkpoints)
-> Result<Vec<u8>, CheckpointStoreError>
{
    Encoder::new(CHECKPOINT_MAGIC).encode(Work::Checkpoints(checkpoints))
}

/// Decodes one complete canonical checkpoint set.
pub(super) fn decode_checkpoints(bytes: &[u8]) -> Result<Checkpoints, CheckpointStoreError>
{
    let mut reader = Reader::new(bytes, CHECKPOINT_MAGIC)?;
    let mut nodes = Vec::new();
    while !reader.is_done() {
        decode_token(&mut reader, &mut nodes)?;
    }
    if nodes.len() != 1 {
        return Err(CheckpointStoreError::Corrupt);
    }
    let checkpoints = pop_checkpoints(&mut nodes)?;
    if encode_checkpoints(&checkpoints)? != bytes {
        return Err(CheckpointStoreError::NonCanonical);
    }
    Ok(checkpoints)
}

/// Deferred encoder work used to avoid recursion on semantic trees.
enum Work<'value>
{
    /// Visits a program.
    Program(&'value Program),
    /// Visits a checkpoint set.
    Checkpoints(&'value Checkpoints),
    /// Visits one item checkpoint.
    ItemCheckpoint(&'value ItemCheckpoint),
    /// Visits a dependency footprint.
    Footprint(&'value Footprint),
    /// Visits an item typing result.
    ItemTyping(&'value ItemTyping),
    /// Visits a checker error.
    TypeError(&'value TypeError),
    /// Visits a term.
    Term(&'value Term),
    /// Visits a type.
    Ty(&'value Ty),
    /// Visits a value.
    Value(&'value Value),
    /// Visits a value type.
    ValueType(&'value ValueType),
    /// Visits a computation type.
    CompType(&'value CompType),
    /// Visits a computation.
    Comp(&'value Comp),
    /// Visits a static argument.
    StaticArg(&'value StaticArg),
    /// Visits a static neutral.
    StaticNeutral(&'value StaticNeutral),
    /// Visits a static term.
    StaticTerm(&'value StaticTerm),
    /// Emits a program after its children.
    EmitProgram(&'value Program),
    /// Emits a checkpoint set after its children.
    EmitCheckpoints(&'value Checkpoints),
    /// Emits an item checkpoint after its children.
    EmitItemCheckpoint(&'value ItemCheckpoint),
    /// Emits a footprint after its children.
    EmitFootprint(&'value Footprint),
    /// Emits an item typing result after its children.
    EmitItemTyping(&'value ItemTyping),
    /// Emits a checker error after its children.
    EmitTypeError(&'value TypeError),
    /// Emits a term after its children.
    EmitTerm(&'value Term),
    /// Emits a type after its children.
    EmitTy(&'value Ty),
    /// Emits a value after its children.
    EmitValue(&'value Value),
    /// Emits a value type after its children.
    EmitValueType(&'value ValueType),
    /// Emits a computation type after its children.
    EmitCompType(&'value CompType),
    /// Emits a computation after its children.
    EmitComp(&'value Comp),
    /// Emits a static argument after its children.
    EmitStaticArg(&'value StaticArg),
    /// Emits a static neutral after its children.
    EmitStaticNeutral(&'value StaticNeutral),
    /// Emits a static term after its children.
    EmitStaticTerm(&'value StaticTerm),
}

/// Iterative canonical encoder with an explicit semantic work stack.
struct Encoder<'value>
{
    /// Canonical bytes emitted so far.
    bytes: Vec<u8>,
    /// Pending semantic traversal and emission steps.
    work: Vec<Work<'value>>,
}

impl<'value> Encoder<'value>
{
    /// Starts an encoder with the format's magic and version prefix.
    fn new(magic: &[u8]) -> Self
    {
        Self {
            bytes: magic.to_vec(),
            work: Vec::new(),
        }
    }

    /// Encodes one root and all transitively reachable supported content.
    fn encode(
        mut self,
        root: Work<'value>,
    ) -> Result<Vec<u8>, CheckpointStoreError>
    {
        self.work.push(root);
        while let Some(work) = self.work.pop() {
            match work {
                | Work::Program(value) => self.visit_program(value),
                | Work::Checkpoints(value) => self.visit_checkpoints(value),
                | Work::ItemCheckpoint(value) => self.visit_item_checkpoint(value),
                | Work::Footprint(value) => self.work.push(Work::EmitFootprint(value)),
                | Work::ItemTyping(value) => self.visit_item_typing(value),
                | Work::TypeError(value) => self.visit_type_error(value),
                | Work::Term(value) => self.visit_term(value),
                | Work::Ty(value) => self.visit_ty(value),
                | Work::Value(value) => self.visit_value(value)?,
                | Work::ValueType(value) => self.visit_value_type(value)?,
                | Work::CompType(value) => self.visit_comp_type(value)?,
                | Work::Comp(value) => self.visit_comp(value)?,
                | Work::StaticArg(value) => self.visit_static_arg(value),
                | Work::StaticNeutral(value) => self.visit_static_neutral(value),
                | Work::StaticTerm(value) => self.visit_static_term(value),
                | Work::EmitProgram(value) => self.emit_program(value)?,
                | Work::EmitCheckpoints(value) => self.emit_checkpoints(value)?,
                | Work::EmitItemCheckpoint(value) => self.emit_item_checkpoint(value)?,
                | Work::EmitFootprint(value) => self.emit_footprint(value)?,
                | Work::EmitItemTyping(value) => self.emit_item_typing(value)?,
                | Work::EmitTypeError(value) => self.emit_type_error(value)?,
                | Work::EmitTerm(value) => self.emit_term(value),
                | Work::EmitTy(value) => self.emit_ty(value),
                | Work::EmitValue(value) => self.emit_value(value)?,
                | Work::EmitValueType(value) => self.emit_value_type(value)?,
                | Work::EmitCompType(value) => self.emit_comp_type(value)?,
                | Work::EmitComp(value) => self.emit_comp(value)?,
                | Work::EmitStaticArg(value) => self.emit_static_arg(value)?,
                | Work::EmitStaticNeutral(value) => self.emit_static_neutral(value)?,
                | Work::EmitStaticTerm(value) => self.emit_static_term(value)?,
            }
        }
        Ok(self.bytes)
    }

    /// Schedules a program's children in source order.
    fn visit_program(
        &mut self,
        value: &'value Program,
    )
    {
        self.work.push(Work::EmitProgram(value));
        for item in value.items.iter().rev() {
            if let Some(ascription) = item.ascription.as_ref() {
                self.work.push(Work::Ty(ascription));
            }
            self.work.push(Work::Term(&item.term));
        }
    }

    /// Schedules checkpoint items in source order.
    fn visit_checkpoints(
        &mut self,
        value: &'value Checkpoints,
    )
    {
        self.work.push(Work::EmitCheckpoints(value));
        for item in value.items.iter().rev() {
            self.work.push(Work::ItemCheckpoint(item));
        }
    }

    /// Schedules the fields of one item checkpoint.
    fn visit_item_checkpoint(
        &mut self,
        value: &'value ItemCheckpoint,
    )
    {
        self.work.push(Work::EmitItemCheckpoint(value));
        self.work.push(Work::ItemTyping(&value.typing));
        self.work.push(Work::Footprint(&value.footprint));
        self.work.push(Work::Term(&value.term));
        if let Some(ascription) = value.ascription.as_ref() {
            self.work.push(Work::Ty(ascription));
        }
    }

    /// Schedules the type-bearing fields of an item result.
    fn visit_item_typing(
        &mut self,
        value: &'value ItemTyping,
    )
    {
        self.work.push(Work::EmitItemTyping(value));
        match *value {
            | ItemTyping::Definition { ref ty, .. } | ItemTyping::Expression { ref ty } => {
                self.work.push(Work::Ty(ty));
            },
            | ItemTyping::TypeError { ref error } => self.work.push(Work::TypeError(error)),
            | ItemTyping::Holey => {},
        }
    }

    /// Schedules the semantic payload of a checker error.
    fn visit_type_error(
        &mut self,
        value: &'value TypeError,
    )
    {
        self.work.push(Work::EmitTypeError(value));
        match *value {
            | TypeError::TypeMismatch(ref mismatch) => {
                self.work.push(Work::Ty(&mismatch.expected));
                self.work.push(Work::Ty(&mismatch.actual));
            },
            | TypeError::ShapeMismatch { ref actual, .. } => self.work.push(Work::Ty(actual)),
            | TypeError::StuckExpr { ref expr, .. } => self.work.push(Work::Term(expr)),
            // The refusal carries no `Ty` or `Term` payload to schedule; the
            // emit pass below declines it outright.
            | TypeError::UnboundVariable { .. }
            | TypeError::GradeError { .. }
            | TypeError::IllFormedType(_) => {},
        }
    }

    /// Schedules a term payload.
    fn visit_term(
        &mut self,
        value: &'value Term,
    )
    {
        self.work.push(Work::EmitTerm(value));
        match *value {
            | Term::Value(ref value) => self.work.push(Work::Value(value)),
            | Term::Comp(ref comp) => self.work.push(Work::Comp(comp)),
        }
    }

    /// Schedules a type payload.
    fn visit_ty(
        &mut self,
        value: &'value Ty,
    )
    {
        self.work.push(Work::EmitTy(value));
        match *value {
            | Ty::Value(ref ty) => self.work.push(Work::ValueType(ty)),
            | Ty::Comp(ref ty) => self.work.push(Work::CompType(ty)),
        }
    }

    /// Schedules a value payload or reports its exact unsupported form.
    fn visit_value(
        &mut self,
        value: &'value Value,
    ) -> Result<(), CheckpointStoreError>
    {
        match *value {
            | Value::Stk(_) => {
                return Err(unsupported(UnsupportedPersistence::OpaqueStack));
            },
            | Value::Ctor { .. } => {
                return Err(unsupported(UnsupportedPersistence::DataIdSerial));
            },
            | Value::Pack { .. } => {
                return Err(unsupported(UnsupportedPersistence::PackageFormer));
            },
            | _ => {},
        }
        self.work.push(Work::EmitValue(value));
        match *value {
            | Value::Pair(ref fst, ref snd) => {
                self.work.push(Work::Value(snd));
                self.work.push(Work::Value(fst));
            },
            | Value::Inj(_, ref payload) | Value::Here(ref payload) => {
                self.work.push(Work::Value(payload));
            },
            | Value::List(ref elements) => {
                for element in elements.iter().rev() {
                    self.work.push(Work::Value(element));
                }
            },
            | Value::Record(ref fields) => {
                for field in fields.values().rev() {
                    self.work.push(Work::Value(field));
                }
            },
            | Value::Thunk(_, ref body) | Value::Run(ref body) => {
                self.work.push(Work::Comp(body));
            },
            | Value::Annot(ref inner, ref ty) => {
                self.work.push(Work::ValueType(ty));
                self.work.push(Work::Value(inner));
            },
            | Value::Var(_)
            | Value::Unit
            | Value::Int(_)
            | Value::Str(_)
            | Value::Num(_)
            | Value::Hole(_)
            | Value::Stk(_)
            | Value::Ctor { .. }
            | Value::Pack { .. } => {},
        }
        Ok(())
    }

    /// Schedules a value-type payload or reports process-local nominal
    /// identifiers.
    fn visit_value_type(
        &mut self,
        value: &'value ValueType,
    ) -> Result<(), CheckpointStoreError>
    {
        match *value {
            | ValueType::Data { .. } => {
                return Err(unsupported(UnsupportedPersistence::DataIdSerial));
            },
            | ValueType::Sealed(_) => {
                return Err(unsupported(UnsupportedPersistence::SealIdSerial));
            },
            | ValueType::Package { .. } => {
                return Err(unsupported(UnsupportedPersistence::PackageFormer));
            },
            | _ => {},
        }
        self.work.push(Work::EmitValueType(value));
        match *value {
            | ValueType::Prod(ref fst, ref snd)
            | ValueType::Sum(ref fst, ref snd)
            | ValueType::Sigma {
                ref fst, ref snd, ..
            } => {
                self.work.push(Work::ValueType(snd));
                self.work.push(Work::ValueType(fst));
            },
            | ValueType::List(ref element) => self.work.push(Work::ValueType(element)),
            | ValueType::Record(ref fields) => {
                for field in fields.values().rev() {
                    self.work.push(Work::ValueType(field));
                }
            },
            | ValueType::Thunk(_, ref body) => self.work.push(Work::CompType(body)),
            | ValueType::Stk(ref consumes, ref delivers) => {
                self.work.push(Work::CompType(delivers));
                self.work.push(Work::CompType(consumes));
            },
            | ValueType::Path {
                ref ty,
                ref lhs,
                ref rhs,
            } => {
                self.work.push(Work::Value(rhs));
                self.work.push(Work::Value(lhs));
                self.work.push(Work::ValueType(ty));
            },
            | ValueType::Family(ref application) => {
                self.work.push(Work::StaticNeutral(application.neutral()));
            },
            | ValueType::Atom(_)
            | ValueType::Unit
            | ValueType::Universe { .. }
            | ValueType::Unknown
            | ValueType::Data { .. }
            | ValueType::Sealed(_)
            | ValueType::Package { .. } => {},
        }
        Ok(())
    }

    /// Schedules a computation type or rejects a non-empty opaque effect row.
    fn visit_comp_type(
        &mut self,
        value: &'value CompType,
    ) -> Result<(), CheckpointStoreError>
    {
        if let CompType::F(_, ref row) = *value
            && !bool::from(row.is_empty())
        {
            return Err(unsupported(UnsupportedPersistence::OpaqueEffectSignature));
        }
        self.work.push(Work::EmitCompType(value));
        match *value {
            | CompType::F(ref of, _) => self.work.push(Work::ValueType(of)),
            | CompType::Arrow {
                ref arg, ref res, ..
            } => {
                self.work.push(Work::CompType(res));
                self.work.push(Work::ValueType(arg));
            },
            | CompType::With(ref fst, ref snd) => {
                self.work.push(Work::CompType(snd));
                self.work.push(Work::CompType(fst));
            },
            | CompType::Family(ref application) => {
                self.work.push(Work::StaticNeutral(application.neutral()));
            },
            | CompType::Unknown => {},
        }
        Ok(())
    }

    /// Schedules one static argument and its recursive payload.
    fn visit_static_arg(
        &mut self,
        value: &'value StaticArg,
    )
    {
        self.work.push(Work::EmitStaticArg(value));
        match value {
            | &StaticArg::Level(_) | &StaticArg::Sort(_) => {},
            | &StaticArg::Type(ref term) => self.work.push(Work::StaticTerm(term)),
            | &StaticArg::Value(ref value) => self.work.push(Work::Value(value)),
        }
    }

    /// Schedules one static neutral without recursing through Rust call frames.
    fn visit_static_neutral(
        &mut self,
        value: &'value StaticNeutral,
    )
    {
        self.work.push(Work::EmitStaticNeutral(value));
        match value {
            | &StaticNeutral::Head(_) => {},
            | &StaticNeutral::App {
                ref head,
                ref argument,
            } => {
                self.work.push(Work::StaticArg(argument));
                self.work.push(Work::StaticNeutral(head));
            },
        }
    }

    /// Schedules one static term and its recursive payload.
    fn visit_static_term(
        &mut self,
        value: &'value StaticTerm,
    )
    {
        self.work.push(Work::EmitStaticTerm(value));
        match value {
            | &StaticTerm::Var(_) | &StaticTerm::Universe(_) => {},
            | &StaticTerm::Quote(ref ty) => self.work.push(Work::Ty(ty)),
            | &StaticTerm::Neutral(ref neutral) => {
                self.work.push(Work::StaticNeutral(neutral));
            },
            | &StaticTerm::Pi { ref codomain, .. }
            | &StaticTerm::Lam {
                body: ref codomain, ..
            } => {
                self.work.push(Work::StaticTerm(codomain));
            },
            | &StaticTerm::App {
                ref function,
                ref argument,
            } => {
                self.work.push(Work::StaticArg(argument));
                self.work.push(Work::StaticTerm(function));
            },
        }
    }

    /// Schedules a computation or rejects opaque effect operations and
    /// handlers.
    fn visit_comp(
        &mut self,
        value: &'value Comp,
    ) -> Result<(), CheckpointStoreError>
    {
        match *value {
            | Comp::Perform { .. } | Comp::Handle { .. } => {
                return Err(unsupported(UnsupportedPersistence::OpaqueEffectSignature));
            },
            // The atoms an elimination minted are mint-order seal serials, so
            // this one is process-local for exactly the reason a sealed type is
            // — the encoding gap is the second obstacle, not the first.
            | Comp::Unpack { .. } => {
                return Err(unsupported(UnsupportedPersistence::SealIdSerial));
            },
            | _ => {},
        }
        self.work.push(Work::EmitComp(value));
        match *value {
            | Comp::Abs(_, ref annotation, ref body) => {
                self.work.push(Work::Comp(body));
                if let Some(annotation) = annotation.as_ref() {
                    self.work.push(Work::ValueType(annotation));
                }
            },
            | Comp::App(ref fun, ref arg) => {
                self.work.push(Work::Value(arg));
                self.work.push(Work::Comp(fun));
            },
            | Comp::Ret(ref value)
            | Comp::Force(ref value)
            | Comp::Dup(ref value)
            | Comp::Drop(ref value) => self.work.push(Work::Value(value)),
            | Comp::Bind(ref bound, _, ref body) => {
                self.work.push(Work::Comp(body));
                self.work.push(Work::Comp(bound));
            },
            | Comp::Case(ref scrut, (_, ref fst), (_, ref snd)) => {
                self.work.push(Work::Comp(snd));
                self.work.push(Work::Comp(fst));
                self.work.push(Work::Value(scrut));
            },
            | Comp::DataCase(ref scrut, ref arms) => {
                for arm in arms.iter().rev() {
                    self.work.push(Work::Comp(&arm.1));
                }
                self.work.push(Work::Value(scrut));
            },
            | Comp::ListCase {
                ref scrut,
                ref nil,
                ref cons,
                ..
            } => {
                self.work.push(Work::Comp(cons));
                self.work.push(Work::Comp(nil));
                self.work.push(Work::Value(scrut));
            },
            | Comp::Split {
                ref scrut,
                ref motive,
                ref body,
                ..
            } => {
                self.work.push(Work::Comp(body));
                if let Some(motive) = motive.as_ref() {
                    self.work.push(Work::CompType(&motive.body));
                }
                self.work.push(Work::Value(scrut));
            },
            | Comp::RecordProj { ref record, .. } => self.work.push(Work::Value(record)),
            | Comp::With(ref fst, ref snd) => {
                self.work.push(Work::Comp(snd));
                self.work.push(Work::Comp(fst));
            },
            | Comp::Prj(_, ref comp) | Comp::Reset(ref comp) => self.work.push(Work::Comp(comp)),
            | Comp::Resume(ref stack, ref comp) => {
                self.work.push(Work::Comp(comp));
                self.work.push(Work::Value(stack));
            },
            | Comp::Shift(_, ref body) | Comp::Fix(_, ref body) => {
                self.work.push(Work::Comp(body));
            },
            | Comp::Native { ref args, .. } => {
                for arg in args.iter().rev() {
                    self.work.push(Work::Value(arg));
                }
            },
            | Comp::Walk {
                ref scrut,
                ref motive,
                ref base,
            } => {
                self.work.push(Work::Comp(&base.body));
                self.work.push(Work::CompType(&motive.body));
                self.work.push(Work::Value(scrut));
            },
            | Comp::Hole(_) | Comp::Perform { .. } | Comp::Handle { .. } | Comp::Unpack { .. } => {
            },
        }
        Ok(())
    }

    /// Emits program metadata after its terms and optional ascriptions.
    fn emit_program(
        &mut self,
        value: &Program,
    ) -> Result<(), CheckpointStoreError>
    {
        self.byte(PROGRAM);
        self.len(value.items.len())?;
        for item in &value.items {
            self.option_string(item.name.as_deref())?;
            self.boolean(item.ascription.is_some());
        }
        Ok(())
    }

    /// Emits the checkpoint sequence header.
    fn emit_checkpoints(
        &mut self,
        value: &Checkpoints,
    ) -> Result<(), CheckpointStoreError>
    {
        self.byte(CHECKPOINTS);
        self.len(value.items.len())
    }

    /// Emits item metadata after its semantic fields.
    fn emit_item_checkpoint(
        &mut self,
        value: &ItemCheckpoint,
    ) -> Result<(), CheckpointStoreError>
    {
        self.byte(ITEM_CHECKPOINT);
        self.option_string(value.name.as_deref())?;
        self.boolean(value.ascription.is_some());
        Ok(())
    }

    /// Emits every dependency-footprint field.
    fn emit_footprint(
        &mut self,
        value: &Footprint,
    ) -> Result<(), CheckpointStoreError>
    {
        self.byte(FOOTPRINT);
        self.len(value.names.len())?;
        for name in &value.names {
            self.string(name)?;
        }
        self.boolean(value.opaque);
        self.boolean(value.has_hole);
        Ok(())
    }

    /// Emits the stable variant and scalar fields of an item typing result.
    fn emit_item_typing(
        &mut self,
        value: &ItemTyping,
    ) -> Result<(), CheckpointStoreError>
    {
        match *value {
            | ItemTyping::Definition {
                ref name, bound, ..
            } => {
                self.byte(TYPING_DEFINITION);
                self.string(name)?;
                self.boolean(bound);
            },
            | ItemTyping::Expression { .. } => self.byte(TYPING_EXPRESSION),
            | ItemTyping::TypeError { .. } => self.byte(TYPING_ERROR),
            | ItemTyping::Holey => self.byte(TYPING_HOLEY),
        }
        Ok(())
    }

    /// Emits the stable variant and scalar fields of a checker error.
    fn emit_type_error(
        &mut self,
        value: &TypeError,
    ) -> Result<(), CheckpointStoreError>
    {
        match *value {
            | TypeError::TypeMismatch(..) => self.byte(ERROR_TYPE_MISMATCH),
            | TypeError::ShapeMismatch { expected, .. } => {
                self.byte(ERROR_SHAPE_MISMATCH);
                self.byte(error_text_tag(expected)?);
            },
            | TypeError::StuckExpr { hint, .. } => {
                self.byte(ERROR_STUCK);
                self.byte(error_text_tag(hint)?);
            },
            | TypeError::UnboundVariable { ref name } => {
                self.byte(ERROR_UNBOUND);
                self.string(name)?;
            },
            | TypeError::GradeError { lower, upper } => {
                self.byte(ERROR_GRADE);
                self.grade(lower);
                self.grade(upper);
            },
            | TypeError::IllFormedType(_) => {
                return Err(unsupported(UnsupportedPersistence::FormationRefusal));
            },
        }
        Ok(())
    }

    /// Emits a term variant tag.
    fn emit_term(
        &mut self,
        value: &Term,
    )
    {
        self.byte(match *value {
            | Term::Value(_) => TERM_VALUE,
            | Term::Comp(_) => TERM_COMP,
        });
    }

    /// Emits a type variant tag.
    fn emit_ty(
        &mut self,
        value: &Ty,
    )
    {
        self.byte(match *value {
            | Ty::Value(_) => TY_VALUE,
            | Ty::Comp(_) => TY_COMP,
        });
    }

    /// Emits the stable variant and scalar fields of a value.
    fn emit_value(
        &mut self,
        value: &Value,
    ) -> Result<(), CheckpointStoreError>
    {
        match *value {
            | Value::Var(ref name) => {
                self.byte(VALUE_VAR);
                self.string(name)?;
            },
            | Value::Unit => self.byte(VALUE_UNIT),
            | Value::Int(value) => {
                self.byte(VALUE_INT);
                self.bytes.extend_from_slice(&value.to_le_bytes());
            },
            | Value::Str(ref value) => {
                self.byte(VALUE_STR);
                self.string(value)?;
            },
            | Value::Num(value) => {
                self.byte(VALUE_NUM);
                match value {
                    | NumLit::U32(value) => {
                        self.byte(0);
                        self.bytes.extend_from_slice(&value.to_le_bytes());
                    },
                    | NumLit::U64(value) => {
                        self.byte(1);
                        self.bytes.extend_from_slice(&value.to_le_bytes());
                    },
                    | NumLit::I32(value) => {
                        self.byte(2);
                        self.bytes.extend_from_slice(&value.to_le_bytes());
                    },
                    | NumLit::I64(value) => {
                        self.byte(3);
                        self.bytes.extend_from_slice(&value.to_le_bytes());
                    },
                    | NumLit::F32(value) => {
                        self.byte(4);
                        self.bytes.extend_from_slice(&value.to_le_bytes());
                    },
                    | NumLit::F64(value) => {
                        self.byte(5);
                        self.bytes.extend_from_slice(&value.to_le_bytes());
                    },
                }
            },
            | Value::Pair(..) => self.byte(VALUE_PAIR),
            | Value::Inj(side, _) => {
                self.byte(VALUE_INJ);
                self.side(side);
            },
            | Value::List(ref elements) => {
                self.byte(VALUE_LIST);
                self.len(elements.len())?;
            },
            | Value::Record(ref fields) => {
                self.byte(VALUE_RECORD);
                self.len(fields.len())?;
                for label in fields.keys() {
                    self.string(label)?;
                }
            },
            | Value::Thunk(grade, _) => {
                self.byte(VALUE_THUNK);
                self.grade(grade);
            },
            | Value::Run(_) => self.byte(VALUE_RUN),
            | Value::Annot(..) => self.byte(VALUE_ANNOT),
            | Value::Hole(hole) => {
                self.byte(VALUE_HOLE);
                self.bytes.extend_from_slice(&hole.to_le_bytes());
            },
            | Value::Here(_) => self.byte(VALUE_HERE),
            | Value::Stk(_) | Value::Ctor { .. } | Value::Pack { .. } => {
                return Err(CheckpointStoreError::Rejected);
            },
        }
        Ok(())
    }

    /// Emits the stable variant and scalar fields of a value type.
    fn emit_value_type(
        &mut self,
        value: &ValueType,
    ) -> Result<(), CheckpointStoreError>
    {
        match *value {
            | ValueType::Atom(ref name) => {
                self.byte(VT_ATOM);
                self.string(name)?;
            },
            | ValueType::Unit => self.byte(VT_UNIT),
            | ValueType::Prod(..) => self.byte(VT_PROD),
            | ValueType::Sum(..) => self.byte(VT_SUM),
            | ValueType::List(_) => self.byte(VT_LIST),
            | ValueType::Record(ref fields) => {
                self.byte(VT_RECORD);
                self.len(fields.len())?;
                for label in fields.keys() {
                    self.string(label)?;
                }
            },
            | ValueType::Thunk(grade, _) => {
                self.byte(VT_THUNK);
                self.grade(grade);
            },
            | ValueType::Stk(..) => self.byte(VT_STK),
            | ValueType::Path { .. } => self.byte(VT_PATH),
            | ValueType::Universe {
                ref sort,
                ref level,
            } => {
                self.byte(VT_UNIVERSE);
                self.sort(sort)?;
                self.level(level)?;
            },
            | ValueType::Family(ref application) => {
                self.byte(VT_FAMILY);
                self.classifier(application.result())?;
            },
            | ValueType::Sigma { ref binder, .. } => {
                self.byte(VT_SIGMA);
                self.string(binder)?;
            },
            | ValueType::Unknown => self.byte(VT_UNKNOWN),
            | ValueType::Data { .. } | ValueType::Sealed(_) | ValueType::Package { .. } => {
                return Err(CheckpointStoreError::Rejected);
            },
        }
        Ok(())
    }

    /// Emits a computation-type variant tag, and a dependent function type's
    /// binder beside it.
    ///
    /// Fallible only because a binder is a string, and a string is the one
    /// payload this encoder can refuse (an over-long one). The tag itself never
    /// fails.
    fn emit_comp_type(
        &mut self,
        value: &CompType,
    ) -> Result<(), CheckpointStoreError>
    {
        match *value {
            | CompType::F(..) => self.byte(CT_F),
            | CompType::Arrow { binder: None, .. } => self.byte(CT_ARROW),
            | CompType::Arrow {
                binder: Some(ref binder),
                ..
            } => {
                self.byte(CT_PI);
                self.string(binder)?;
            },
            | CompType::With(..) => self.byte(CT_WITH),
            | CompType::Family(ref application) => {
                self.byte(CT_FAMILY);
                self.classifier(application.result())?;
            },
            | CompType::Unknown => self.byte(CT_UNKNOWN),
        }
        Ok(())
    }

    /// Emits one static argument after its recursive payload.
    fn emit_static_arg(
        &mut self,
        value: &StaticArg,
    ) -> Result<(), CheckpointStoreError>
    {
        match value {
            | &StaticArg::Level(ref level) => {
                self.byte(STATIC_ARG_LEVEL);
                self.level(level)?;
            },
            | &StaticArg::Sort(ref sort) => {
                self.byte(STATIC_ARG_SORT);
                self.sort(sort)?;
            },
            | &StaticArg::Type(_) => self.byte(STATIC_ARG_TYPE),
            | &StaticArg::Value(_) => self.byte(STATIC_ARG_VALUE),
        }
        Ok(())
    }

    /// Emits one static neutral after its recursive payload.
    fn emit_static_neutral(
        &mut self,
        value: &StaticNeutral,
    ) -> Result<(), CheckpointStoreError>
    {
        match value {
            | &StaticNeutral::Head(ref variable) => {
                self.byte(STATIC_NEUTRAL_HEAD);
                self.string(variable.name().as_ref())?;
            },
            | &StaticNeutral::App { .. } => self.byte(STATIC_NEUTRAL_APP),
        }
        Ok(())
    }

    /// Emits one static term after its recursive payload.
    fn emit_static_term(
        &mut self,
        value: &StaticTerm,
    ) -> Result<(), CheckpointStoreError>
    {
        match value {
            | &StaticTerm::Var(ref variable) => {
                self.byte(STATIC_TERM_VAR);
                self.string(variable.name().as_ref())?;
            },
            | &StaticTerm::Universe(ref classifier) => {
                self.byte(STATIC_TERM_UNIVERSE);
                self.classifier(classifier)?;
            },
            | &StaticTerm::Quote(_) => self.byte(STATIC_TERM_QUOTE),
            | &StaticTerm::Pi { ref binder, .. } => {
                self.byte(STATIC_TERM_PI);
                self.string(binder.variable().name().as_ref())?;
                self.classifier(binder.classifier())?;
            },
            | &StaticTerm::Lam { ref binder, .. } => {
                self.byte(STATIC_TERM_LAM);
                self.string(binder.variable().name().as_ref())?;
                self.classifier(binder.classifier())?;
            },
            | &StaticTerm::App { .. } => self.byte(STATIC_TERM_APP),
            | &StaticTerm::Neutral(_) => self.byte(STATIC_TERM_NEUTRAL),
        }
        Ok(())
    }

    /// Emits a classifier's sort and level in canonical order.
    fn classifier(
        &mut self,
        value: &Classifier,
    ) -> Result<(), CheckpointStoreError>
    {
        self.sort(value.sort())?;
        self.level(value.level())
    }

    /// Emits the stable variant and scalar fields of a computation.
    fn emit_comp(
        &mut self,
        value: &Comp,
    ) -> Result<(), CheckpointStoreError>
    {
        match *value {
            | Comp::Abs(ref binder, ref annotation, _) => {
                self.byte(COMP_ABS);
                self.string(binder)?;
                self.boolean(annotation.is_some());
            },
            | Comp::App(..) => self.byte(COMP_APP),
            | Comp::Ret(_) => self.byte(COMP_RET),
            | Comp::Bind(_, ref binder, _) => {
                self.byte(COMP_BIND);
                self.string(binder)?;
            },
            | Comp::Force(_) => self.byte(COMP_FORCE),
            | Comp::Case(_, (ref fst_name, _), (ref snd_name, _)) => {
                self.byte(COMP_CASE);
                self.string(fst_name)?;
                self.string(snd_name)?;
            },
            | Comp::DataCase(_, ref arms) => {
                self.byte(COMP_DATA_CASE);
                self.len(arms.len())?;
                for arm in arms {
                    self.string(&arm.0)?;
                }
            },
            | Comp::ListCase {
                ref head, ref tail, ..
            } => {
                self.byte(COMP_LIST_CASE);
                self.string(head)?;
                self.string(tail)?;
            },
            | Comp::Split {
                ref fst_name,
                ref snd_name,
                ref motive,
                ..
            } => {
                self.byte(COMP_SPLIT);
                self.string(fst_name)?;
                self.string(snd_name)?;
                self.boolean(motive.is_some());
                if let Some(motive) = motive.as_ref() {
                    self.string(&motive.binder)?;
                }
            },
            | Comp::RecordProj { ref label, .. } => {
                self.byte(COMP_RECORD_PROJ);
                self.string(label)?;
            },
            | Comp::With(..) => self.byte(COMP_WITH),
            | Comp::Prj(side, _) => {
                self.byte(COMP_PRJ);
                self.side(side);
            },
            | Comp::Dup(_) => self.byte(COMP_DUP),
            | Comp::Drop(_) => self.byte(COMP_DROP),
            | Comp::Resume(..) => self.byte(COMP_RESUME),
            | Comp::Reset(_) => self.byte(COMP_RESET),
            | Comp::Shift(ref binder, _) => {
                self.byte(COMP_SHIFT);
                self.string(binder)?;
            },
            | Comp::Fix(ref binder, _) => {
                self.byte(COMP_FIX);
                self.string(binder)?;
            },
            | Comp::Hole(hole) => {
                self.byte(COMP_HOLE);
                self.bytes.extend_from_slice(&hole.to_le_bytes());
            },
            | Comp::Native { prim, ref args } => {
                self.byte(COMP_NATIVE);
                self.byte(native_tag(prim));
                self.len(args.len())?;
            },
            | Comp::Walk {
                ref motive,
                ref base,
                ..
            } => {
                self.byte(COMP_WALK);
                self.string(&motive.x)?;
                self.string(&motive.y)?;
                self.string(&motive.q)?;
                self.string(&base.x)?;
            },
            | Comp::Perform { .. } | Comp::Handle { .. } | Comp::Unpack { .. } => {
                return Err(CheckpointStoreError::Rejected);
            },
        }
        Ok(())
    }

    /// Appends one variant or scalar byte.
    fn byte(
        &mut self,
        byte: u8,
    )
    {
        self.bytes.push(byte);
    }

    /// Appends a canonical Boolean byte.
    fn boolean(
        &mut self,
        value: bool,
    )
    {
        self.byte(u8::from(value));
    }

    /// Appends a canonical side tag.
    fn side(
        &mut self,
        value: Side,
    )
    {
        self.byte(match value {
            | Side::Fst => 0,
            | Side::Snd => 1,
        });
    }

    /// Appends a checked fixed-width sequence length.
    fn len(
        &mut self,
        value: usize,
    ) -> Result<(), CheckpointStoreError>
    {
        let value = u32::try_from(value).map_err(|_error| CheckpointStoreError::Rejected)?;
        self.bytes.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    /// Appends a length-prefixed UTF-8 string.
    fn string(
        &mut self,
        value: &str,
    ) -> Result<(), CheckpointStoreError>
    {
        self.len(value.len())?;
        self.bytes.extend_from_slice(value.as_bytes());
        Ok(())
    }

    /// Appends an optional length-prefixed UTF-8 string.
    fn option_string(
        &mut self,
        value: Option<&str>,
    ) -> Result<(), CheckpointStoreError>
    {
        self.boolean(value.is_some());
        if let Some(value) = value {
            self.string(value)?;
        }
        Ok(())
    }

    /// Appends the canonical finite-or-omega grade representation.
    fn grade(
        &mut self,
        value: Grade,
    )
    {
        if bool::from(Grade::OMEGA.leq(value)) {
            self.byte(1);
        }
        else {
            self.byte(0);
            self.bytes
                .extend_from_slice(&grade_bound(value).to_le_bytes());
        }
    }
    /// Appends a classifier sort in the canonical grammar.
    fn sort(
        &mut self,
        value: &SortExpr,
    ) -> Result<(), CheckpointStoreError>
    {
        match *value {
            | SortExpr::Ground(sort) => self.byte(match sort {
                | gandr_core_term::classifier::GroundSort::Value => 0,
                | gandr_core_term::classifier::GroundSort::Computation => 1,
            }),
            | SortExpr::Param(ref param) => {
                self.byte(2);
                self.string(param.name().as_ref())?;
            },
        }
        Ok(())
    }

    /// Appends one canonical kernel-strata level.
    fn level(
        &mut self,
        value: &Level,
    ) -> Result<(), CheckpointStoreError>
    {
        self.bytes
            .extend_from_slice(&u64::from(value.constant_part()).to_le_bytes());
        let atoms = value.atoms().collect::<Vec<_>>();
        self.len(atoms.len())?;
        for (variable, offset) in atoms {
            self.bytes
                .extend_from_slice(&u32::from(variable.index()).to_le_bytes());
            self.bytes
                .extend_from_slice(&u64::from(offset).to_le_bytes());
        }
        Ok(())
    }
}

/// Recovers the finite grade bound through the public grade order.
fn grade_bound(value: Grade) -> u64
{
    let mut low = 0_u64;
    let mut high = u64::MAX;
    while low < high {
        let midpoint = low.saturating_add(high.saturating_sub(low) >> 1);
        if bool::from(value.leq(Grade::fin(GradeBound::from(midpoint)))) {
            high = midpoint;
        }
        else {
            low = midpoint.saturating_add(1);
        }
    }
    low
}

/// Wraps one exact unsupported-form reason.
fn unsupported(reason: UnsupportedPersistence) -> CheckpointStoreError
{
    CheckpointStoreError::UnsupportedPersistence(reason)
}

/// Bounds-checked cursor over a canonical byte stream.
struct Reader<'bytes>
{
    /// Full encoded byte stream.
    bytes: &'bytes [u8],
    /// Next unread byte offset.
    cursor: usize,
}

impl<'bytes> Reader<'bytes>
{
    /// Validates a format prefix and starts after it.
    fn new(
        bytes: &'bytes [u8],
        magic: &[u8],
    ) -> Result<Self, CheckpointStoreError>
    {
        if bytes.get(.. magic.len()) != Some(magic) {
            return Err(CheckpointStoreError::Corrupt);
        }
        Ok(Self {
            bytes,
            cursor: magic.len(),
        })
    }

    /// Reports whether every byte has been consumed exactly.
    fn is_done(&self) -> bool
    {
        self.cursor == self.bytes.len()
    }
    /// Returns the number of unread bytes.
    fn remaining(&self) -> usize
    {
        self.bytes.len().saturating_sub(self.cursor)
    }

    /// Takes one checked-width slice and advances the cursor.
    fn take(
        &mut self,
        width: usize,
    ) -> Result<&'bytes [u8], CheckpointStoreError>
    {
        let end = self
            .cursor
            .checked_add(width)
            .ok_or(CheckpointStoreError::Corrupt)?;
        let value = self
            .bytes
            .get(self.cursor .. end)
            .ok_or(CheckpointStoreError::Corrupt)?;
        self.cursor = end;
        Ok(value)
    }

    /// Reads one byte.
    fn byte(&mut self) -> Result<u8, CheckpointStoreError>
    {
        self.take(1)?
            .first()
            .copied()
            .ok_or(CheckpointStoreError::Corrupt)
    }

    /// Reads a canonical Boolean byte.
    fn boolean(&mut self) -> Result<bool, CheckpointStoreError>
    {
        match self.byte()? {
            | 0 => Ok(false),
            | 1 => Ok(true),
            | _ => Err(CheckpointStoreError::Corrupt),
        }
    }

    /// Reads one little-endian `u32`.
    fn u32(&mut self) -> Result<u32, CheckpointStoreError>
    {
        let bytes: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_error| CheckpointStoreError::Corrupt)?;
        Ok(u32::from_le_bytes(bytes))
    }

    /// Reads one little-endian `u64`.
    fn u64(&mut self) -> Result<u64, CheckpointStoreError>
    {
        let bytes: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_error| CheckpointStoreError::Corrupt)?;
        Ok(u64::from_le_bytes(bytes))
    }

    /// Reads one little-endian `i32`.
    fn i32(&mut self) -> Result<i32, CheckpointStoreError>
    {
        let bytes: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_error| CheckpointStoreError::Corrupt)?;
        Ok(i32::from_le_bytes(bytes))
    }

    /// Reads one little-endian `i64`.
    fn i64(&mut self) -> Result<i64, CheckpointStoreError>
    {
        let bytes: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_error| CheckpointStoreError::Corrupt)?;
        Ok(i64::from_le_bytes(bytes))
    }

    /// Reads and widens a checked sequence length.
    fn len(&mut self) -> Result<usize, CheckpointStoreError>
    {
        usize::try_from(self.u32()?).map_err(|_error| CheckpointStoreError::Corrupt)
    }

    /// Reads one length-prefixed UTF-8 string.
    fn string(&mut self) -> Result<String, CheckpointStoreError>
    {
        let width = self.len()?;
        let value = core::str::from_utf8(self.take(width)?)
            .map_err(|_error| CheckpointStoreError::Corrupt)?;
        Ok(value.to_owned())
    }

    /// Reads one optional length-prefixed UTF-8 string.
    fn option_string(&mut self) -> Result<Option<String>, CheckpointStoreError>
    {
        if self.boolean()? {
            Ok(Some(self.string()?))
        }
        else {
            Ok(None)
        }
    }

    /// Reads one canonical side tag.
    fn side(&mut self) -> Result<Side, CheckpointStoreError>
    {
        match self.byte()? {
            | 0 => Ok(Side::Fst),
            | 1 => Ok(Side::Snd),
            | _ => Err(CheckpointStoreError::Corrupt),
        }
    }

    /// Reads one canonical finite-or-omega grade.
    fn grade(&mut self) -> Result<Grade, CheckpointStoreError>
    {
        match self.byte()? {
            | 0 => Ok(Grade::fin(GradeBound::from(self.u64()?))),
            | 1 => Ok(Grade::OMEGA),
            | _ => Err(CheckpointStoreError::Corrupt),
        }
    }
    /// Reads one classifier sort.
    fn sort(&mut self) -> Result<SortExpr, CheckpointStoreError>
    {
        match self.byte()? {
            | 0 => Ok(SortExpr::value()),
            | 1 => Ok(SortExpr::computation()),
            | 2 => {
                let name = self.string()?;
                Ok(SortExpr::Param(SortParam::new(name.as_str())))
            },
            | _ => Err(CheckpointStoreError::Corrupt),
        }
    }
    /// Reads one canonical classifier.
    fn classifier(&mut self) -> Result<Classifier, CheckpointStoreError>
    {
        let sort = self.sort()?;
        let level = self.level()?;
        Ok(Classifier::new(sort, level))
    }

    /// Reads and reconstructs one canonical kernel-strata level.
    ///
    /// # Contract
    /// - requires: the reader cursor points to a canonical level payload.
    /// - ensures: returns the same canonical level represented by that payload.
    /// - provides: a level rebuilt through the public strata algebra.
    /// - fails: over-cap offsets return
    ///   [`CheckpointStoreError::LevelOffsetTooLarge`]; malformed bytes and
    ///   reconstruction overflow return [`CheckpointStoreError::Corrupt`].
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`CheckpointStoreError::LevelOffsetTooLarge`] when a variable
    /// atom's offset meets or exceeds [`MAX_DECODED_LEVEL_OFFSET`].
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the round-trip witness and the over-cap witness
    ///   distinguish the reconstructed level from refused malformed input.
    /// - witness: `persistence::tests::universe_sorts_and_levels_round_trip`
    /// - witness: `persistence::tests::oversized_level_offset_is_refused_with_exact_error`
    fn level(&mut self) -> Result<Level, CheckpointStoreError>
    {
        let mut level = Level::constant(LevelConstant::from(self.u64()?));
        let count = self.len()?;
        for _ in 0 .. count {
            let variable = LevelVar::from(LevelVarIndex::from(self.u32()?));
            let raw_offset = self.u64()?;
            if raw_offset >= MAX_DECODED_LEVEL_OFFSET {
                return Err(CheckpointStoreError::LevelOffsetTooLarge { offset: raw_offset });
            }
            let mut atom = Level::var(variable);
            let mut remaining = raw_offset;
            while remaining > 0_u64 {
                let successor = atom
                    .succ()
                    .map_err(|_error| CheckpointStoreError::Corrupt)?;
                atom = successor;
                let next_remaining = remaining
                    .checked_sub(1_u64)
                    .ok_or(CheckpointStoreError::Corrupt)?;
                remaining = next_remaining;
            }
            level = level.max(&atom);
        }
        Ok(level)
    }
}

/// Completed semantic nodes held by the iterative postfix decoder.
#[expect(
    clippy::large_enum_variant,
    reason = "The postfix decoder keeps complete nodes inline for iterative ownership."
)]
enum Node
{
    /// A complete checkpoint set.
    Checkpoints(Checkpoints),
    /// One item checkpoint.
    ItemCheckpoint(ItemCheckpoint),
    /// One dependency footprint.
    Footprint(Footprint),
    /// One item typing result.
    ItemTyping(ItemTyping),
    /// One checker error.
    TypeError(TypeError),
    /// One term.
    Term(Term),
    /// One type.
    Ty(Ty),
    /// One value.
    Value(Value),
    /// One value type.
    ValueType(ValueType),
    /// One computation type.
    CompType(CompType),
    /// One computation.
    Comp(Comp),
    /// One static argument.
    StaticArg(StaticArg),
    /// One static neutral.
    StaticNeutral(StaticNeutral),
    /// One static term.
    StaticTerm(StaticTerm),
}

/// Decodes one postfix token and reduces available children into a semantic
/// node.
fn decode_token(
    reader: &mut Reader<'_>,
    nodes: &mut Vec<Node>,
) -> Result<(), CheckpointStoreError>
{
    match reader.byte()? {
        | CHECKPOINTS => {
            let count = reader.len()?;
            let items = pop_many(nodes, count, pop_item_checkpoint)?;
            nodes.push(Node::Checkpoints(Checkpoints { items }));
        },
        | ITEM_CHECKPOINT => {
            let name = reader.option_string()?;
            let has_ascription = reader.boolean()?;
            let typing = pop_item_typing(nodes)?;
            let footprint = pop_footprint(nodes)?;
            let term = pop_term(nodes)?;
            let ascription = if has_ascription {
                Some(pop_ty(nodes)?)
            }
            else {
                None
            };
            nodes.push(Node::ItemCheckpoint(ItemCheckpoint {
                name,
                ascription,
                term,
                footprint,
                typing,
            }));
        },
        | FOOTPRINT => {
            let count = reader.len()?;
            let mut names = alloc::collections::BTreeSet::new();
            for _ in 0 .. count {
                let _inserted = names.insert(reader.string()?);
            }
            let opaque = reader.boolean()?;
            let has_hole = reader.boolean()?;
            nodes.push(Node::Footprint(Footprint {
                names,
                opaque,
                has_hole,
            }));
        },
        | TYPING_DEFINITION => {
            let name = reader.string()?;
            let bound = reader.boolean()?;
            let ty = pop_ty(nodes)?;
            nodes.push(Node::ItemTyping(ItemTyping::Definition { name, ty, bound }));
        },
        | TYPING_EXPRESSION => {
            let ty = pop_ty(nodes)?;
            nodes.push(Node::ItemTyping(ItemTyping::Expression { ty }));
        },
        | TYPING_ERROR => {
            let error = pop_type_error(nodes)?;
            nodes.push(Node::ItemTyping(ItemTyping::TypeError { error }));
        },
        | TYPING_HOLEY => nodes.push(Node::ItemTyping(ItemTyping::Holey)),
        | ERROR_TYPE_MISMATCH => {
            let expected = pop_ty(nodes)?;
            let actual = pop_ty(nodes)?;
            nodes.push(Node::TypeError(TypeError::type_mismatch(expected, actual)));
        },
        | ERROR_SHAPE_MISMATCH => {
            let expected = decode_error_text(reader.byte()?)?;
            let actual = pop_ty(nodes)?;
            nodes.push(Node::TypeError(TypeError::ShapeMismatch {
                expected,
                actual,
            }));
        },
        | ERROR_STUCK => {
            let hint = decode_error_text(reader.byte()?)?;
            let expr = pop_term(nodes)?;
            nodes.push(Node::TypeError(TypeError::StuckExpr { expr, hint }));
        },
        | ERROR_UNBOUND => {
            let name = reader.string()?;
            nodes.push(Node::TypeError(TypeError::UnboundVariable { name }));
        },
        | ERROR_GRADE => {
            let lower = reader.grade()?;
            let upper = reader.grade()?;
            nodes.push(Node::TypeError(TypeError::GradeError { lower, upper }));
        },
        | TERM_VALUE => {
            let value = pop_value(nodes)?;
            nodes.push(Node::Term(Term::Value(value)));
        },
        | TERM_COMP => {
            let comp = pop_comp(nodes)?;
            nodes.push(Node::Term(Term::Comp(comp)));
        },
        | TY_VALUE => {
            let value = pop_value_type(nodes)?;
            nodes.push(Node::Ty(Ty::Value(value)));
        },
        | TY_COMP => {
            let value = pop_comp_type(nodes)?;
            nodes.push(Node::Ty(Ty::Comp(value)));
        },
        | STATIC_ARG_LEVEL => {
            nodes.push(Node::StaticArg(StaticArg::Level(reader.level()?)));
        },
        | STATIC_ARG_SORT => {
            nodes.push(Node::StaticArg(StaticArg::Sort(reader.sort()?)));
        },
        | STATIC_ARG_TYPE => {
            let term = pop_static_term(nodes)?;
            nodes.push(Node::StaticArg(StaticArg::Type(Rc::new(term))));
        },
        | STATIC_ARG_VALUE => {
            let value = pop_value(nodes)?;
            nodes.push(Node::StaticArg(StaticArg::Value(Rc::new(value))));
        },
        | STATIC_NEUTRAL_HEAD => {
            nodes.push(Node::StaticNeutral(StaticNeutral::head(StaticVar::new(
                reader.string()?,
            ))));
        },
        | STATIC_NEUTRAL_APP => {
            let argument = pop_static_arg(nodes)?;
            let head = pop_static_neutral(nodes)?;
            nodes.push(Node::StaticNeutral(StaticNeutral::app(head, argument)));
        },
        | STATIC_TERM_VAR => {
            nodes.push(Node::StaticTerm(StaticTerm::Var(StaticVar::new(
                reader.string()?,
            ))));
        },
        | STATIC_TERM_UNIVERSE => {
            nodes.push(Node::StaticTerm(StaticTerm::Universe(reader.classifier()?)));
        },
        | STATIC_TERM_QUOTE => {
            let ty = pop_ty(nodes)?;
            nodes.push(Node::StaticTerm(StaticTerm::Quote(Rc::new(ty))));
        },
        | STATIC_TERM_PI => {
            let name = reader.string()?;
            let classifier = reader.classifier()?;
            let codomain = pop_static_term(nodes)?;
            nodes.push(Node::StaticTerm(StaticTerm::Pi {
                binder: StaticBinder::new(StaticVar::new(name), classifier),
                codomain: Rc::new(codomain),
            }));
        },
        | STATIC_TERM_LAM => {
            let name = reader.string()?;
            let classifier = reader.classifier()?;
            let body = pop_static_term(nodes)?;
            nodes.push(Node::StaticTerm(StaticTerm::Lam {
                binder: StaticBinder::new(StaticVar::new(name), classifier),
                body: Rc::new(body),
            }));
        },
        | STATIC_TERM_APP => {
            let argument = pop_static_arg(nodes)?;
            let function = pop_static_term(nodes)?;
            nodes.push(Node::StaticTerm(StaticTerm::App {
                function: Rc::new(function),
                argument,
            }));
        },
        | STATIC_TERM_NEUTRAL => {
            let neutral = pop_static_neutral(nodes)?;
            nodes.push(Node::StaticTerm(StaticTerm::Neutral(neutral)));
        },
        | VALUE_VAR => nodes.push(Node::Value(Value::Var(reader.string()?))),
        | VALUE_UNIT => nodes.push(Node::Value(Value::Unit)),
        | VALUE_INT => nodes.push(Node::Value(Value::Int(reader.i64()?))),
        | VALUE_STR => nodes.push(Node::Value(Value::Str(reader.string()?))),
        | VALUE_NUM => {
            let value = match reader.byte()? {
                | 0 => NumLit::U32(reader.u32()?),
                | 1 => NumLit::U64(reader.u64()?),
                | 2 => NumLit::I32(reader.i32()?),
                | 3 => NumLit::I64(reader.i64()?),
                | 4 => NumLit::F32(reader.u32()?),
                | 5 => NumLit::F64(reader.u64()?),
                | _ => return Err(CheckpointStoreError::Corrupt),
            };
            nodes.push(Node::Value(Value::Num(value)));
        },
        | VALUE_PAIR => {
            let snd = pop_value(nodes)?;
            let fst = pop_value(nodes)?;
            nodes.push(Node::Value(Value::Pair(Rc::new(fst), Rc::new(snd))));
        },
        | VALUE_INJ => {
            let side = reader.side()?;
            let payload = pop_value(nodes)?;
            nodes.push(Node::Value(Value::Inj(side, Rc::new(payload))));
        },
        | VALUE_LIST => {
            let count = reader.len()?;
            let elements = pop_many(nodes, count, pop_value)?
                .into_iter()
                .map(Rc::new)
                .collect();
            nodes.push(Node::Value(Value::List(elements)));
        },
        | VALUE_RECORD => {
            let count = reader.len()?;
            let labels = read_strings(reader, count)?;
            let values = pop_many(nodes, count, pop_value)?;
            let fields = labels
                .into_iter()
                .zip(values)
                .map(|(label, value)| (label, Rc::new(value)))
                .collect();
            nodes.push(Node::Value(Value::Record(fields)));
        },
        | VALUE_THUNK => {
            let grade = reader.grade()?;
            let body = pop_comp(nodes)?;
            nodes.push(Node::Value(Value::Thunk(grade, Rc::new(body))));
        },
        | VALUE_RUN => {
            let body = pop_comp(nodes)?;
            nodes.push(Node::Value(Value::Run(Rc::new(body))));
        },
        | VALUE_ANNOT => {
            let ty = pop_value_type(nodes)?;
            let value = pop_value(nodes)?;
            nodes.push(Node::Value(Value::Annot(Rc::new(value), Rc::new(ty))));
        },
        | VALUE_HOLE => nodes.push(Node::Value(Value::Hole(reader.u32()?))),
        | VALUE_HERE => {
            let value = pop_value(nodes)?;
            nodes.push(Node::Value(Value::Here(Rc::new(value))));
        },
        | VT_ATOM => nodes.push(Node::ValueType(ValueType::Atom(reader.string()?))),
        | VT_UNIT => nodes.push(Node::ValueType(ValueType::Unit)),
        | VT_PROD => binary_value_type(nodes, ValueType::prod)?,
        | VT_SUM => binary_value_type(nodes, ValueType::sum)?,
        | VT_LIST => {
            let element = pop_value_type(nodes)?;
            nodes.push(Node::ValueType(ValueType::list(element)));
        },
        | VT_RECORD => {
            let count = reader.len()?;
            let labels = read_strings(reader, count)?;
            let values = pop_many(nodes, count, pop_value_type)?;
            nodes.push(Node::ValueType(ValueType::record(
                labels.into_iter().zip(values),
            )));
        },
        | VT_THUNK => {
            let grade = reader.grade()?;
            let body = pop_comp_type(nodes)?;
            nodes.push(Node::ValueType(ValueType::thunk(grade, body)));
        },
        | VT_STK => {
            let delivers = pop_comp_type(nodes)?;
            let consumes = pop_comp_type(nodes)?;
            nodes.push(Node::ValueType(ValueType::stk(consumes, delivers)));
        },
        | VT_PATH => {
            let rhs = pop_value(nodes)?;
            let lhs = pop_value(nodes)?;
            let ty = pop_value_type(nodes)?;
            nodes.push(Node::ValueType(ValueType::path(ty, lhs, rhs)));
        },
        | VT_UNIVERSE => {
            let sort = reader.sort()?;
            let level = reader.level()?;
            nodes.push(Node::ValueType(ValueType::universe(sort, level)));
        },
        | VT_FAMILY => {
            let result = reader.classifier()?;
            let neutral = pop_static_neutral(nodes)?;
            nodes.push(Node::ValueType(ValueType::family(FamilyApp::new(
                neutral, result,
            ))));
        },
        | VT_SIGMA => {
            let binder = reader.string()?;
            let snd = pop_value_type(nodes)?;
            let fst = pop_value_type(nodes)?;
            nodes.push(Node::ValueType(ValueType::sigma(fst, binder.as_str(), snd)));
        },
        | VT_UNKNOWN => nodes.push(Node::ValueType(ValueType::Unknown)),
        | CT_F => {
            let of = pop_value_type(nodes)?;
            nodes.push(Node::CompType(CompType::F(Rc::new(of), EffectRow::EMPTY)));
        },
        | CT_ARROW => {
            let result = pop_comp_type(nodes)?;
            let arg = pop_value_type(nodes)?;
            nodes.push(Node::CompType(CompType::arrow(arg, result)));
        },
        | CT_PI => {
            let binder = reader.string()?;
            let result = pop_comp_type(nodes)?;
            let arg = pop_value_type(nodes)?;
            nodes.push(Node::CompType(CompType::pi(binder, arg, result)));
        },
        | CT_WITH => {
            let snd = pop_comp_type(nodes)?;
            let fst = pop_comp_type(nodes)?;
            nodes.push(Node::CompType(CompType::with(fst, snd)));
        },
        | CT_FAMILY => {
            let result = reader.classifier()?;
            let neutral = pop_static_neutral(nodes)?;
            nodes.push(Node::CompType(CompType::family(FamilyApp::new(
                neutral, result,
            ))));
        },
        | CT_UNKNOWN => nodes.push(Node::CompType(CompType::Unknown)),
        | COMP_ABS => {
            let binder = reader.string()?;
            let has_annotation = reader.boolean()?;
            let body = pop_comp(nodes)?;
            let annotation = if has_annotation {
                Some(Rc::new(pop_value_type(nodes)?))
            }
            else {
                None
            };
            nodes.push(Node::Comp(Comp::Abs(binder, annotation, Rc::new(body))));
        },
        | COMP_APP => {
            let arg = pop_value(nodes)?;
            let fun = pop_comp(nodes)?;
            nodes.push(Node::Comp(Comp::App(Rc::new(fun), Rc::new(arg))));
        },
        | COMP_RET => unary_value_comp(nodes, Comp::ret)?,
        | COMP_BIND => {
            let binder = reader.string()?;
            let body = pop_comp(nodes)?;
            let bound = pop_comp(nodes)?;
            nodes.push(Node::Comp(Comp::Bind(
                Rc::new(bound),
                binder,
                Rc::new(body),
            )));
        },
        | COMP_FORCE => unary_value_comp(nodes, Comp::force)?,
        | COMP_CASE => {
            let fst_name = reader.string()?;
            let snd_name = reader.string()?;
            let snd = pop_comp(nodes)?;
            let fst = pop_comp(nodes)?;
            let scrut = pop_value(nodes)?;
            nodes.push(Node::Comp(Comp::Case(
                Rc::new(scrut),
                (fst_name, Rc::new(fst)),
                (snd_name, Rc::new(snd)),
            )));
        },
        | COMP_DATA_CASE => {
            let count = reader.len()?;
            let binders = read_strings(reader, count)?;
            let arms = pop_many(nodes, count, pop_comp)?
                .into_iter()
                .map(Rc::new)
                .collect::<Vec<_>>();
            let scrut = pop_value(nodes)?;
            nodes.push(Node::Comp(Comp::DataCase(
                Rc::new(scrut),
                binders.into_iter().zip(arms).collect(),
            )));
        },
        | COMP_LIST_CASE => {
            let head = reader.string()?;
            let tail = reader.string()?;
            let cons = pop_comp(nodes)?;
            let nil = pop_comp(nodes)?;
            let scrut = pop_value(nodes)?;
            nodes.push(Node::Comp(Comp::ListCase {
                scrut: Rc::new(scrut),
                nil: Rc::new(nil),
                head,
                tail,
                cons: Rc::new(cons),
            }));
        },
        | COMP_SPLIT => {
            let fst_name = reader.string()?;
            let snd_name = reader.string()?;
            let has_motive = reader.boolean()?;
            let binder = if has_motive {
                Some(reader.string()?)
            }
            else {
                None
            };
            let body = pop_comp(nodes)?;
            let motive = if let Some(binder) = binder {
                let body = pop_comp_type(nodes)?;
                Some(Box::new(gandr_core_term::syntax::SplitMotive::new(
                    binder.as_str(),
                    body,
                )))
            }
            else {
                None
            };
            let scrut = pop_value(nodes)?;
            nodes.push(Node::Comp(Comp::Split {
                scrut: Rc::new(scrut),
                fst_name,
                snd_name,
                motive,
                body: Rc::new(body),
            }));
        },
        | COMP_RECORD_PROJ => {
            let label = reader.string()?;
            let record = pop_value(nodes)?;
            nodes.push(Node::Comp(Comp::RecordProj {
                record: Rc::new(record),
                label,
            }));
        },
        | COMP_WITH => {
            let snd = pop_comp(nodes)?;
            let fst = pop_comp(nodes)?;
            nodes.push(Node::Comp(Comp::With(Rc::new(fst), Rc::new(snd))));
        },
        | COMP_PRJ => {
            let side = reader.side()?;
            let comp = pop_comp(nodes)?;
            nodes.push(Node::Comp(Comp::Prj(side, Rc::new(comp))));
        },
        | COMP_DUP => unary_value_comp(nodes, Comp::dup)?,
        | COMP_DROP => unary_value_comp(nodes, Comp::drop)?,
        | COMP_RESUME => {
            let comp = pop_comp(nodes)?;
            let stack = pop_value(nodes)?;
            nodes.push(Node::Comp(Comp::Resume(Rc::new(stack), Rc::new(comp))));
        },
        | COMP_RESET => {
            let comp = pop_comp(nodes)?;
            nodes.push(Node::Comp(Comp::Reset(Rc::new(comp))));
        },
        | COMP_SHIFT => {
            let binder = reader.string()?;
            let body = pop_comp(nodes)?;
            nodes.push(Node::Comp(Comp::Shift(binder, Rc::new(body))));
        },
        | COMP_FIX => {
            let binder = reader.string()?;
            let body = pop_comp(nodes)?;
            nodes.push(Node::Comp(Comp::Fix(binder, Rc::new(body))));
        },
        | COMP_HOLE => nodes.push(Node::Comp(Comp::Hole(reader.u32()?))),
        | COMP_NATIVE => {
            let prim = decode_native(reader.byte()?)?;
            let count = reader.len()?;
            let args = pop_many(nodes, count, pop_value)?
                .into_iter()
                .map(Rc::new)
                .collect();
            nodes.push(Node::Comp(Comp::Native { prim, args }));
        },
        | COMP_WALK => {
            let x = reader.string()?;
            let y = reader.string()?;
            let q = reader.string()?;
            let base_x = reader.string()?;
            let base_body = pop_comp(nodes)?;
            let motive_body = pop_comp_type(nodes)?;
            let scrut = pop_value(nodes)?;
            nodes.push(Node::Comp(Comp::Walk {
                scrut: Rc::new(scrut),
                motive: Box::new(gandr_core_term::syntax::WalkMotive::new(
                    x.as_str(),
                    y.as_str(),
                    q.as_str(),
                    motive_body,
                )),
                base: gandr_core_term::syntax::WalkBase::new(base_x.as_str(), base_body),
            }));
        },
        | _ => return Err(CheckpointStoreError::Corrupt),
    }
    Ok(())
}

/// Reads a fixed count of length-prefixed UTF-8 strings.
fn read_strings(
    reader: &mut Reader<'_>,
    count: usize,
) -> Result<Vec<String>, CheckpointStoreError>
{
    let minimum_width = count
        .checked_mul(core::mem::size_of::<u32>())
        .ok_or(CheckpointStoreError::Corrupt)?;
    if minimum_width > reader.remaining() {
        return Err(CheckpointStoreError::Corrupt);
    }
    let mut values = Vec::with_capacity(count);
    for _ in 0 .. count {
        values.push(reader.string()?);
    }
    Ok(values)
}

/// Pops a fixed count of homogeneous nodes while preserving encoded order.
fn pop_many<T>(
    nodes: &mut Vec<Node>,
    count: usize,
    pop: fn(&mut Vec<Node>) -> Result<T, CheckpointStoreError>,
) -> Result<Vec<T>, CheckpointStoreError>
{
    if count > nodes.len() {
        return Err(CheckpointStoreError::Corrupt);
    }
    let mut values = Vec::with_capacity(count);
    for _ in 0 .. count {
        values.push(pop(nodes)?);
    }
    values.reverse();
    Ok(values)
}

/// Declares a typed node-stack popper with fail-closed variant checking.
macro_rules! pop_node {
    ($name:ident, $variant:ident, $ty:ty) => {
        #[doc = concat!("Pops one `", stringify!($variant), "` node.")]
        fn $name(nodes: &mut Vec<Node>) -> Result<$ty, CheckpointStoreError>
        {
            match nodes.pop() {
                | Some(Node::$variant(value)) => Ok(value),
                | _ => Err(CheckpointStoreError::Corrupt),
            }
        }
    };
}

pop_node!(pop_checkpoints, Checkpoints, Checkpoints);
pop_node!(pop_item_checkpoint, ItemCheckpoint, ItemCheckpoint);
pop_node!(pop_footprint, Footprint, Footprint);
pop_node!(pop_item_typing, ItemTyping, ItemTyping);
pop_node!(pop_type_error, TypeError, TypeError);
pop_node!(pop_term, Term, Term);
pop_node!(pop_ty, Ty, Ty);
pop_node!(pop_value, Value, Value);
pop_node!(pop_value_type, ValueType, ValueType);
pop_node!(pop_comp_type, CompType, CompType);
pop_node!(pop_comp, Comp, Comp);
pop_node!(pop_static_arg, StaticArg, StaticArg);
pop_node!(pop_static_neutral, StaticNeutral, StaticNeutral);
pop_node!(pop_static_term, StaticTerm, StaticTerm);

/// Reduces two value-type nodes through a binary constructor.
fn binary_value_type(
    nodes: &mut Vec<Node>,
    build: fn(ValueType, ValueType) -> ValueType,
) -> Result<(), CheckpointStoreError>
{
    let snd = pop_value_type(nodes)?;
    let fst = pop_value_type(nodes)?;
    nodes.push(Node::ValueType(build(fst, snd)));
    Ok(())
}

/// Reduces one value node through a computation constructor.
fn unary_value_comp(
    nodes: &mut Vec<Node>,
    build: fn(Value) -> Comp,
) -> Result<(), CheckpointStoreError>
{
    let value = pop_value(nodes)?;
    nodes.push(Node::Comp(build(value)));
    Ok(())
}

/// Returns the stable tag for a checker-owned static error string.
fn error_text_tag(value: &'static str) -> Result<u8, CheckpointStoreError>
{
    ERROR_TEXTS
        .iter()
        .position(|candidate| *candidate == value)
        .and_then(|index| u8::try_from(index).ok())
        .ok_or(CheckpointStoreError::Rejected)
}

/// Recovers a checker-owned static error string from its stable tag.
fn decode_error_text(tag: u8) -> Result<&'static str, CheckpointStoreError>
{
    ERROR_TEXTS
        .get(usize::from(tag))
        .copied()
        .ok_or(CheckpointStoreError::Corrupt)
}

/// Stable table of checker-owned diagnostic strings reachable through persisted
/// errors.
const ERROR_TEXTS: &[&str] = &[
    text::ANNOTATE_INJECTION,
    text::ANNOTATE_LIST,
    text::ANNOTATE_BINDER,
    text::ABS_NEEDS_ARROW,
    text::CASE_NEEDS_CHECK,
    text::LIST_CASE_NEEDS_CHECK,
    text::ANNOTATE_CTOR,
    text::DATA_CASE_NEEDS_CHECK,
    text::SPLIT_NEEDS_MOTIVE,
    text::WITH_NEEDS_WITH,
    text::DUP_NEEDS_RETURNER_PRODUCT,
    text::PERFORM_UNKNOWN_OP,
    text::HANDLE_NEEDS_CHECK,
    text::HANDLE_NEEDS_RETURNER,
    text::HANDLER_CLAUSES_MISMATCH,
    text::STK_NEEDS_STK_TYPE,
    text::RESET_NEEDS_CHECK,
    text::SHIFT_NEEDS_CHECK,
    text::SHIFT_NEEDS_RESET,
    text::SHAPE_ARROW,
    text::SHAPE_THUNK,
    text::SHAPE_RETURNER,
    text::SHAPE_SUM,
    text::SHAPE_DATA,
    text::SHAPE_LIST,
    text::SHAPE_PROD,
    text::SHAPE_RECORD,
    text::RECORD_NO_FIELD,
    text::SHAPE_WITH,
    text::SHAPE_STK,
    text::SHAPE_VALUE,
    text::SHAPE_COMP,
    text::SHAPE_PATH,
    text::CASE_ON_PATH_WITHOUT_K,
];

/// Returns the stable tag for a native primitive.
fn native_tag(value: NativePrim) -> u8
{
    match value {
        | NativePrim::Id => 0,
        | NativePrim::Const => 1,
        | NativePrim::Add => 2,
        | NativePrim::Sub => 3,
        | NativePrim::Mul => 4,
        | NativePrim::Eq => 5,
        | NativePrim::Ne => 6,
        | NativePrim::Lt => 7,
        | NativePrim::Le => 8,
        | NativePrim::Gt => 9,
        | NativePrim::Ge => 10,
        | NativePrim::And => 11,
        | NativePrim::Or => 12,
        | NativePrim::Neg => 13,
        | NativePrim::ListConcat => 14,
        | NativePrim::Each => 15,
        | NativePrim::Where => 16,
        | NativePrim::Reduce => 17,
        | NativePrim::Any => 18,
        | NativePrim::All => 19,
        | NativePrim::Flatten => 20,
        | NativePrim::Uniq => 21,
        | NativePrim::Sort => 22,
        | NativePrim::Get => 23,
        | NativePrim::Insert => 24,
        | NativePrim::RecordUpdate => 25,
        | NativePrim::Set => 26,
        | NativePrim::UpdateAt => 27,
        | NativePrim::InsertAt => 28,
        | NativePrim::RemoveAt => 29,
        | NativePrim::Push => 30,
        | NativePrim::UpdateWhere => 31,
        | NativePrim::StringEscape => 32,
        | NativePrim::StringContains => 33,
        | NativePrim::StringStartsWith => 34,
        | NativePrim::StringEndsWith => 35,
        | NativePrim::StringEq => 36,
        | NativePrim::StringSplit => 37,
        | NativePrim::RegexExtract => 38,
        | NativePrim::PathJoin => 39,
        | NativePrim::PathBasename => 40,
        | NativePrim::PathExtension => 41,
        | NativePrim::Div => 42,
        | NativePrim::Mod => 43,
        | NativePrim::Not => 44,
        | NativePrim::ListLength => 45,
        | NativePrim::ListAt => 46,
        | NativePrim::StringAppend => 47,
        | NativePrim::StringLength => 48,
    }
}

/// Recovers a native primitive from its stable tag.
fn decode_native(tag: u8) -> Result<NativePrim, CheckpointStoreError>
{
    match tag {
        | 0 => Ok(NativePrim::Id),
        | 1 => Ok(NativePrim::Const),
        | 2 => Ok(NativePrim::Add),
        | 3 => Ok(NativePrim::Sub),
        | 4 => Ok(NativePrim::Mul),
        | 5 => Ok(NativePrim::Eq),
        | 6 => Ok(NativePrim::Ne),
        | 7 => Ok(NativePrim::Lt),
        | 8 => Ok(NativePrim::Le),
        | 9 => Ok(NativePrim::Gt),
        | 10 => Ok(NativePrim::Ge),
        | 11 => Ok(NativePrim::And),
        | 12 => Ok(NativePrim::Or),
        | 13 => Ok(NativePrim::Neg),
        | 14 => Ok(NativePrim::ListConcat),
        | 15 => Ok(NativePrim::Each),
        | 16 => Ok(NativePrim::Where),
        | 17 => Ok(NativePrim::Reduce),
        | 18 => Ok(NativePrim::Any),
        | 19 => Ok(NativePrim::All),
        | 20 => Ok(NativePrim::Flatten),
        | 21 => Ok(NativePrim::Uniq),
        | 22 => Ok(NativePrim::Sort),
        | 23 => Ok(NativePrim::Get),
        | 24 => Ok(NativePrim::Insert),
        | 25 => Ok(NativePrim::RecordUpdate),
        | 26 => Ok(NativePrim::Set),
        | 27 => Ok(NativePrim::UpdateAt),
        | 28 => Ok(NativePrim::InsertAt),
        | 29 => Ok(NativePrim::RemoveAt),
        | 30 => Ok(NativePrim::Push),
        | 31 => Ok(NativePrim::UpdateWhere),
        | 32 => Ok(NativePrim::StringEscape),
        | 33 => Ok(NativePrim::StringContains),
        | 34 => Ok(NativePrim::StringStartsWith),
        | 35 => Ok(NativePrim::StringEndsWith),
        | 36 => Ok(NativePrim::StringEq),
        | 37 => Ok(NativePrim::StringSplit),
        | 38 => Ok(NativePrim::RegexExtract),
        | 39 => Ok(NativePrim::PathJoin),
        | 40 => Ok(NativePrim::PathBasename),
        | 41 => Ok(NativePrim::PathExtension),
        | 42 => Ok(NativePrim::Div),
        | 43 => Ok(NativePrim::Mod),
        | 44 => Ok(NativePrim::Not),
        | 45 => Ok(NativePrim::ListLength),
        | 46 => Ok(NativePrim::ListAt),
        | 47 => Ok(NativePrim::StringAppend),
        | 48 => Ok(NativePrim::StringLength),
        | _ => Err(CheckpointStoreError::Corrupt),
    }
}
