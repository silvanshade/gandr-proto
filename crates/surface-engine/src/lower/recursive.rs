//! Explicit-stack driver for caller-controlled lowering recursion.
//!
//! The migration keeps the syntax-directed lowering rules in one request
//! machine while replacing native mutual recursion with
//! `gandr-theory-recursion`'s heap-backed continuation stack. During the
//! conversion each request retains the existing rule body; the finished machine
//! schedules every descendant as a request before returning to its parent.

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::vec::IntoIter;
use alloc::vec::Vec;
use core::convert::Infallible;
use core::convert::identity;
use core::mem;

use gandr_core_term::boundary::NameRef;
use gandr_core_term::grade::Grade;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::SealId;
use gandr_core_term::types::Ty;
use gandr_core_term::types::ValueType;
use gandr_theory_recursion::Machine;
use gandr_theory_recursion::Step;
use gandr_theory_recursion::run;

use super::COut;
use super::EOut;
use super::Hoist;
use super::LowerError;
use super::LowerResult;
use super::Lowerer;
use super::VOut;
use super::data::DataConstructorCall;
use super::data::DataConstructorPlan;
use super::entry;
use super::node_kinds;
use super::sole_inner_expression;
use crate::boundary::NodeText;
use crate::boundary::SourceRange;
use crate::origin::ElabKind;
use crate::origin::OriginEntry;
use crate::origin::OriginFacet;
use crate::origin::OriginFacetKind;
use crate::origin::OriginNode;
use crate::synnode::SynNode;
/// Stable handle for one pending-hoist buffer owned by the lowering machine.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct HoistBufferId(usize);

/// Position of the next declared-data payload field.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct DataFieldIndex(usize);

impl DataFieldIndex
{
    /// The first payload position.
    const FIRST: Self = Self(0);

    /// Returns the proper successor position when representable.
    fn successor(self) -> Option<Self>
    {
        self.0.checked_add(1).map(Self)
    }
}

/// One lowering operation scheduled on the explicit recursion stack.
enum Request<'tree>
{
    /// Lower an expression without imposing a value/computation sort.
    Expr
    {
        /// Expression CST node.
        node: SynNode<'tree>,
        /// Buffer receiving computation-to-value hoists.
        hoists: HoistBufferId,
    },
    /// Lower an expression in value position.
    ValueExpr
    {
        /// Expression CST node.
        node: SynNode<'tree>,
        /// Buffer receiving computation-to-value hoists.
        hoists: HoistBufferId,
    },
    /// Lower an expression in computation position.
    CompExpr
    {
        /// Expression CST node.
        node: SynNode<'tree>,
    },
    /// Lower an ascription.
    Annotation
    {
        /// Annotation CST node.
        node: SynNode<'tree>,
        /// Buffer receiving computation-to-value hoists.
        hoists: HoistBufferId,
    },
    /// Lower a value under a known expected type.
    ValueExprExpecting
    {
        /// Expression CST node.
        node: SynNode<'tree>,
        /// Expected semantic value type.
        expected: ValueType,
        /// Buffer receiving computation-to-value hoists.
        hoists: HoistBufferId,
    },
    /// Lower the fields of one prepared data-constructor application.
    DataConstructor
    {
        /// Whole constructor-call node.
        call_node: SynNode<'tree>,
        /// Constructor plan resolved before descendant lowering.
        plan: DataConstructorPlan,
        /// Payload arguments retained once for the request chain.
        arguments: Rc<[SynNode<'tree>]>,
        /// Next payload argument.
        next: DataFieldIndex,
        /// Lowered payload components.
        components: Vec<Value>,
        /// Payload origin children.
        origins: Vec<OriginNode>,
        /// Buffer receiving field hoists.
        hoists: HoistBufferId,
    },
    /// Lower a tuple's value-position components.
    Tuple
    {
        /// Tuple CST node.
        node: SynNode<'tree>,
        /// Buffer receiving component hoists.
        hoists: HoistBufferId,
    },
    /// Lower a package introduction's witnesses and payload.
    Pack
    {
        /// Pack CST node.
        node: SynNode<'tree>,
        /// Buffer receiving payload hoists.
        hoists: HoistBufferId,
    },
    /// Lower a list literal's value-position elements.
    ListExpr
    {
        /// List CST node.
        node: SynNode<'tree>,
        /// Buffer receiving element hoists.
        hoists: HoistBufferId,
    },
    /// Lower a `force` form.
    Force
    {
        /// Force CST node.
        node: SynNode<'tree>,
    },
    /// Lower a `ret` form.
    Ret
    {
        /// Return CST node.
        node: SynNode<'tree>,
    },
    /// Lower unary negation.
    Unary
    {
        /// Unary-expression CST node.
        node: SynNode<'tree>,
    },
    /// Lower a thunk literal.
    Thunk
    {
        /// Thunk CST node.
        node: SynNode<'tree>,
    },
    /// Lower a lambda literal.
    Lambda
    {
        /// Lambda CST node.
        node: SynNode<'tree>,
    },
    /// Lower a computation block.
    Block
    {
        /// Block CST node.
        node: SynNode<'tree>,
    },
    /// Lower one block's statement/tail chain.
    Chain
    {
        /// Remaining statements.
        statements: StatementCursor<'tree>,
        /// Optional tail expression.
        tail: Option<SynNode<'tree>>,
        /// Enclosing block node.
        block_node: SynNode<'tree>,
    },
    /// Lower one statement together with its continuation.
    Statement
    {
        /// Current statement.
        first: SynNode<'tree>,
        /// Remaining statements.
        rest: StatementCursor<'tree>,
        /// Optional tail expression.
        tail: Option<SynNode<'tree>>,
        /// Enclosing block node.
        block_node: SynNode<'tree>,
        /// Origin covering the statement and continuation.
        span: OriginEntry,
    },
    /// Lower one value-binding statement.
    LetStatement
    {
        /// Current `let` statement.
        node: SynNode<'tree>,
        /// Remaining statements.
        rest: StatementCursor<'tree>,
        /// Optional tail expression.
        tail: Option<SynNode<'tree>>,
        /// Enclosing block node.
        block_node: SynNode<'tree>,
        /// Origin covering the statement and continuation.
        span: OriginEntry,
    },
}

/// One completed lowering request.
enum Lowered
{
    /// Sort-neutral expression output.
    Expr(EOut),
    /// Value-position output.
    Value(VOut),
    /// Computation-position output.
    Comp(COut),
}

/// Final constructor used after a sequence of value-position children lowers.
#[derive(Clone, Copy)]
enum ValueCollection
{
    /// Right-nested tuple.
    Tuple,
    /// Flat list literal.
    List,
}

/// Computation former assembled after one value-position child returns.
#[derive(Clone, Copy)]
enum UnaryCompForm
{
    /// Core `Force`.
    Force,
    /// Core `Ret`.
    Ret,
    /// Surface negation elaboration.
    Negate,
}

/// One suspended continuation step: consumes the machine and the completed
/// child value, and yields the next machine step.
type FrameStep<'run, 'src, 'tree> = dyn FnOnce(
        &mut LowerMachine<'run, 'src, 'tree>,
        LowerResult<Lowered>,
    ) -> Step<Request<'tree>, Frame<'run, 'src, 'tree>, LowerResult<Lowered>>
    + 'run;

#[repr(transparent)]
/// One suspended parent continuation.
struct Frame<'run, 'src, 'tree>(Box<FrameStep<'run, 'src, 'tree>>);

/// Source-ordered statement sequence shared by suspended continuations.
#[derive(Clone)]
struct StatementCursor<'tree>
{
    /// Statement nodes retained once for the whole block.
    nodes: Rc<[SynNode<'tree>]>,
    /// Next node to consume.
    next: usize,
}

impl<'tree> StatementCursor<'tree>
{
    /// Starts at the first node in `statements`.
    fn new(statements: Vec<SynNode<'tree>>) -> Self
    {
        Self {
            nodes: Rc::from(statements),
            next: 0,
        }
    }

    /// Returns the next node and the proper successor cursor.
    fn pop(self) -> (Option<SynNode<'tree>>, Self)
    {
        let node = self.nodes.get(self.next).copied();
        let next = node
            .and_then(|_| self.next.checked_add(1))
            .unwrap_or(self.next);
        (node, Self {
            nodes: self.nodes,
            next,
        })
    }

    /// Returns the end of the remaining source chain, excluding its enclosing
    /// block delimiter.
    fn end_byte(
        &self,
        tail: Option<SynNode<'tree>>,
    ) -> Option<usize>
    {
        tail.or_else(|| self.nodes.last().copied())
            .map(|node| node.end_byte().0)
    }
}

/// A single transition of the lowering request machine.
type LowerStep<'run, 'src, 'tree> =
    Step<Request<'tree>, Frame<'run, 'src, 'tree>, LowerResult<Lowered>>;

/// Mutable state shared by every request in one lowering run.
struct LowerMachine<'run, 'src, 'tree>
{
    /// Existing lowering registries, allocators, and source view.
    lowerer: &'run mut Lowerer<'src>,
    /// Hoist buffers retained across suspended parent requests.
    hoists: Vec<Option<Vec<Hoist>>>,
    /// Associates the machine with the CST view lifetime it traverses.
    tree: core::marker::PhantomData<SynNode<'tree>>,
}

impl<'run, 'src, 'tree: 'run> LowerMachine<'run, 'src, 'tree>
{
    /// Starts a machine with one caller-owned hoist buffer moved into its
    /// stable buffer table.
    fn new(
        lowerer: &'run mut Lowerer<'src>,
        hoists: Vec<Hoist>,
    ) -> Self
    {
        Self {
            lowerer,
            hoists: alloc::vec![Some(hoists)],
            tree: core::marker::PhantomData,
        }
    }

    /// Allocates an empty pending-hoist buffer and returns its stable handle.
    fn allocate_hoists(&mut self) -> HoistBufferId
    {
        let id = HoistBufferId(self.hoists.len());
        self.hoists.push(Some(Vec::new()));
        id
    }

    /// Appends one hoist to a retained buffer.
    fn push_hoist(
        &mut self,
        id: HoistBufferId,
        node: SynNode<'tree>,
        hoist: Hoist,
    ) -> LowerResult<()>
    {
        self.hoists
            .get_mut(id.0)
            .and_then(Option::as_mut)
            .map(|hoists| hoists.push(hoist))
            .ok_or_else(|| malformed(node))
    }

    /// Suspends the current rule behind one child request.
    fn descend(
        request: Request<'tree>,
        resume: impl FnOnce(&mut Self, LowerResult<Lowered>) -> LowerStep<'run, 'src, 'tree> + 'run,
    ) -> LowerStep<'run, 'src, 'tree>
    {
        Step::Descend {
            request,
            frame: Frame(Box::new(resume)),
        }
    }

    /// Returns one completed request or structured failure.
    fn returned(result: LowerResult<Lowered>) -> LowerStep<'run, 'src, 'tree>
    {
        Step::Return(result)
    }

    /// Runs a closure against one hoist buffer without retaining simultaneous
    /// mutable borrows of the lowerer and the buffer table.
    fn with_hoists<T>(
        &mut self,
        id: HoistBufferId,
        node: SynNode<'tree>,
        lower: impl FnOnce(&mut Lowerer<'src>, &mut Vec<Hoist>) -> LowerResult<T>,
    ) -> LowerResult<T>
    {
        let Some(slot) = self.hoists.get_mut(id.0)
        else {
            return Err(malformed(node));
        };
        let Some(mut hoists) = slot.take()
        else {
            return Err(malformed(node));
        };
        let result = lower(self.lowerer, &mut hoists);
        let Some(slot_back) = self.hoists.get_mut(id.0)
        else {
            return Err(malformed(node));
        };
        *slot_back = Some(hoists);
        result
    }

    /// Returns one hoist buffer to the caller after the machine stops.
    fn take_hoists(
        &mut self,
        id: HoistBufferId,
        node: SynNode<'tree>,
    ) -> LowerResult<Vec<Hoist>>
    {
        self.hoists
            .get_mut(id.0)
            .and_then(Option::take)
            .ok_or_else(|| malformed(node))
    }

    /// Starts one sort-neutral expression request.
    fn begin_expr(
        &mut self,
        node: SynNode<'tree>,
        hoists: HoistBufferId,
    ) -> LowerStep<'run, 'src, 'tree>
    {
        match node.kind() {
            | node_kinds::PARENTHESIZED_EXPRESSION => match sole_inner_expression(node) {
                | Ok(inner) => Self::descend(
                    Request::Expr {
                        node: inner,
                        hoists,
                    },
                    move |_machine, output| {
                        Self::returned(expect_expr(output, node).map(Lowered::Expr))
                    },
                ),
                | Err(error) => Self::returned(Err(error)),
            },
            | node_kinds::TUPLE_EXPRESSION => {
                Self::descend(Request::Tuple { node, hoists }, move |_machine, output| {
                    Self::returned(
                        expect_value(output, node).map(|value| Lowered::Expr(EOut::Value(value))),
                    )
                })
            },
            | node_kinds::LIST_EXPRESSION => Self::descend(
                Request::ListExpr { node, hoists },
                move |_machine, output| {
                    Self::returned(
                        expect_value(output, node).map(|value| Lowered::Expr(EOut::Value(value))),
                    )
                },
            ),
            | node_kinds::PACK_EXPRESSION => {
                Self::descend(Request::Pack { node, hoists }, move |_machine, output| {
                    Self::returned(
                        expect_value(output, node).map(|value| Lowered::Expr(EOut::Value(value))),
                    )
                })
            },
            | node_kinds::FORCE_EXPRESSION => {
                Self::descend(Request::Force { node }, move |_machine, output| {
                    Self::returned(
                        expect_comp(output, node).map(|comp| Lowered::Expr(EOut::Comp(comp))),
                    )
                })
            },
            | node_kinds::RET_EXPRESSION => {
                Self::descend(Request::Ret { node }, move |_machine, output| {
                    Self::returned(
                        expect_comp(output, node).map(|comp| Lowered::Expr(EOut::Comp(comp))),
                    )
                })
            },
            | node_kinds::UNARY_EXPRESSION => {
                Self::descend(Request::Unary { node }, move |_machine, output| {
                    Self::returned(
                        expect_comp(output, node).map(|comp| Lowered::Expr(EOut::Comp(comp))),
                    )
                })
            },
            | node_kinds::THUNK_EXPRESSION => {
                Self::descend(Request::Thunk { node }, move |_machine, output| {
                    Self::returned(
                        expect_value(output, node).map(|value| Lowered::Expr(EOut::Value(value))),
                    )
                })
            },
            | node_kinds::LAMBDA_EXPRESSION => {
                Self::descend(Request::Lambda { node }, move |_machine, output| {
                    Self::returned(
                        expect_comp(output, node).map(|comp| Lowered::Expr(EOut::Comp(comp))),
                    )
                })
            },
            | node_kinds::BLOCK => {
                Self::descend(Request::Block { node }, move |_machine, output| {
                    Self::returned(
                        expect_comp(output, node).map(|comp| Lowered::Expr(EOut::Comp(comp))),
                    )
                })
            },
            | node_kinds::ANNOTATION_EXPRESSION => Self::descend(
                Request::Annotation { node, hoists },
                move |_machine, output| {
                    Self::returned(expect_expr(output, node).map(Lowered::Expr))
                },
            ),
            | node_kinds::CALL_EXPRESSION => {
                let Some(call) = self.lowerer.data_constructor_call(node, None)
                else {
                    return Self::returned(self.legacy_expr(node, hoists).map(Lowered::Expr));
                };
                let request = match self.prepare_data_request(call, hoists) {
                    | Ok(request) => request,
                    | Err(error) => return Self::returned(Err(error)),
                };
                Self::descend(request, move |_machine, output| {
                    Self::returned(
                        expect_value(output, node).map(|value| Lowered::Expr(EOut::Value(value))),
                    )
                })
            },
            | _ => Self::returned(self.legacy_expr(node, hoists).map(Lowered::Expr)),
        }
    }

    /// Prepares one data-constructor field request.
    fn prepare_data_request(
        &self,
        call: DataConstructorCall<'tree>,
        hoists: HoistBufferId,
    ) -> LowerResult<Request<'tree>>
    {
        let plan = self.lowerer.prepare_data_constructor(
            call.call_node,
            call.constructor,
            &call.arguments,
            call.expected_args.as_deref(),
        )?;
        let capacity = plan.field_types().len();
        Ok(Request::DataConstructor {
            call_node: call.call_node,
            plan,
            arguments: Rc::from(call.arguments),
            next: DataFieldIndex::FIRST,
            components: Vec::with_capacity(capacity),
            origins: Vec::with_capacity(capacity),
            hoists,
        })
    }

    /// Starts one source ascription request.
    fn begin_annotation(
        &self,
        node: SynNode<'tree>,
        hoists: HoistBufferId,
    ) -> LowerStep<'run, 'src, 'tree>
    {
        let ty_node = match super::required_field(node, node_kinds::FIELD_TYPE) {
            | Ok(ty_node) => ty_node,
            | Err(error) => return Self::returned(Err(error)),
        };
        let value_node = match super::required_field(node, node_kinds::FIELD_VALUE) {
            | Ok(value_node) => value_node,
            | Err(error) => return Self::returned(Err(error)),
        };
        let lowered_type = match self.lowerer.lower_type_node(ty_node) {
            | Ok(lowered_type) => lowered_type,
            | Err(error) => return Self::returned(Err(error)),
        };
        match lowered_type {
            | Ty::Value(ascription) => Self::descend(
                Request::ValueExprExpecting {
                    node: value_node,
                    expected: ascription.clone(),
                    hoists,
                },
                move |_machine, output| {
                    let result = expect_value(output, value_node).and_then(|inner| {
                        let readback = inner.readback_value()?;
                        VOut::from_legacy_value(
                            &Value::annot(readback, ascription),
                            OriginNode::new(super::entry(node, None), vec![inner.origin]),
                        )
                        .map(|value| Lowered::Expr(EOut::Value(value)))
                    });
                    Self::returned(result)
                },
            ),
            | Ty::Comp(ascription) => Self::descend(
                Request::CompExpr { node: value_node },
                move |_machine, output| {
                    let result = expect_comp(output, value_node).and_then(|body| {
                        let annotated = Value::annot(
                            Value::thunk(Grade::OMEGA, {
                                let readback_comp = body.readback_comp()?;
                                identity(readback_comp)
                            }),
                            ValueType::thunk(Grade::OMEGA, ascription),
                        );
                        COut::from_legacy_comp(
                            &Comp::Force(Rc::new(annotated)),
                            Lowerer::comp_ascription_origin(node, body.origin),
                        )
                        .map(|comp| Lowered::Expr(EOut::Comp(comp)))
                    });
                    Self::returned(result)
                },
            ),
        }
    }

    /// Starts value lowering under a known expected type.
    fn begin_value_expr_expecting(
        &self,
        node: SynNode<'tree>,
        expected: &ValueType,
        hoists: HoistBufferId,
    ) -> LowerStep<'run, 'src, 'tree>
    {
        let Some(call) = self.lowerer.data_constructor_call(node, Some(expected))
        else {
            return Self::descend(
                Request::ValueExpr { node, hoists },
                move |_machine, output| {
                    Self::returned(expect_value(output, node).map(Lowered::Value))
                },
            );
        };
        let request = match self.prepare_data_request(call, hoists) {
            | Ok(request) => request,
            | Err(error) => return Self::returned(Err(error)),
        };
        Self::descend(request, move |machine, output| {
            let result = match expect_value(output, node) {
                | Err(ref error) if bool::from(machine.lowerer.total()) => {
                    machine.lowerer.value_hole(node, error)
                },
                | other => other,
            };
            Self::returned(result.map(Lowered::Value))
        })
    }

    /// Lowers one constructor payload field, or assembles the completed value.
    ///
    /// # Termination
    /// - reason: field continuations are scheduled on the explicit driver;
    /// - measure: `next` advances by one source argument per continuation;
    /// - boundedness: constructor arity and the parsed argument list are
    ///   finite.
    fn begin_data_constructor(
        call_node: SynNode<'tree>,
        plan: DataConstructorPlan,
        arguments: Rc<[SynNode<'tree>]>,
        next: DataFieldIndex,
        mut components: Vec<Value>,
        mut origins: Vec<OriginNode>,
        hoists: HoistBufferId,
    ) -> LowerStep<'run, 'src, 'tree>
    {
        let Some(argument) = arguments.get(next.0).copied()
        else {
            return Self::returned(
                plan.finish(call_node, components, origins)
                    .map(Lowered::Value),
            );
        };
        let Some(field_type) = plan.field_types().get(next.0).cloned()
        else {
            return Self::returned(Err(malformed(call_node)));
        };
        let is_unknown = matches!(field_type, ValueType::Unknown);
        let request = if matches!(field_type, ValueType::Data { .. }) {
            Request::ValueExprExpecting {
                node: argument,
                expected: field_type.clone(),
                hoists,
            }
        }
        else {
            Request::ValueExpr {
                node: argument,
                hoists,
            }
        };
        Self::descend(request, move |_machine, output| {
            let lowered = match expect_value(output, argument) {
                | Ok(lowered) => lowered,
                | Err(error) => return Self::returned(Err(error)),
            };
            let value = match lowered.readback_value() {
                | Ok(value) => value,
                | Err(error) => return Self::returned(Err(error)),
            };
            origins.push(lowered.origin);
            components.push(if is_unknown {
                value
            }
            else {
                Value::annot(value, field_type)
            });
            let Some(next) = next.successor()
            else {
                return Self::returned(Err(malformed(call_node)));
            };
            Self::descend(
                Request::DataConstructor {
                    call_node,
                    plan,
                    arguments,
                    next,
                    components,
                    origins,
                    hoists,
                },
                move |_, data_output| Self::returned(data_output),
            )
        })
    }

    /// Starts one value-position request.
    fn begin_value_expr(
        &mut self,
        node: SynNode<'tree>,
        hoists: HoistBufferId,
    ) -> LowerStep<'run, 'src, 'tree>
    {
        if node.kind() == node_kinds::HOLE {
            return Self::returned(self.lowerer.user_value_hole(node).map(Lowered::Value));
        }
        Self::descend(Request::Expr { node, hoists }, move |machine, output| {
            let lowered = match expect_expr(output, node) {
                | Err(ref error) if bool::from(machine.lowerer.total()) => {
                    return Self::returned(
                        machine.lowerer.value_hole(node, error).map(Lowered::Value),
                    );
                },
                | Err(error) => return Self::returned(Err(error)),
                | Ok(lowered) => lowered,
            };
            let value = match lowered {
                | EOut::Value(value) => Ok(value),
                | EOut::Comp(bound) => {
                    let name = machine.lowerer.fresh_name();
                    let origin = OriginNode::leaf(entry(node, Some(ElabKind::BindHoist)));
                    machine
                        .push_hoist(hoists, node, Hoist {
                            name: name.clone(),
                            bound,
                        })
                        .and_then(|()| VOut::from_legacy_value(&Value::Var(name), origin))
                },
            };
            Self::returned(value.map(Lowered::Value))
        })
    }

    /// Starts one computation-position request.
    fn begin_comp_expr(
        &mut self,
        node: SynNode<'tree>,
    ) -> LowerStep<'run, 'src, 'tree>
    {
        if node.kind() == node_kinds::HOLE {
            return Self::returned(self.lowerer.user_comp_hole(node).map(Lowered::Comp));
        }
        let hoists = self.allocate_hoists();
        Self::descend(Request::Expr { node, hoists }, move |machine, output| {
            let lowered = match expect_expr(output, node) {
                | Err(ref error) if bool::from(machine.lowerer.total()) => {
                    let result = (|| {
                        let hole = machine.lowerer.comp_hole(node, error)?;
                        let hoists = machine.take_hoists(hoists, node)?;
                        Lowerer::wrap_hoists(hoists, hole, node)
                    })();

                    return Self::returned(result.map(Lowered::Comp));
                },
                | Err(error) => return Self::returned(Err(error)),
                | Ok(lowered) => lowered,
            };
            let body = match lowered {
                | EOut::Comp(comp) => Ok(comp),
                | EOut::Value(value) => value.readback_value().and_then(|readback| {
                    COut::from_legacy_comp(
                        &Comp::Ret(Rc::new(readback)),
                        OriginNode::new(entry(node, Some(ElabKind::RetCoercion)), alloc::vec![
                            value.origin
                        ]),
                    )
                }),
            };
            let result = (|| {
                let body = body?;
                let hoists = machine.take_hoists(hoists, node)?;
                Lowerer::wrap_hoists(hoists, body, node)
            })();
            Self::returned(result.map(Lowered::Comp))
        })
    }

    /// Starts a `force`, `ret`, or unary-negation request.
    fn begin_unary_comp(
        &mut self,
        form: UnaryCompForm,
        node: SynNode<'tree>,
    ) -> LowerStep<'run, 'src, 'tree>
    {
        let field = match form {
            | UnaryCompForm::Force | UnaryCompForm::Ret => node_kinds::FIELD_VALUE,
            | UnaryCompForm::Negate => node_kinds::FIELD_OPERAND,
        };
        let child = match super::required_field(node, field) {
            | Ok(child) => child,
            | Err(error) => return Self::returned(Err(error)),
        };
        let hoists = self.allocate_hoists();
        Self::descend(
            Request::ValueExpr {
                node: child,
                hoists,
            },
            move |machine, output| {
                let result = (|| {
                    let value = expect_value(output, child)?;
                    let body = finish_unary_comp(form, node, value)?;
                    let hoists = machine.take_hoists(hoists, node)?;
                    Lowerer::wrap_hoists(hoists, body, node)
                })();
                Self::returned(result.map(Lowered::Comp))
            },
        )
    }

    /// Starts one package-introduction request.
    ///
    /// The witnesses lower as types before the payload is descended, so a
    /// malformed witness is refused at the pack rather than after its payload
    /// has been walked. The payload is an ordinary value-position expression
    /// and shares the caller's hoist buffer, since a pack is a value and has
    /// nowhere of its own to bind a hoist.
    fn begin_pack(
        &self,
        node: SynNode<'tree>,
        hoists: HoistBufferId,
    ) -> LowerStep<'run, 'src, 'tree>
    {
        let mut witnesses = Vec::new();
        for witness in node.children_by_field_name(node_kinds::FIELD_COMPONENT) {
            match self.lowerer.lower_value_type_node(witness) {
                | Ok(witness) => witnesses.push(witness),
                | Err(error) => return Self::returned(Err(error)),
            }
        }
        let payload_node = match super::required_field(node, node_kinds::FIELD_ARGUMENT) {
            | Ok(payload_node) => payload_node,
            | Err(error) => return Self::returned(Err(error)),
        };
        Self::descend(
            Request::ValueExpr {
                node: payload_node,
                hoists,
            },
            move |_machine, output| {
                let result = expect_value(output, payload_node).and_then(|payload| {
                    let readback = payload.readback_value()?;
                    VOut::from_legacy_value(
                        &Value::pack(witnesses, readback),
                        OriginNode::new(entry(node, None), alloc::vec![payload.origin]),
                    )
                });
                Self::returned(result.map(Lowered::Value))
            },
        )
    }

    /// Starts one thunk-literal request.
    fn begin_thunk(
        &self,
        node: SynNode<'tree>,
    ) -> LowerStep<'run, 'src, 'tree>
    {
        let grade = match super::types::parse_grade(self.lowerer.source, node) {
            | Ok(grade) => grade,
            | Err(error) => return Self::returned(Err(error)),
        };
        let grade_range = node
            .child_by_field_name(node_kinds::FIELD_GRADE)
            .map(SynNode::byte_range);
        let body_node = match super::required_field(node, node_kinds::FIELD_BODY) {
            | Ok(body_node) => body_node,
            | Err(error) => return Self::returned(Err(error)),
        };
        Self::descend(
            Request::Block { node: body_node },
            move |_machine, output| {
                let result = expect_comp(output, body_node).and_then(|body| {
                    let readback = body.readback_comp()?;
                    let mut origin = OriginNode::new(entry(node, None), alloc::vec![body.origin]);
                    if let Some(byte_range) = grade_range {
                        origin = origin.with_facet(OriginFacet {
                            kind: OriginFacetKind::Grade,
                            byte_range,
                        });
                    }
                    VOut::from_legacy_value(&Value::thunk(grade, readback), origin)
                });
                Self::returned(result.map(Lowered::Value))
            },
        )
    }

    /// Starts one lambda-literal request.
    fn begin_lambda(
        &mut self,
        node: SynNode<'tree>,
    ) -> LowerStep<'run, 'src, 'tree>
    {
        let params_node = match super::required_field(node, node_kinds::FIELD_PARAMETERS) {
            | Ok(params_node) => params_node,
            | Err(error) => return Self::returned(Err(error)),
        };
        let params = match self.lowerer.parameters(params_node) {
            | Ok(params) if params.is_empty() => {
                return Self::returned(Err(LowerError::Unsupported {
                    kind: node.kind(),
                    byte_range: node.byte_range(),
                }));
            },
            | Ok(params) => params,
            | Err(error) => return Self::returned(Err(error)),
        };
        let body_node = match super::required_field(node, node_kinds::FIELD_BODY) {
            | Ok(body_node) => body_node,
            | Err(error) => return Self::returned(Err(error)),
        };
        Self::descend(
            Request::Block { node: body_node },
            move |_machine, output| {
                let result = expect_comp(output, body_node)
                    .and_then(|body| Lowerer::curry_abs(params, body, &entry(node, None), None));
                Self::returned(result.map(Lowered::Comp))
            },
        )
    }

    /// Executes the block rule while its statement machine is being migrated.
    fn begin_block(
        &self,
        node: SynNode<'tree>,
    ) -> LowerStep<'run, 'src, 'tree>
    {
        let (statements, tail) = match split_block(self.lowerer, node) {
            | Ok(parts) => parts,
            | Err(error) => return Self::returned(Err(error)),
        };
        Self::descend(
            Request::Chain {
                statements: StatementCursor::new(statements),
                tail,
                block_node: node,
            },
            move |_machine, output| Self::returned(expect_comp(output, node).map(Lowered::Comp)),
        )
    }

    /// Starts or resumes one statement-chain request.
    ///
    /// # Termination
    /// - reason: re-entry occurs only after the explicit driver resumes a saved
    ///   continuation, never by a native recursive call;
    /// - measure: each `Statement` request advances the shared cursor by one;
    /// - boundedness: a parsed block contains finitely many statements.
    fn begin_chain(
        &mut self,
        statements: StatementCursor<'tree>,
        tail: Option<SynNode<'tree>>,
        block_node: SynNode<'tree>,
    ) -> LowerStep<'run, 'src, 'tree>
    {
        let chain_end = statements.end_byte(tail);
        let (first, rest) = statements.pop();
        let Some(first) = first
        else {
            return match tail {
                | Some(tail_node) => Self::descend(
                    Request::CompExpr { node: tail_node },
                    move |_machine, output| {
                        Self::returned(expect_comp(output, tail_node).map(Lowered::Comp))
                    },
                ),
                | None => {
                    let error = LowerError::EmptyBlock {
                        byte_range: block_node.byte_range(),
                    };
                    if bool::from(self.lowerer.total()) {
                        Self::returned(
                            self.lowerer
                                .comp_hole(block_node, &error)
                                .map(Lowered::Comp),
                        )
                    }
                    else {
                        Self::returned(Err(error))
                    }
                },
            };
        };
        let span = OriginEntry {
            cst_node: first.cst_node(),
            cst_hash: first.cst_hash(),
            byte_range: SourceRange(
                first.start_byte().0 .. chain_end.unwrap_or(first.end_byte().0),
            ),
            elaboration: None,
            note: None,
        };
        let retry_rest = rest.clone();
        let saved_hoist = self.lowerer.hoist.clone();
        let saved_holes = self.lowerer.holes.clone();
        Self::descend(
            Request::Statement {
                first,
                rest,
                tail,
                block_node,
                span: span.clone(),
            },
            move |machine, output| match expect_comp(output, first) {
                | Ok(comp) => Self::returned(Ok(Lowered::Comp(comp))),
                | Err(error) => {
                    machine.lowerer.hoist = saved_hoist;
                    machine.lowerer.holes = saved_holes;
                    if !bool::from(machine.lowerer.total()) {
                        return Self::returned(Err(error));
                    }
                    let bound = match machine.lowerer.comp_hole(first, &error) {
                        | Ok(bound) => bound,
                        | Err(error) => return Self::returned(Err(error)),
                    };
                    Self::descend(
                        Request::Chain {
                            statements: retry_rest,
                            tail,
                            block_node,
                        },
                        move |_machine, chain_output| {
                            let result =
                                expect_comp(chain_output, block_node).and_then(|rest_comp| {
                                    bind_outputs(
                                        bound,
                                        node_kinds::DISCARD_BINDER.to_owned(),
                                        rest_comp,
                                        super::with_elab(&span, Some(ElabKind::SeqDiscard)),
                                    )
                                });
                            Self::returned(result.map(Lowered::Comp))
                        },
                    )
                },
            },
        )
    }

    /// Starts one statement request.
    ///
    /// # Termination
    /// - reason: continuation requests are returned to the explicit driver;
    /// - measure: every continuation carries the proper successor cursor;
    /// - boundedness: the enclosing block is finite.
    fn begin_statement(
        &mut self,
        first: SynNode<'tree>,
        rest: StatementCursor<'tree>,
        tail: Option<SynNode<'tree>>,
        block_node: SynNode<'tree>,
        span: OriginEntry,
    ) -> LowerStep<'run, 'src, 'tree>
    {
        if first.is_error().0 || first.is_missing().0 {
            return Self::returned(Err(LowerError::Syntax {
                byte_range: first.byte_range(),
            }));
        }
        match first.kind() {
            | node_kinds::EXPRESSION_STATEMENT => {
                let inner = match sole_inner_expression(first) {
                    | Ok(inner) => inner,
                    | Err(error) => return Self::returned(Err(error)),
                };
                Self::descend(
                    Request::CompExpr { node: inner },
                    move |_machine, output| match expect_comp(output, inner) {
                        | Ok(bound) => Self::continue_bound_statement(
                            bound,
                            node_kinds::DISCARD_BINDER.to_owned(),
                            rest,
                            tail,
                            block_node,
                            super::with_elab(&span, Some(ElabKind::SeqDiscard)),
                        ),
                        | Err(error) => Self::returned(Err(error)),
                    },
                )
            },
            | node_kinds::BIND_STATEMENT => {
                let pattern = match super::required_field(first, node_kinds::FIELD_PATTERN) {
                    | Ok(pattern) => pattern,
                    | Err(error) => return Self::returned(Err(error)),
                };
                let binder = match self.lowerer.pattern_binder(pattern) {
                    | Ok(binder) => binder,
                    | Err(error) => return Self::returned(Err(error)),
                };
                let source = match super::required_field(first, node_kinds::FIELD_SOURCE) {
                    | Ok(source) => source,
                    | Err(error) => return Self::returned(Err(error)),
                };
                // `run p : B <- t ;` means `run p <- (t : B) ;`: the optional
                // annotation names the bound computation's type and is spent
                // through the existing computation ascription, so the bind
                // itself lowers identically either way.
                let annotation = match self.lowerer.run_bind_annotation(first) {
                    | Ok(annotation) => annotation,
                    | Err(error) => return Self::returned(Err(error)),
                };
                Self::descend(
                    Request::CompExpr { node: source },
                    move |_machine, output| {
                        let bound =
                            expect_comp(output, source).and_then(|bound| match annotation {
                                | Some(ascription) => {
                                    Lowerer::comp_ascribed_binding(source, bound, ascription)
                                },
                                | None => Ok(bound),
                            });
                        match bound {
                            | Ok(bound) => Self::continue_bound_statement(
                                bound, binder, rest, tail, block_node, span,
                            ),
                            | Err(error) => Self::returned(Err(error)),
                        }
                    },
                )
            },
            // `unpack m : Sig = E ;` binds the module variable over the REST
            // of its block, which is what makes the elimination check-only:
            // the block's answer arrives from outside, so an atom minted here
            // cannot reach it.
            | node_kinds::UNPACK_STATEMENT => {
                let binder = match super::required_field(first, node_kinds::FIELD_NAME)
                    .and_then(|name| super::node_text(self.lowerer.source, name))
                {
                    | Ok(binder) => binder.0.to_owned(),
                    | Err(error) => return Self::returned(Err(error)),
                };
                let signature_node = match super::required_field(first, node_kinds::FIELD_TYPE) {
                    | Ok(signature_node) => signature_node,
                    | Err(error) => return Self::returned(Err(error)),
                };
                let signature = match self.lowerer.lower_value_type_node(signature_node) {
                    | Ok(signature) => signature,
                    | Err(error) => return Self::returned(Err(error)),
                };
                if !matches!(signature, ValueType::Package { .. } | ValueType::Unknown) {
                    return Self::returned(Err(LowerError::UnpackNeedsPackageSignature {
                        byte_range: signature_node.byte_range(),
                    }));
                }
                let atoms = self
                    .lowerer
                    .mint_unpack_atoms(NameRef::from(binder.as_str()), &signature);
                let source_node = match super::required_field(first, node_kinds::FIELD_SOURCE) {
                    | Ok(source_node) => source_node,
                    | Err(error) => return Self::returned(Err(error)),
                };
                let hoists = self.allocate_hoists();
                Self::descend(
                    Request::ValueExpr {
                        node: source_node,
                        hoists,
                    },
                    move |machine, output| {
                        let parts = (|| {
                            let scrut = expect_value(output, source_node)?;
                            let hoists = machine.take_hoists(hoists, source_node)?;
                            Ok(UnpackParts {
                                scrut,
                                signature,
                                atoms,
                                binder,
                                hoists,
                            })
                        })();
                        match parts {
                            | Ok(parts) => {
                                Self::continue_unpack_statement(parts, rest, tail, block_node, span)
                            },
                            | Err(error) => Self::returned(Err(error)),
                        }
                    },
                )
            },
            | node_kinds::LET_STATEMENT => Self::descend(
                Request::LetStatement {
                    node: first,
                    rest,
                    tail,
                    block_node,
                    span,
                },
                move |_machine, output| {
                    Self::returned(expect_comp(output, first).map(Lowered::Comp))
                },
            ),
            | kind => Self::returned(Err(LowerError::Unsupported {
                kind,
                byte_range: first.byte_range(),
            })),
        }
    }

    /// Lowers an unpack continuation once its package value has returned.
    ///
    /// The rest of the block becomes the elimination's **body**, so the module
    /// variable scopes over exactly the statements that follow it — the
    /// declaration-granular shape the module design asks of a sealing binder.
    fn continue_unpack_statement(
        parts: UnpackParts,
        rest: StatementCursor<'tree>,
        tail: Option<SynNode<'tree>>,
        block_node: SynNode<'tree>,
        span: OriginEntry,
    ) -> LowerStep<'run, 'src, 'tree>
    {
        Self::descend(
            Request::Chain {
                statements: rest,
                tail,
                block_node,
            },
            move |_machine, output| {
                let result = (|| {
                    let body = expect_comp(output, block_node)?;
                    let unpacked = COut::from_legacy_comp(
                        &Comp::Unpack {
                            scrut: Rc::new({
                                let readback_value = parts.scrut.readback_value()?;
                                identity(readback_value)
                            }),
                            signature: Rc::new(parts.signature),
                            atoms: parts.atoms,
                            binder: parts.binder,
                            body: Rc::new({
                                let readback_comp = body.readback_comp()?;
                                identity(readback_comp)
                            }),
                        },
                        OriginNode::new(span, vec![parts.scrut.origin, body.origin]),
                    )?;
                    Lowerer::wrap_hoists(parts.hoists, unpacked, block_node)
                })();
                Self::returned(result.map(Lowered::Comp))
            },
        )
    }

    /// Lowers a statement continuation after its bound computation returns.
    fn continue_bound_statement(
        bound: COut,
        binder: String,
        rest: StatementCursor<'tree>,
        tail: Option<SynNode<'tree>>,
        block_node: SynNode<'tree>,
        span: OriginEntry,
    ) -> LowerStep<'run, 'src, 'tree>
    {
        Self::descend(
            Request::Chain {
                statements: rest,
                tail,
                block_node,
            },
            move |_machine, output| {
                let result = expect_comp(output, block_node)
                    .and_then(|rest_comp| bind_outputs(bound, binder, rest_comp, span));
                Self::returned(result.map(Lowered::Comp))
            },
        )
    }

    /// Starts one value-binding statement request.
    fn begin_let_statement(
        &mut self,
        node: SynNode<'tree>,
        rest: StatementCursor<'tree>,
        tail: Option<SynNode<'tree>>,
        block_node: SynNode<'tree>,
        span: OriginEntry,
    ) -> LowerStep<'run, 'src, 'tree>
    {
        let pattern = match super::required_field(node, node_kinds::FIELD_PATTERN) {
            | Ok(pattern) => pattern,
            | Err(error) => return Self::returned(Err(error)),
        };
        let value_node = match super::required_field(node, node_kinds::FIELD_VALUE) {
            | Ok(value_node) => value_node,
            | Err(error) => return Self::returned(Err(error)),
        };
        match pattern.kind() {
            | node_kinds::IDENTIFIER | node_kinds::WILDCARD => {
                let binder = match self.lowerer.pattern_binder(pattern) {
                    | Ok(binder) => binder,
                    | Err(error) => return Self::returned(Err(error)),
                };
                let hoists = self.allocate_hoists();
                Self::descend(
                    Request::Expr {
                        node: value_node,
                        hoists,
                    },
                    move |machine, output| {
                        let lowered = match expect_expr(output, value_node) {
                            | Err(ref error) if bool::from(machine.lowerer.total()) => {
                                match machine.lowerer.comp_hole(value_node, error) {
                                    | Ok(hole) => EOut::Comp(hole),
                                    | Err(error) => return Self::returned(Err(error)),
                                }
                            },
                            | Err(error) => return Self::returned(Err(error)),
                            | Ok(lowered) => lowered,
                        };
                        let bound = match let_bound(value_node, lowered) {
                            | Ok(bound) => bound,
                            | Err(error) => return Self::returned(Err(error)),
                        };
                        Self::descend(
                            Request::Chain {
                                statements: rest,
                                tail,
                                block_node,
                            },
                            move |inner_machine, chain_output| {
                                let result = (|| {
                                    let rest_comp = expect_comp(chain_output, block_node)?;
                                    let body = bind_outputs(
                                        bound,
                                        binder,
                                        rest_comp,
                                        super::with_elab(&span, Some(ElabKind::LetValueBind)),
                                    )?;
                                    let hoists = inner_machine.take_hoists(hoists, node)?;
                                    Lowerer::wrap_hoists_entry(hoists, body, &span)
                                })();
                                Self::returned(result.map(Lowered::Comp))
                            },
                        )
                    },
                )
            },
            | node_kinds::TUPLE_PATTERN => {
                let hoists = self.allocate_hoists();
                let elements = super::named_non_extra_children(pattern);
                Self::descend(
                    Request::ValueExpr {
                        node: value_node,
                        hoists,
                    },
                    move |_machine, output| match expect_value(output, value_node) {
                        | Ok(scrutinee) => Self::descend(
                            Request::Chain {
                                statements: rest,
                                tail,
                                block_node,
                            },
                            move |machine, chain_output| {
                                let result = (|| {
                                    let rest_comp = expect_comp(chain_output, block_node)?;
                                    let body = finish_split_chain(
                                        machine.lowerer,
                                        &elements,
                                        scrutinee,
                                        rest_comp,
                                        &span,
                                        None,
                                    )?;
                                    let hoists = machine.take_hoists(hoists, node)?;
                                    Lowerer::wrap_hoists_entry(hoists, body, &span)
                                })();
                                Self::returned(result.map(Lowered::Comp))
                            },
                        ),
                        | Err(error) => Self::returned(Err(error)),
                    },
                )
            },
            | kind => Self::returned(Err(LowerError::Unsupported {
                kind,
                byte_range: pattern.byte_range(),
            })),
        }
    }

    /// Starts tuple component lowering.
    fn begin_tuple(
        node: SynNode<'tree>,
        hoists: HoistBufferId,
    ) -> LowerStep<'run, 'src, 'tree>
    {
        Self::continue_values(
            ValueCollection::Tuple,
            node,
            hoists,
            super::named_non_extra_children(node).into_iter(),
            Vec::new(),
        )
    }

    /// Starts list element lowering.
    fn begin_list_expr(
        node: SynNode<'tree>,
        hoists: HoistBufferId,
    ) -> LowerStep<'run, 'src, 'tree>
    {
        Self::continue_values(
            ValueCollection::List,
            node,
            hoists,
            super::named_non_extra_children(node).into_iter(),
            Vec::new(),
        )
    }

    /// Lowers the next value-position child or assembles the completed
    /// collection.
    ///
    /// # Termination
    /// - reason: re-entry occurs only after `gandr-theory-recursion` resumes a
    ///   saved continuation, so native call depth stays constant;
    /// - measure: the remaining child iterator loses exactly one node;
    /// - boundedness: a CST node has finitely many children.
    fn continue_values(
        collection: ValueCollection,
        node: SynNode<'tree>,
        hoists: HoistBufferId,
        mut remaining: IntoIter<SynNode<'tree>>,
        mut lowered: Vec<VOut>,
    ) -> LowerStep<'run, 'src, 'tree>
    {
        let Some(child) = remaining.next()
        else {
            let result = match collection {
                | ValueCollection::Tuple => finish_tuple(node, lowered),
                | ValueCollection::List => finish_list(node, lowered),
            };
            return Self::returned(result.map(Lowered::Value));
        };
        Self::descend(
            Request::ValueExpr {
                node: child,
                hoists,
            },
            move |_machine, output| match expect_value(output, child) {
                | Ok(value) => {
                    lowered.push(value);
                    Self::continue_values(collection, node, hoists, remaining, lowered)
                },
                | Err(error) => Self::returned(Err(error)),
            },
        )
    }

    /// Executes the current expression rule. Descendant scheduling is moved
    /// into request/frame pairs as the corresponding rule enters the machine.
    fn legacy_expr(
        &mut self,
        node: SynNode<'tree>,
        hoists: HoistBufferId,
    ) -> LowerResult<EOut>
    {
        if node.is_error().0 || node.is_missing().0 {
            return Err(LowerError::Syntax {
                byte_range: node.byte_range(),
            });
        }
        match node.kind() {
            | node_kinds::IDENTIFIER => {
                let name = self.lowerer.text(node).map(NodeText::to_owned)?;
                VOut::from_legacy_value(&Value::Var(name), OriginNode::leaf(entry(node, None)))
                    .map(EOut::Value)
            },
            | node_kinds::NUMBER => self.lowerer.number_literal(node).map(EOut::Value),
            | node_kinds::TYPED_NUMBER => self.lowerer.typed_number_literal(node).map(EOut::Value),
            | node_kinds::STRING => self.lowerer.string_literal(node).map(EOut::Value),
            | node_kinds::BOOLEAN => Lowerer::boolean_literal(node).map(EOut::Value),
            | node_kinds::UNIT => {
                VOut::from_legacy_value(&Value::Unit, OriginNode::leaf(entry(node, None)))
                    .map(EOut::Value)
            },
            | node_kinds::HOLE => self.lowerer.user_value_hole(node).map(EOut::Value),
            | node_kinds::PARENTHESIZED_EXPRESSION => {
                let inner = sole_inner_expression(node)?;
                self.with_hoists(hoists, node, |lowerer, hoists| lowerer.expr(inner, hoists))
            },
            | node_kinds::TUPLE_EXPRESSION => self
                .with_hoists(hoists, node, |lowerer, hoists| lowerer.tuple(node, hoists))
                .map(EOut::Value),
            | node_kinds::LIST_EXPRESSION => self
                .with_hoists(hoists, node, |lowerer, hoists| {
                    lowerer.list_expr(node, hoists)
                })
                .map(EOut::Value),
            | node_kinds::ANNOTATION_EXPRESSION => {
                self.with_hoists(hoists, node, |lowerer, hoists| {
                    lowerer.annotation(node, hoists)
                })
            },
            | node_kinds::THUNK_EXPRESSION => self.lowerer.thunk(node).map(EOut::Value),
            | node_kinds::BLOCK => self.lowerer.block(node).map(EOut::Comp),
            | node_kinds::LAMBDA_EXPRESSION => self.lowerer.lambda(node).map(EOut::Comp),
            | node_kinds::CALL_EXPRESSION => {
                self.with_hoists(hoists, node, |lowerer, hoists| lowerer.call(node, hoists))
            },
            | node_kinds::FORCE_EXPRESSION => self.lowerer.force(node).map(EOut::Comp),
            | node_kinds::RET_EXPRESSION => self.lowerer.ret(node).map(EOut::Comp),
            | node_kinds::CASE_EXPRESSION => self.lowerer.case(node).map(EOut::Comp),
            | node_kinds::IF_EXPRESSION => self.lowerer.if_sugar(node).map(EOut::Comp),
            | node_kinds::CO_EXPRESSION => self.lowerer.co(node).map(EOut::Comp),
            | node_kinds::RECORD_EXPRESSION => self
                .with_hoists(hoists, node, |lowerer, hoists| {
                    lowerer.record_expr(node, hoists)
                })
                .map(EOut::Value),
            | node_kinds::RECORD_UPDATE_EXPRESSION => self.lowerer.record_update(node),
            | node_kinds::PROJECTION_EXPRESSION => self.lowerer.projection(node),
            | node_kinds::BINARY_EXPRESSION => self.lowerer.binary(node).map(EOut::Comp),
            | node_kinds::UNARY_EXPRESSION => self.lowerer.unary(node).map(EOut::Comp),
            | node_kinds::SHELL_BLOCK => self.lowerer.shell_block(node).map(EOut::Comp),
            | node_kinds::CONSTRUCTOR => match self.lowerer.bare_data_constructor(node)? {
                | Some(constructor) => Ok(EOut::Value(constructor)),
                | None => Err(LowerError::Unsupported {
                    kind: node.kind(),
                    byte_range: node.byte_range(),
                }),
            },
            | kind => Err(LowerError::Unsupported {
                kind,
                byte_range: node.byte_range(),
            }),
        }
    }
}

/// Partitions one block into its source-ordered statements and optional tail.
fn split_block<'tree>(
    lowerer: &Lowerer<'_>,
    node: SynNode<'tree>,
) -> LowerResult<(Vec<SynNode<'tree>>, Option<SynNode<'tree>>)>
{
    let children = super::named_non_extra_children(node);
    let last_index = children.len().saturating_sub(1);
    let mut statements = Vec::new();
    let mut tail = None;
    for (index, child) in children.into_iter().enumerate() {
        let kind = child.kind();
        let mut is_statement = kind == node_kinds::LET_STATEMENT
            || kind == node_kinds::BIND_STATEMENT
            || kind == node_kinds::UNPACK_STATEMENT
            || kind == node_kinds::EXPRESSION_STATEMENT
            || node_kinds::UNSUPPORTED_STATEMENTS.contains(&kind);
        if bool::from(lowerer.total()) && (child.is_error().0 || child.is_missing().0) {
            is_statement = !(index == last_index && tail.is_none());
        }
        if is_statement {
            if tail.is_some() {
                if bool::from(lowerer.total()) {
                    continue;
                }
                return Err(LowerError::MalformedNode {
                    kind: node.kind(),
                    byte_range: node.byte_range(),
                });
            }
            statements.push(child);
        }
        else if tail.is_some() {
            if bool::from(lowerer.total()) {
                continue;
            }
            return Err(LowerError::MalformedNode {
                kind: node.kind(),
                byte_range: node.byte_range(),
            });
        }
        else {
            tail = Some(child);
        }
    }
    Ok((statements, tail))
}

/// Converts a `let` right-hand side into its bound computation.
fn let_bound(
    node: SynNode<'_>,
    lowered: EOut,
) -> LowerResult<COut>
{
    match lowered {
        | EOut::Comp(comp) => Ok(comp),
        | EOut::Value(value) => COut::from_legacy_comp(
            &Comp::Ret(Rc::new({
                let readback_value = value.readback_value()?;
                identity(readback_value)
            })),
            OriginNode::new(super::entry(node, Some(ElabKind::LetValueBind)), vec![
                value.origin,
            ]),
        ),
    }
}

/// Assembles one computation bind from already-lowered children.
fn bind_outputs(
    bound: COut,
    binder: String,
    rest: COut,
    origin: OriginEntry,
) -> LowerResult<COut>
{
    COut::from_legacy_comp(
        &Comp::Bind(
            Rc::new({
                let readback_comp = bound.readback_comp()?;
                identity(readback_comp)
            }),
            binder,
            Rc::new({
                let readback_comp = rest.readback_comp()?;
                identity(readback_comp)
            }),
        ),
        OriginNode::new(origin, vec![bound.origin, rest.origin]),
    )
}

/// Everything an `unpack` statement carries into its body continuation.
///
/// Grouped into one struct so the continuation closure takes a single moved
/// value rather than five, which keeps it under the argument-count wall and
/// keeps the pieces named where they are used.
struct UnpackParts
{
    /// The lowered package value.
    scrut: VOut,
    /// The ascribed package signature.
    signature: ValueType,
    /// The atoms minted for this elimination, in signature order.
    atoms: Vec<SealId>,
    /// The module variable bound over the body.
    binder: String,
    /// The hoists the package value produced, wrapped around the elimination.
    hoists: Vec<Hoist>,
}

/// One suspended outer layer of an n-ary tuple-pattern split.
struct SplitLayer
{
    /// Scrutinee for this layer.
    scrutinee: VOut,
    /// First binder for this layer.
    first_binder: String,
    /// Fresh name carrying the remaining tuple.
    rest_binder: String,
    /// Elaboration attached to this layer.
    elaboration: Option<ElabKind>,
}

/// Builds one motive-less split from already-lowered children.
fn build_split(
    scrutinee: VOut,
    first_binder: String,
    second_binder: String,
    body: COut,
    span: &OriginEntry,
    elaboration: Option<ElabKind>,
) -> LowerResult<COut>
{
    COut::from_legacy_comp(
        &Comp::Split {
            scrut: Rc::new({
                let readback_value = scrutinee.readback_value()?;
                identity(readback_value)
            }),
            fst_name: first_binder,
            snd_name: second_binder,
            motive: None,
            body: Rc::new({
                let readback_comp = body.readback_comp()?;
                identity(readback_comp)
            }),
        },
        OriginNode::new(super::with_elab(span, elaboration), vec![
            scrutinee.origin,
            body.origin,
        ]),
    )
}

/// Builds an n-ary tuple-pattern split without native recursion.
fn finish_split_chain(
    lowerer: &mut Lowerer<'_>,
    elements: &[SynNode<'_>],
    scrutinee: VOut,
    rest: COut,
    span: &OriginEntry,
    elaboration: Option<ElabKind>,
) -> LowerResult<COut>
{
    let mut remaining = elements;
    let mut current_scrutinee = scrutinee;
    let mut current_elaboration = elaboration;
    let mut layers = Vec::new();
    let inner = loop {
        let Some((head, tail)) = remaining.split_first()
        else {
            return Err(LowerError::MalformedNode {
                kind: node_kinds::TUPLE_PATTERN,
                byte_range: span.byte_range.clone(),
            });
        };
        let first_binder = lowerer.pattern_binder(*head)?;
        if let (Some(last), 1) = (tail.first(), tail.len()) {
            let second_binder = lowerer.pattern_binder(*last)?;
            let split = build_split(
                current_scrutinee,
                first_binder,
                second_binder,
                rest,
                span,
                current_elaboration,
            )?;
            break split;
        }
        if tail.is_empty() {
            return Err(LowerError::MalformedNode {
                kind: node_kinds::TUPLE_PATTERN,
                byte_range: span.byte_range.clone(),
            });
        }
        let fresh = lowerer.fresh_name();
        let inner_scrutinee = VOut::from_legacy_value(
            &Value::Var(fresh.clone()),
            OriginNode::leaf(super::with_elab(span, Some(ElabKind::SplitNest))),
        )?;
        layers.push(SplitLayer {
            scrutinee: current_scrutinee,
            first_binder,
            rest_binder: fresh,
            elaboration: current_elaboration,
        });
        current_scrutinee = inner_scrutinee;
        current_elaboration = Some(ElabKind::SplitNest);
        remaining = tail;
    };
    layers.into_iter().rev().try_fold(inner, |body, layer| {
        build_split(
            layer.scrutinee,
            layer.first_binder,
            layer.rest_binder,
            body,
            span,
            layer.elaboration,
        )
    })
}

impl<'run, 'src, 'tree: 'run> Machine for LowerMachine<'run, 'src, 'tree>
{
    type Request = Request<'tree>;
    type Frame = Frame<'run, 'src, 'tree>;
    type Output = LowerResult<Lowered>;
    type Error = Infallible;

    fn begin(
        &mut self,
        request: Self::Request,
    ) -> Result<LowerStep<'run, 'src, 'tree>, Self::Error>
    {
        Ok(match request {
            | Request::Expr { node, hoists } => self.begin_expr(node, hoists),
            | Request::ValueExpr { node, hoists } => self.begin_value_expr(node, hoists),
            | Request::CompExpr { node } => self.begin_comp_expr(node),
            | Request::Annotation { node, hoists } => self.begin_annotation(node, hoists),
            | Request::ValueExprExpecting {
                node,
                expected,
                hoists,
            } => self.begin_value_expr_expecting(node, &expected, hoists),
            | Request::DataConstructor {
                call_node,
                plan,
                arguments,
                next,
                components,
                origins,
                hoists,
            } => Self::begin_data_constructor(
                call_node, plan, arguments, next, components, origins, hoists,
            ),
            | Request::Tuple { node, hoists } => Self::begin_tuple(node, hoists),
            | Request::ListExpr { node, hoists } => Self::begin_list_expr(node, hoists),
            | Request::Force { node } => self.begin_unary_comp(UnaryCompForm::Force, node),
            | Request::Ret { node } => self.begin_unary_comp(UnaryCompForm::Ret, node),
            | Request::Unary { node } => self.begin_unary_comp(UnaryCompForm::Negate, node),
            | Request::Pack { node, hoists } => self.begin_pack(node, hoists),
            | Request::Thunk { node } => self.begin_thunk(node),
            | Request::Lambda { node } => self.begin_lambda(node),
            | Request::Block { node } => self.begin_block(node),
            | Request::Chain {
                statements,
                tail,
                block_node,
            } => self.begin_chain(statements, tail, block_node),
            | Request::Statement {
                first,
                rest,
                tail,
                block_node,
                span,
            } => self.begin_statement(first, rest, tail, block_node, span),
            | Request::LetStatement {
                node,
                rest,
                tail,
                block_node,
                span,
            } => self.begin_let_statement(node, rest, tail, block_node, span),
        })
    }

    fn resume(
        &mut self,
        frame: Self::Frame,
        output: Self::Output,
    ) -> Result<LowerStep<'run, 'src, 'tree>, Self::Error>
    {
        Ok(frame.0(self, output))
    }
}

/// Assembles right-nested tuple output from source-ordered components.
fn finish_tuple(
    node: SynNode<'_>,
    lowered: Vec<VOut>,
) -> LowerResult<VOut>
{
    let total = lowered.len();
    let mut reversed = lowered.into_iter().rev();
    let Some(mut accumulator) = reversed.next()
    else {
        return Err(malformed(node));
    };
    for (index, component) in reversed.enumerate() {
        let elaboration = if index == total.saturating_sub(2) {
            None
        }
        else {
            Some(ElabKind::TupleNest)
        };
        let left = component.readback_value()?;
        let right = accumulator.readback_value()?;
        accumulator = VOut::from_legacy_value(
            &Value::Pair(Rc::new(left), Rc::new(right)),
            OriginNode::new(entry(node, elaboration), alloc::vec![
                component.origin,
                accumulator.origin
            ]),
        )?;
    }
    Ok(accumulator)
}

/// Assembles a list output from source-ordered elements.
fn finish_list(
    node: SynNode<'_>,
    lowered: Vec<VOut>,
) -> LowerResult<VOut>
{
    let mut elements = Vec::with_capacity(lowered.len());
    let mut origins = Vec::with_capacity(lowered.len());
    for element in lowered {
        let readback = element.readback_value()?;
        elements.push(Rc::new(readback));
        origins.push(element.origin);
    }
    VOut::from_legacy_value(
        &Value::List(elements),
        OriginNode::new(entry(node, None), origins),
    )
}

/// Builds a one-child computation former.
fn finish_unary_comp(
    form: UnaryCompForm,
    node: SynNode<'_>,
    value: VOut,
) -> LowerResult<COut>
{
    let payload = value.readback_value()?;
    let value_origin = value.origin;
    match form {
        | UnaryCompForm::Force => COut::from_legacy_comp(
            &Comp::Force(Rc::new(payload)),
            OriginNode::new(entry(node, None), alloc::vec![value_origin]),
        ),
        | UnaryCompForm::Ret => COut::from_legacy_comp(
            &Comp::Ret(Rc::new(payload)),
            OriginNode::new(entry(node, None), alloc::vec![value_origin]),
        ),
        | UnaryCompForm::Negate => {
            let elaboration = Some(ElabKind::OperatorElab);
            let function = VOut::from_legacy_value(
                &Value::var(node_kinds::UNARY_NEG),
                OriginNode::leaf(entry(node, elaboration)),
            )?;
            let function_value = function.readback_value()?;
            let head = COut::from_legacy_comp(
                &Comp::Force(Rc::new(function_value)),
                OriginNode::new(entry(node, elaboration), alloc::vec![function.origin]),
            )?;
            let head_comp = head.readback_comp()?;
            COut::from_legacy_comp(
                &Comp::App(Rc::new(head_comp), Rc::new(payload)),
                OriginNode::new(entry(node, elaboration), alloc::vec![
                    head.origin,
                    value_origin
                ]),
            )
        },
    }
}

/// Lowers one expression through the explicit-stack driver.
///
/// # Contract
/// - requires: `node` belongs to the CST view used to construct `lowerer`;
/// - ensures: preserves the existing expression rule and source-order hoists;
/// - provides: the migration seam through which every descendant becomes an
///   explicit request/frame pair;
/// - fails: returns the first structured lowering error;
/// - panics: none.
pub(super) fn expr(
    lowerer: &mut Lowerer<'_>,
    node: SynNode<'_>,
    hoists: &mut Vec<Hoist>,
) -> LowerResult<EOut>
{
    let input = mem::take(hoists);
    let root_hoists = HoistBufferId(0);
    let mut machine = LowerMachine::new(lowerer, input);
    let result = match run(&mut machine, Request::Expr {
        node,
        hoists: root_hoists,
    }) {
        | Ok(result) => result,
        | Err(never) => match never {},
    };
    *hoists = machine.take_hoists(root_hoists, node)?;
    expect_expr(result, node)
}

/// Lowers one expression in value position through the explicit-stack driver.
pub(super) fn value_expr(
    lowerer: &mut Lowerer<'_>,
    node: SynNode<'_>,
    hoists: &mut Vec<Hoist>,
) -> LowerResult<VOut>
{
    let input = mem::take(hoists);
    let root_hoists = HoistBufferId(0);
    let mut machine = LowerMachine::new(lowerer, input);
    let result = match run(&mut machine, Request::ValueExpr {
        node,
        hoists: root_hoists,
    }) {
        | Ok(result) => result,
        | Err(never) => match never {},
    };
    *hoists = machine.take_hoists(root_hoists, node)?;
    expect_value(result, node)
}

/// Lowers one annotation through the explicit-stack driver.
pub(super) fn annotation(
    lowerer: &mut Lowerer<'_>,
    node: SynNode<'_>,
    hoists: &mut Vec<Hoist>,
) -> LowerResult<EOut>
{
    let input = mem::take(hoists);
    let root_hoists = HoistBufferId(0);
    let mut machine = LowerMachine::new(lowerer, input);
    let result = match run(&mut machine, Request::Annotation {
        node,
        hoists: root_hoists,
    }) {
        | Ok(result) => result,
        | Err(never) => match never {},
    };
    *hoists = machine.take_hoists(root_hoists, node)?;
    expect_expr(result, node)
}

/// Lowers one declared-data constructor through the explicit-stack driver.
pub(super) fn data_constructor(
    lowerer: &mut Lowerer<'_>,
    call_node: SynNode<'_>,
    constructor: SynNode<'_>,
    arguments: &[SynNode<'_>],
    hoists: &mut Vec<Hoist>,
    expected_args: Option<&[ValueType]>,
) -> LowerResult<VOut>
{
    let plan =
        lowerer.prepare_data_constructor(call_node, constructor, arguments, expected_args)?;
    let capacity = plan.field_types().len();
    let input = mem::take(hoists);
    let root_hoists = HoistBufferId(0);
    let mut machine = LowerMachine::new(lowerer, input);
    let result = match run(&mut machine, Request::DataConstructor {
        call_node,
        plan,
        arguments: Rc::from(arguments),
        next: DataFieldIndex::FIRST,
        components: Vec::with_capacity(capacity),
        origins: Vec::with_capacity(capacity),
        hoists: root_hoists,
    }) {
        | Ok(result) => result,
        | Err(never) => match never {},
    };
    *hoists = machine.take_hoists(root_hoists, call_node)?;
    expect_value(result, call_node)
}

/// Lowers tuple components through the explicit-stack driver.
pub(super) fn tuple(
    lowerer: &mut Lowerer<'_>,
    node: SynNode<'_>,
    hoists: &mut Vec<Hoist>,
) -> LowerResult<VOut>
{
    run_value_collection(lowerer, node, hoists, ValueCollection::Tuple)
}

/// Lowers list elements through the explicit-stack driver.
pub(super) fn list_expr(
    lowerer: &mut Lowerer<'_>,
    node: SynNode<'_>,
    hoists: &mut Vec<Hoist>,
) -> LowerResult<VOut>
{
    run_value_collection(lowerer, node, hoists, ValueCollection::List)
}

/// Runs one collection request while preserving the caller's hoist buffer.
fn run_value_collection(
    lowerer: &mut Lowerer<'_>,
    node: SynNode<'_>,
    hoists: &mut Vec<Hoist>,
    collection: ValueCollection,
) -> LowerResult<VOut>
{
    let input = mem::take(hoists);
    let root_hoists = HoistBufferId(0);
    let mut machine = LowerMachine::new(lowerer, input);
    let request = match collection {
        | ValueCollection::Tuple => Request::Tuple {
            node,
            hoists: root_hoists,
        },
        | ValueCollection::List => Request::ListExpr {
            node,
            hoists: root_hoists,
        },
    };
    let result = match run(&mut machine, request) {
        | Ok(result) => result,
        | Err(never) => match never {},
    };
    *hoists = machine.take_hoists(root_hoists, node)?;
    expect_value(result, node)
}

/// Lowers one thunk literal through the explicit-stack driver.
pub(super) fn thunk(
    lowerer: &mut Lowerer<'_>,
    node: SynNode<'_>,
) -> LowerResult<VOut>
{
    let mut machine = LowerMachine::new(lowerer, Vec::new());
    let result = match run(&mut machine, Request::Thunk { node }) {
        | Ok(result) => result,
        | Err(never) => match never {},
    };
    expect_value(result, node)
}

/// Lowers one lambda literal through the explicit-stack driver.
pub(super) fn lambda(
    lowerer: &mut Lowerer<'_>,
    node: SynNode<'_>,
) -> LowerResult<COut>
{
    let mut machine = LowerMachine::new(lowerer, Vec::new());
    let result = match run(&mut machine, Request::Lambda { node }) {
        | Ok(result) => result,
        | Err(never) => match never {},
    };
    expect_comp(result, node)
}

/// Lowers one computation block through the explicit-stack driver.
pub(super) fn block(
    lowerer: &mut Lowerer<'_>,
    node: SynNode<'_>,
) -> LowerResult<COut>
{
    let mut machine = LowerMachine::new(lowerer, Vec::new());
    let result = match run(&mut machine, Request::Block { node }) {
        | Ok(result) => result,
        | Err(never) => match never {},
    };
    expect_comp(result, node)
}

/// Lowers one expression in computation position through the explicit-stack
/// driver.
pub(super) fn comp_expr(
    lowerer: &mut Lowerer<'_>,
    node: SynNode<'_>,
) -> LowerResult<COut>
{
    let mut machine = LowerMachine::new(lowerer, Vec::new());
    let result = match run(&mut machine, Request::CompExpr { node }) {
        | Ok(result) => result,
        | Err(never) => match never {},
    };
    expect_comp(result, node)
}

/// Lowers one `force` form through the explicit-stack driver.
pub(super) fn force(
    lowerer: &mut Lowerer<'_>,
    node: SynNode<'_>,
) -> LowerResult<COut>
{
    run_unary_comp(lowerer, node, UnaryCompForm::Force)
}

/// Lowers one `ret` form through the explicit-stack driver.
pub(super) fn ret(
    lowerer: &mut Lowerer<'_>,
    node: SynNode<'_>,
) -> LowerResult<COut>
{
    run_unary_comp(lowerer, node, UnaryCompForm::Ret)
}

/// Lowers one unary-negation form through the explicit-stack driver.
pub(super) fn unary(
    lowerer: &mut Lowerer<'_>,
    node: SynNode<'_>,
) -> LowerResult<COut>
{
    run_unary_comp(lowerer, node, UnaryCompForm::Negate)
}

/// Runs one computation former with a single value-position child.
fn run_unary_comp(
    lowerer: &mut Lowerer<'_>,
    node: SynNode<'_>,
    form: UnaryCompForm,
) -> LowerResult<COut>
{
    let mut machine = LowerMachine::new(lowerer, Vec::new());
    let request = match form {
        | UnaryCompForm::Force => Request::Force { node },
        | UnaryCompForm::Ret => Request::Ret { node },
        | UnaryCompForm::Negate => Request::Unary { node },
    };
    let result = match run(&mut machine, request) {
        | Ok(result) => result,
        | Err(never) => match never {},
    };
    expect_comp(result, node)
}

/// Extracts a sort-neutral expression result from one completed request.
fn expect_expr(
    result: LowerResult<Lowered>,
    node: SynNode<'_>,
) -> LowerResult<EOut>
{
    match result? {
        | Lowered::Expr(output) => Ok(output),
        | Lowered::Value(_) | Lowered::Comp(_) => Err(malformed(node)),
    }
}

/// Extracts a value result from one completed request.
fn expect_value(
    result: LowerResult<Lowered>,
    node: SynNode<'_>,
) -> LowerResult<VOut>
{
    match result? {
        | Lowered::Value(output) => Ok(output),
        | Lowered::Expr(_) | Lowered::Comp(_) => Err(malformed(node)),
    }
}

/// Extracts a computation result from one completed request.
fn expect_comp(
    result: LowerResult<Lowered>,
    node: SynNode<'_>,
) -> LowerResult<COut>
{
    match result? {
        | Lowered::Comp(output) => Ok(output),
        | Lowered::Expr(_) | Lowered::Value(_) => Err(malformed(node)),
    }
}

/// Builds the structured fallback for an impossible internal machine handle.
fn malformed(node: SynNode<'_>) -> LowerError
{
    LowerError::MalformedNode {
        kind: node.kind(),
        byte_range: node.byte_range(),
    }
}
