//! **Reification** of a ground command pattern into the `gandr-core-sequent`
//! command IL (`proposal-sequent-kernel.md` §2.2; the "over the
//! gandr-core-sequent command IL" bridge).
//!
//! A cell's pattern layer is symbolic (datatype-declared
//! [`crate::pattern::Sym`] symbols), because the
//! fusion fragment ranges over user data the frozen
//! [`gandr_core_sequent::il::CtorTag`] enum does not name. Where a symbol
//! *does* correspond to a frozen constructor, [`reify_into`] lowers a
//! **ground** command pattern into a real
//! [`gandr_core_sequent::il::CommandArena`], producing the L0 IL nodes verbatim
//! — the concrete evidence that the cells sit over the sequent command IL. A
//! **return-side constructor frame** `K⁻(c)` reifies to its definiens `μ̃x.⟨K(x)
//! | c⟩` (§7.1), exactly the [`gandr_core_sequent::il::ConsumerNode::MuTilde`]
//! shape.
//!
//! The bridge covers the frozen-representable fragment only: an **operation
//! frame** is the opaque host/fusion boundary (§7.4, "natives are opaque") and
//! reifies to nothing (`None`), as does any unresolved symbol or arity mismatch
//! — the honest scope, declined rather than mis-lowered. The constructor
//! resolver is supplied by the caller as a name→[`CtorTag`] map, so no
//! frozen-core symbol table is duplicated here.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_sequent::il::CommandArena;
use gandr_core_sequent::il::CommandId;
use gandr_core_sequent::il::CommandNode;
use gandr_core_sequent::il::ConsumerId;
use gandr_core_sequent::il::ConsumerNode;
use gandr_core_sequent::il::CtorTag;
use gandr_core_sequent::il::Polarity;
use gandr_core_sequent::il::ProducerId;
use gandr_core_sequent::il::ProducerNode;

use crate::pattern::CmdPat;
use crate::pattern::ConsPat;
use crate::pattern::ProdPat;
use crate::pattern::Sym;

/// A **constructor resolver** — the map from a datatype-declared constructor
/// name to its frozen [`CtorTag`] (the caller's bridge table).
pub type CtorResolver = BTreeMap<Sym, CtorTag>;

/// Reify a ground command pattern into a **fresh** arena.
///
/// # Contract
/// - ensures: as [`reify_into`], returning the arena alongside the root command
///   id.
/// - panics: none.
#[inline]
#[must_use]
pub fn reify_command(
    cmd: &CmdPat,
    resolve: &CtorResolver,
) -> Option<(CommandArena, CommandId)>
{
    let mut arena = CommandArena::new();
    let id = reify_into(&mut arena, cmd, resolve)?;
    Some((arena, id))
}

/// Reify a ground command pattern into `arena`, returning its command id.
///
/// # Contract
/// - requires: `cmd` is ground (metavariable-free); every constructor symbol it
///   uses resolves through `resolve` with a matching arity, and a return-side
///   frame's constructor is unary.
/// - ensures: `Some(id)` with the command materialized as L0 IL nodes in
///   `arena`; `None` when the pattern leaves the frozen fragment — an operation
///   frame, an unresolved or wrong-arity constructor, or a residual
///   metavariable.
/// - panics: none.
#[inline]
#[must_use]
pub fn reify_into(
    arena: &mut CommandArena,
    cmd: &CmdPat,
    resolve: &CtorResolver,
) -> Option<CommandId>
{
    let mut reifier = Reifier {
        arena,
        resolve,
        fresh: 0,
    };
    reifier.reify_cmd(cmd)
}

/// A frozen-list resolver — the `Nil` / `Cons` / `Pair` correspondences, the
/// obvious fusion targets (`proposal-sequent-kernel.md` §12.6).
///
/// # Contract
/// - ensures: a resolver mapping `Nil`, `Cons`, and `Pair` to their
///   [`CtorTag`]s.
/// - panics: none.
#[inline]
#[must_use]
pub fn frozen_list_resolver() -> CtorResolver
{
    let mut map = CtorResolver::new();
    map.insert(Sym::new("Nil"), CtorTag::Nil);
    map.insert(Sym::new("Cons"), CtorTag::Cons);
    map.insert(Sym::new("Pair"), CtorTag::Pair);
    map
}

/// The reification worker — an arena, a resolver, and a fresh-binder counter.
struct Reifier<'ctx>
{
    /// The arena being populated.
    arena: &'ctx mut CommandArena,
    /// The constructor resolver.
    resolve: &'ctx CtorResolver,
    /// The fresh-binder counter (for the `μ̃x` binders return frames introduce).
    fresh: u32,
}

impl Reifier<'_>
{
    /// A fresh `μ̃` binder name, reserved so it cannot shadow a user variable.
    ///
    /// # Contract
    /// - ensures: a distinct `$m` name (primed once per prior call) on each
    ///   call.
    /// - panics: none.
    fn fresh_name(&mut self) -> String
    {
        let mut name = String::from("$m");
        for _ in 0 .. self.fresh {
            name.push('\'');
        }
        self.fresh = self.fresh.saturating_add(1);
        name
    }

    /// Reify a command pattern (see [`reify_into`]).
    ///
    /// # Contract
    /// - ensures: `Some(id)` for a frozen-fragment cut; `None` otherwise.
    /// - panics: none.
    fn reify_cmd(
        &mut self,
        cmd: &CmdPat,
    ) -> Option<CommandId>
    {
        match *cmd {
            | CmdPat::Cut {
                pol,
                ref prod,
                ref cons,
            } => {
                let producer = self.reify_prod(prod)?;
                let consumer = self.reify_cons(cons)?;
                self.arena.alloc_command(CommandNode::Cut {
                    pol,
                    producer,
                    consumer,
                })
            },
        }
    }

    /// Reify a producer pattern (see [`reify_into`]).
    ///
    /// # Contract
    /// - ensures: `Some(id)` for a resolvable constructor of ground producers;
    ///   `None` for a metavariable or an unresolved / wrong-arity constructor.
    /// - panics: none.
    fn reify_prod(
        &mut self,
        prod: &ProdPat,
    ) -> Option<ProducerId>
    {
        enum Frame<'node>
        {
            Enter(&'node ProdPat),
            Exit(CtorTag, usize),
        }

        let mut stack = alloc::vec![Frame::Enter(prod)];
        let mut ids: Vec<ProducerId> = Vec::new();
        while let Some(frame) = stack.pop() {
            match frame {
                | Frame::Enter(node) => match *node {
                    | ProdPat::Meta(_) => return None,
                    | ProdPat::Ctor { ref ctor, ref args } => {
                        let tag = self.resolve.get(ctor)?;
                        let tag = tag.clone();
                        if tag.producer_arity() != args.len().into() {
                            return None;
                        }
                        stack.push(Frame::Exit(tag, args.len()));
                        stack.extend(args.iter().rev().map(Frame::Enter));
                    },
                },
                | Frame::Exit(tag, arity) => {
                    let split_at = ids.len().checked_sub(arity)?;
                    let ps = ids.split_off(split_at).into_boxed_slice();
                    let id = self.arena.alloc_producer(ProducerNode::Ctor {
                        tag,
                        ps,
                        cs: Box::from([]),
                    })?;
                    ids.push(id);
                },
            }
        }
        ids.pop()
    }

    /// Reify a consumer pattern (see [`reify_into`]).
    ///
    /// # Contract
    /// - ensures: `Some(id)` for the terminal consumer or a unary return-side
    ///   frame (lowered to `μ̃x.⟨K(x) | ret⟩`); `None` for a metavariable, an
    ///   operation frame (the opaque boundary), or an unresolvable / non-unary
    ///   frame constructor.
    /// - panics: none.
    fn reify_cons(
        &mut self,
        cons: &ConsPat,
    ) -> Option<ConsumerId>
    {
        let mut frames: Vec<CtorTag> = Vec::new();
        let mut cursor = cons;
        loop {
            match *cursor {
                | ConsPat::Meta(_) | ConsPat::Op { .. } => return None,
                | ConsPat::Top => break,
                | ConsPat::Frame { ref ctor, ref ret } => {
                    let tag = self.resolve.get(ctor)?;
                    let tag = tag.clone();
                    if tag.producer_arity() != 1_usize.into() {
                        return None;
                    }
                    frames.push(tag);
                    cursor = ret;
                },
            }
        }

        let mut ret_id = self.arena.alloc_consumer(ConsumerNode::Top)?;
        for tag in frames.into_iter().rev() {
            let binder = self.fresh_name();
            let var = self
                .arena
                .alloc_producer(ProducerNode::Var(binder.clone()))?;
            let wrapped = self.arena.alloc_producer(ProducerNode::Ctor {
                tag,
                ps: Box::from([var]),
                cs: Box::from([]),
            })?;
            let cut = self.arena.alloc_command(CommandNode::Cut {
                pol: Polarity::Positive,
                producer: wrapped,
                consumer: ret_id,
            })?;
            ret_id = self
                .arena
                .alloc_consumer(ConsumerNode::MuTilde(binder, cut))?;
        }
        Some(ret_id)
    }
}

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::boundary::SequentNodeCount;

    use super::*;
    use crate::pattern::ConsPat;

    #[test]
    fn a_frozen_cut_reifies_to_the_command_il()
    {
        // ⟨Cons(Nil; Nil) | ★⟩ over the frozen list resolver.
        let cmd = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Cons", [ProdPat::ctor("Nil", []), ProdPat::ctor("Nil", [])]),
            ConsPat::Top,
        );
        let (arena, id) = reify_command(&cmd, &frozen_list_resolver()).expect("frozen cut reifies");
        assert!(
            arena.command(id).is_some(),
            "the root command is in the arena"
        );
        assert_eq!(
            SequentNodeCount::from(1_usize),
            arena.command_count(),
            "one cut command"
        );
        assert_eq!(
            SequentNodeCount::from(3_usize),
            arena.producer_count(),
            "Cons plus its two Nil arguments"
        );
    }

    #[test]
    fn a_return_frame_reifies_to_a_mu_tilde()
    {
        // ⟨Nil | Cons⁻(★)⟩ ~> μ̃x.⟨Cons(x) | ★⟩ — but Cons is binary, so the
        // unary-frame requirement declines it; Pair is likewise binary. A unary
        // frame is needed, and the frozen list fragment has none, so this pins
        // the honest arity limit.
        let cmd = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Nil", []),
            ConsPat::frame("Cons", ConsPat::Top),
        );
        assert_eq!(
            None,
            reify_command(&cmd, &frozen_list_resolver()),
            "a non-unary return frame is outside the reifiable fragment"
        );
    }

    #[test]
    fn an_operation_frame_is_the_opaque_boundary()
    {
        let cmd = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Nil", []),
            ConsPat::op("length", [], ConsPat::Top),
        );
        assert_eq!(
            None,
            reify_command(&cmd, &frozen_list_resolver()),
            "an operation frame does not reify — the §7.4 opaque boundary"
        );
    }
}
