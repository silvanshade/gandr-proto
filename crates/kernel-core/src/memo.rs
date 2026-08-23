//! The sharing-aware checking differential and the collapse measurement.
//!
//! # What is being proved, and what is being measured
//!
//! Two different things, and conflating them is how a memo ships wrong.
//!
//! **Proved**: checking with the memo answers exactly what checking without it
//! answers. The comparison is not against a re-implementation — it is the same
//! function at two type parameters, one of which is the memo that never
//! answers. That is the property the memo's presence on the default path rests
//! on, and it is a permanent suite member so the rollback target stays
//! continuously proven rather than assumed.
//!
//! **Measured**: how much work the memo removes. On deeply self-similar terms
//! the checker without a memo expands one goal per *occurrence*, and with one
//! expands one per *distinct support* — an exponential against a linear count.
//!
//! # The differential's teeth
//!
//! A comparison that both sides pass for the wrong reason proves nothing, so
//! the suite carries a memo that has been **poisoned** with an entry the
//! checker would never record, chosen so the poisoned run must answer
//! differently: it turns a refusal into an acceptance. That is not a
//! hypothetical corruption — it is the shape of the exact hazard the memo's
//! one-call lifetime exists to prevent, since the choke point truncates the
//! arena after every verdict and a surviving entry would name re-allocated
//! nodes.

use alloc::vec::Vec;

use gandr_kernel_check_memo::CheckMemo;
use gandr_kernel_check_memo::NullMemo;
use gandr_kernel_check_memo::VerdictMemo;

use crate::arena::TermArena;
use crate::arena::ValueId;
use crate::arena::ValueTypeId;
use crate::check::check_declaration_with_memo;
use crate::decl::Declaration;
use crate::decl::DeclarationBuilder;
use crate::decl::LevelSignature;
use crate::env::AdmittedDeclaration;
use crate::error::KernelError;
use crate::levels::LevelContext;
use crate::levels::LevelParamCount;
use crate::support::NodeOutcome;
use crate::support::NodeSupport;
use crate::support::SupportPlane;

/// The memo the checker uses on its default path.
type LiveMemo = VerdictMemo<NodeSupport, NodeOutcome>;

/// No prior declarations.
fn no_entries() -> Vec<AdmittedDeclaration>
{
    Vec::new()
}

/// The monomorphic level context every fixture here checks under.
fn levels() -> LevelContext
{
    LevelContext::admit(LevelParamCount::from(0), Vec::new())
        .expect("a monomorphic level context admits")
}

/// How many times the composite former is iterated.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CompositeDepth(u32);

impl From<CompositeDepth> for u32
{
    #[inline]
    fn from(value: CompositeDepth) -> Self
    {
        value.0
    }
}

/// The depths the collapse law is pinned at, low to high. Three, because one
/// depth cannot distinguish an exponential law from a coincidence.
const PINNED_DEPTHS: [CompositeDepth; 3] =
    [CompositeDepth(8), CompositeDepth(12), CompositeDepth(16)];

/// The depth the shared/unshared comparison runs at — small enough that the
/// unshared spelling fits comfortably in the arena.
const DIFFERENTIAL_DEPTH: CompositeDepth = CompositeDepth(10);

/// A count of goals, occurrences, or supports.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct GoalCount(u64);

impl From<GoalCount> for u64
{
    #[inline]
    fn from(value: GoalCount) -> Self
    {
        value.0
    }
}

/// `2^depth`, saturating.
fn two_to_the(depth: CompositeDepth) -> GoalCount
{
    let mut power: u64 = 1;
    for _step in 0 .. u32::from(depth) {
        power = power.saturating_mul(2);
    }
    GoalCount(power)
}

/// The **maximally shared** iterated composite: the value `t` with `t₀ = ()`
/// and `t_{k+1} = (t_k, t_k)`, checked against `T₀ = Unit`,
/// `T_{k+1} = T_k × T_k`.
///
/// Both are built by naming the previous level's id **twice**, so the arena
/// holds `depth + 1` nodes per family while the term denotes a balanced tree of
/// `2^depth` leaves. This is the coherence-artifact shape in miniature: deep
/// self-similarity, where every level is two copies of the level beneath it.
fn shared_composite(
    arena: &mut TermArena,
    depth: CompositeDepth,
) -> (ValueTypeId, ValueId)
{
    let mut declared = arena.value_type_unit();
    let mut body = arena.value_unit();
    for _step in 0 .. u32::from(depth) {
        declared = arena.value_type_product(declared, declared);
        body = arena.value_pair(body, body);
    }
    (declared, body)
}

/// The **unshared** spelling of the same composite: every occurrence gets its
/// own arena node, folded bottom-up so nothing recurses.
fn unshared_composite(
    arena: &mut TermArena,
    depth: CompositeDepth,
) -> (ValueTypeId, ValueId)
{
    let leaves = usize::try_from(u64::from(two_to_the(depth))).expect("the leaf count fits");
    let mut types: Vec<ValueTypeId> = Vec::with_capacity(leaves);
    let mut values: Vec<ValueId> = Vec::with_capacity(leaves);
    for _leaf in 0 .. leaves {
        types.push(arena.value_type_unit());
        values.push(arena.value_unit());
    }
    while types.len() > 1 {
        let mut next_types: Vec<ValueTypeId> = Vec::new();
        let mut next_values: Vec<ValueId> = Vec::new();
        for slot in types.chunks(2) {
            let (Some(&first), Some(&second)) = (slot.first(), slot.last())
            else {
                panic!("each level has an even length");
            };
            next_types.push(arena.value_type_product(first, second));
        }
        for slot in values.chunks(2) {
            let (Some(&first), Some(&second)) = (slot.first(), slot.last())
            else {
                panic!("each level has an even length");
            };
            next_values.push(arena.value_pair(first, second));
        }
        types = next_types;
        values = next_values;
    }
    let (Some(&declared), Some(&body)) = (types.first(), values.first())
    else {
        panic!("the fold leaves one root per family");
    };
    (declared, body)
}

/// The unedited counterpart of [`edited_composite`]: the same composite paired
/// with **itself**, so the two fixtures differ by exactly one edit.
fn shared_composite_pair(
    arena: &mut TermArena,
    depth: CompositeDepth,
) -> (ValueTypeId, ValueId)
{
    let (declared, body) = shared_composite(arena, depth);
    let pair_type = arena.value_type_product(declared, declared);
    let pair = arena.value_pair(body, body);
    (pair_type, pair)
}

/// A **depth-`d` edit** of the shared composite, minted beside the original in
/// the same arena.
///
/// The edit is at the leaf, so its spine runs the whole way to the root: each
/// level is re-minted as a fresh node pairing the edited child with the
/// *original* untouched sibling. That is `depth + 1` new value nodes and
/// nothing else — every off-spine subterm is the very same arena id the
/// original names.
///
/// Returned as one declaration checking `(original, edited)` against
/// `T_d × T_d`, so both spellings are checked in **one** call. That matters:
/// the memo's lifetime is a single check, deliberately, so measuring an edit's
/// cost by warming a memo on the original and reusing it across calls would be
/// measuring exactly the unsound thing the lifetime rule forbids. Checking both
/// in one pass is the honest form of the same question — what does the edited
/// spine cost, given everything it shares with the original.
fn edited_composite(
    arena: &mut TermArena,
    depth: CompositeDepth,
) -> (ValueTypeId, ValueId)
{
    let mut declared = arena.value_type_unit();
    let mut original = arena.value_unit();
    let mut levels: Vec<ValueId> = Vec::new();
    levels.push(original);
    for _step in 0 .. u32::from(depth) {
        declared = arena.value_type_product(declared, declared);
        original = arena.value_pair(original, original);
        levels.push(original);
    }

    // The edited spine: a fresh leaf, then one fresh pair per level, each
    // sharing the original's untouched sibling.
    let mut edited = arena.value_unit();
    for level in 0 .. u32::from(depth) {
        let index = usize::try_from(level).expect("the level index fits");
        let Some(&sibling) = levels.get(index)
        else {
            panic!("every level below the root was recorded");
        };
        edited = arena.value_pair(edited, sibling);
    }

    let pair_type = arena.value_type_product(declared, declared);
    let body = arena.value_pair(original, edited);
    (pair_type, body)
}

/// Mint one definition's content into a fresh arena and finalize it.
///
/// A bare arena rather than an [`Environment`], so the differential can run the
/// same declaration twice without an admission mutating what the second run
/// sees.
fn stage<F>(build: F) -> (TermArena, Declaration)
where
    F: FnOnce(&mut TermArena) -> (ValueTypeId, ValueId),
{
    let mut arena = TermArena::new();
    let declaration = {
        let mut builder = DeclarationBuilder::new(&mut arena);
        let (declared, body) = build(builder.arena());
        builder.def(LevelSignature::monomorphic(), declared, body)
    };
    (arena, declaration)
}

/// Check one staged definition with `memo`, reporting the verdict.
fn check_with<M>(
    arena: &mut TermArena,
    declaration: &Declaration,
    memo: &mut M,
) -> Result<(), KernelError>
where
    M: CheckMemo<NodeSupport, NodeOutcome>,
{
    check_declaration_with_memo(arena, &no_entries(), &levels(), declaration, memo)
}

/// How many of a memo's entries belong to one plane.
///
/// # Contract
/// - requires: nothing.
/// - ensures: the count of recorded supports whose plane is `plane`.
/// - provides: the per-plane split of the collapse measurement, so a silently
///   dead memo on one machine cannot hide behind the other's numbers.
/// - fails: never.
/// - panics: never.
fn plane_count(
    memo: &LiveMemo,
    plane: SupportPlane,
) -> GoalCount
{
    let mut found: u64 = 0;
    for support in memo.supports() {
        if support.plane() == plane {
            found = found.saturating_add(1);
        }
    }
    GoalCount(found)
}

/// Check the same definition twice — once with the live memo, once with the
/// memo that never answers — and require the two verdicts to agree.
///
/// Each run gets its own arena built by the same closure, so the checker
/// intermediates one run mints cannot reach the other.
fn verdicts_agree<F>(build: F) -> Result<(), KernelError>
where
    F: Fn(&mut TermArena) -> (ValueTypeId, ValueId),
{
    let (mut fresh_arena, fresh_declaration) = stage(&build);
    let mut null = NullMemo;
    let memoless = check_with(&mut fresh_arena, &fresh_declaration, &mut null);

    let (mut memo_arena, memo_declaration) = stage(&build);
    let mut live: LiveMemo = VerdictMemo::new();
    let memoized = check_with(&mut memo_arena, &memo_declaration, &mut live);

    assert_eq!(
        memoless.is_ok(),
        memoized.is_ok(),
        "memoized checking must reach the memoless verdict: {memoless:?} against {memoized:?}"
    );
    if let (Err(memoless_error), Err(memoized_error)) = (memoless.as_ref(), memoized.as_ref()) {
        assert_eq!(
            memoless_error, memoized_error,
            "a refusal must be the same refusal, not merely a refusal"
        );
    }
    memoless
}

#[cfg(test)]
mod tests
{
    use gandr_kernel_check_memo::CheckMemo as _;
    use gandr_kernel_check_memo::MemoEntryCount;
    use gandr_kernel_check_memo::NullMemo;
    use gandr_kernel_check_memo::VerdictMemo;
    use gandr_kernel_strata::Level;
    use gandr_kernel_strata::LevelConstant;

    use super::CompositeDepth;
    use super::DIFFERENTIAL_DEPTH;
    use super::LiveMemo;
    use super::PINNED_DEPTHS;
    use super::check_with;
    use super::edited_composite;
    use super::plane_count;
    use super::shared_composite;
    use super::shared_composite_pair;
    use super::stage;
    use super::two_to_the;
    use super::unshared_composite;
    use super::verdicts_agree;
    use crate::arena::TermArena;
    use crate::arena::ValueId;
    use crate::arena::ValueTypeId;
    use crate::decl::LevelSignature;
    use crate::env::Environment;
    use crate::error::KernelError;
    use crate::probe;
    use crate::support::LooseDepth;
    use crate::support::NodeOutcome;
    use crate::support::NodeSupport;
    use crate::support::SupportGoal;
    use crate::support::SupportPlane;
    use crate::term::ConstantIndex;

    /// The arc's headline: what the memo removes, stated as a law rather than
    /// as one number.
    ///
    /// Checking one declaration runs two iterative machines over the shared
    /// graph — the checker's goal loop over the body and the type-formation
    /// walk over the declared type. Without a memo each expands a shared node
    /// once per **occurrence**, so at depth `d` they expand `5·2^d − 2` goals
    /// in total: `3·2^d − 1` body checks (one per `(t_k, T_k)` occurrence plus
    /// the synthesis each unit leaf falls through to) and `2^(d+1) − 1` type
    /// formations.
    ///
    /// With the memo, each expands once per **distinct support**: `2d + 3` in
    /// total — `d + 1` body checks, one leaf synthesis, and `d + 1` type
    /// formations. Every remaining goal is served as a hit without entering
    /// the node.
    ///
    /// **Occurrences are asserted, not only the small number.** A workload
    /// that quietly lost its sharing would report a memoless count equal to
    /// its memoized count and sail through any assertion that looked only at
    /// the collapse; pinning both sides at three depths is what makes the
    /// ratio mean something.
    #[test]
    fn the_memo_collapses_per_occurrence_checking_to_per_support_checking()
    {
        for depth in PINNED_DEPTHS {
            let power = u64::from(two_to_the(depth));
            let expected_memoless = power.saturating_mul(5).saturating_sub(2);
            let expected_memoized = u64::from(u32::from(depth))
                .saturating_mul(2)
                .saturating_add(3);

            let (mut fresh_arena, fresh_declaration) =
                stage(|arena| shared_composite(arena, depth));
            let mut null = NullMemo;
            probe::begin();
            let memoless = check_with(&mut fresh_arena, &fresh_declaration, &mut null);
            let memoless_work = probe::end();
            assert!(
                memoless.is_ok(),
                "the composite is well typed: {memoless:?}"
            );

            let (mut memo_arena, memo_declaration) = stage(|arena| shared_composite(arena, depth));
            let mut live: LiveMemo = VerdictMemo::new();
            probe::begin();
            let memoized = check_with(&mut memo_arena, &memo_declaration, &mut live);
            let memoized_work = probe::end();
            assert!(
                memoized.is_ok(),
                "the composite is well typed: {memoized:?}"
            );

            assert_eq!(
                expected_memoless,
                u64::from(memoless_work.expansions()),
                "at depth {depth:?} the memoless walks expand one goal per occurrence"
            );
            assert_eq!(
                0,
                u64::from(memoless_work.hits()),
                "the memo that never answers never answers"
            );
            assert_eq!(
                expected_memoized,
                u64::from(memoized_work.expansions()),
                "at depth {depth:?} the memo reduces the work to one goal per support"
            );
            assert_eq!(
                MemoEntryCount::from(
                    usize::try_from(expected_memoized).expect("the support count fits")
                ),
                live.entry_count(),
                "the memo recorded exactly the supports it expanded, counted independently"
            );

            // The split matters: one memo serves both machines, and an
            // aggregate count cannot tell a live memo from one whose type half
            // never fired. The term plane holds one check per level plus the
            // leaf synthesis; the type plane holds one formation per level.
            let expected_term = u64::from(u32::from(depth)).saturating_add(2);
            let expected_type = u64::from(u32::from(depth)).saturating_add(1);
            assert_eq!(
                expected_term,
                u64::from(plane_count(&live, SupportPlane::Term)),
                "the term plane collapsed to one check per level plus the leaf synthesis"
            );
            assert_eq!(
                expected_type,
                u64::from(plane_count(&live, SupportPlane::Type)),
                "the type plane collapsed to one formation per level, independently of the term plane"
            );
            // The memo does not merely answer repeats — it stops the machine
            // entering their subtrees, so the memoized run reaches far fewer
            // goals in total than the memoless run expanded. Pinning the hit
            // count keeps that distinction visible: hits are the goals reached
            // and served, not the occurrences avoided.
            assert!(
                u64::from(memoized_work.hits()) < expected_memoless,
                "the memo answers repeats without walking the subtrees beneath them"
            );
            assert!(
                u64::from(memoized_work.hits()) > 0,
                "a self-similar term must reach a repeated support at all"
            );
        }
    }

    /// Sharing bought the checker nothing before the memo, and this pins that
    /// baseline: the maximally shared composite and its fully unshared
    /// spelling drive exactly the same memoless goal count.
    ///
    /// The memo is what turns the representation's sharing into checking work
    /// saved, and the second half measures it: the shared spelling collapses,
    /// the unshared one cannot, because there is nothing to collapse.
    #[test]
    fn only_the_memo_turns_sharing_into_saved_checking()
    {
        let mut null = NullMemo;

        let (mut shared_arena, shared_declaration) =
            stage(|arena| shared_composite(arena, DIFFERENTIAL_DEPTH));
        probe::begin();
        let shared = check_with(&mut shared_arena, &shared_declaration, &mut null);
        let shared_memoless = probe::end();
        assert!(shared.is_ok(), "the shared composite checks: {shared:?}");

        let (mut plain_arena, plain_declaration) =
            stage(|arena| unshared_composite(arena, DIFFERENTIAL_DEPTH));
        probe::begin();
        let plain = check_with(&mut plain_arena, &plain_declaration, &mut null);
        let plain_memoless = probe::end();
        assert!(plain.is_ok(), "the unshared composite checks: {plain:?}");

        assert_eq!(
            u64::from(shared_memoless.expansions()),
            u64::from(plain_memoless.expansions()),
            "without a memo the checker walks the graph as a tree, so sharing changes no count"
        );

        let (mut shared_memo_arena, shared_memo_declaration) =
            stage(|arena| shared_composite(arena, DIFFERENTIAL_DEPTH));
        let mut shared_live: LiveMemo = VerdictMemo::new();
        probe::begin();
        let shared_memoized = check_with(
            &mut shared_memo_arena,
            &shared_memo_declaration,
            &mut shared_live,
        );
        let shared_work = probe::end();
        assert!(shared_memoized.is_ok(), "the shared composite checks");

        let (mut plain_memo_arena, plain_memo_declaration) =
            stage(|arena| unshared_composite(arena, DIFFERENTIAL_DEPTH));
        let mut plain_live: LiveMemo = VerdictMemo::new();
        probe::begin();
        let plain_memoized = check_with(
            &mut plain_memo_arena,
            &plain_memo_declaration,
            &mut plain_live,
        );
        let plain_work = probe::end();
        assert!(plain_memoized.is_ok(), "the unshared composite checks");

        let expected_shared = u64::from(u32::from(DIFFERENTIAL_DEPTH))
            .saturating_mul(2)
            .saturating_add(3);
        assert_eq!(
            expected_shared,
            u64::from(shared_work.expansions()),
            "the memo collapses the shared spelling to one goal per level"
        );
        assert_eq!(
            u64::from(plain_memoless.expansions()),
            u64::from(plain_work.expansions()),
            "the unshared spelling has nothing to collapse, so the memo saves it nothing"
        );
        assert_eq!(
            0,
            u64::from(plain_work.hits()),
            "no support recurs in the unshared spelling, so no goal is ever hit"
        );
    }

    /// **The capability, at the public choke point.** A self-similar
    /// definition whose tree expansion is over a billion nodes admits —
    /// checked, through [`Environment::add_decl`] — in a few dozen goal
    /// expansions.
    ///
    /// Depth twenty-eight is not an arbitrary number. It is the depth at which
    /// the repeated-pair diamond's expanded size passes the export reader's
    /// per-declaration work budget, which is the size the reader was given that
    /// budget to refuse; the same shape had to be admitted through the
    /// unchecked bypass to be built at all, because checking it meant walking
    /// `2^29` occurrences. The memo is what makes the checked path viable, and
    /// this is the assertion that says so.
    ///
    /// The reader's budget is untouched by this arc and still refuses such an
    /// artifact on **import**. What changed is the cost of *checking* a shared
    /// term, so the budget's role moves from being the sole bound on checker
    /// work to being one defence among two.
    #[test]
    fn a_self_similar_definition_past_the_tree_expansion_wall_admits_checked()
    {
        /// The depth whose tree expansion passes the reader's per-declaration
        /// work budget.
        const WALL_DEPTH: CompositeDepth = CompositeDepth(28);

        let mut environment = Environment::new();
        let declaration = {
            let mut builder = environment.stage();
            let (declared, body) = shared_composite(builder.arena(), WALL_DEPTH);
            builder.def(LevelSignature::monomorphic(), declared, body)
        };

        probe::begin();
        let admitted = environment.add_decl(declaration);
        let work = probe::end();

        assert!(
            admitted.is_ok(),
            "the self-similar definition admits through the checked choke point: {admitted:?}"
        );

        let expected = u64::from(u32::from(WALL_DEPTH))
            .saturating_mul(2)
            .saturating_add(3);
        assert_eq!(
            expected,
            u64::from(work.expansions()),
            "one goal per support, whatever the occurrence count"
        );

        let avoided = u64::from(two_to_the(WALL_DEPTH))
            .saturating_mul(5)
            .saturating_sub(2);
        assert!(
            avoided > 1_000_000_000,
            "the fixture's tree expansion really is past a billion goals ({avoided})"
        );
        assert!(
            u64::from(work.expansions()) < 100,
            "and the checked admission cost under a hundred"
        );
    }

    /// The differential over the self-similar corpus: the memoized verdict is
    /// the memoless verdict, at every depth the collapse law is pinned at.
    #[test]
    fn memoized_checking_agrees_with_memoless_checking()
    {
        for depth in PINNED_DEPTHS {
            let outcome = verdicts_agree(|arena| shared_composite(arena, depth));
            assert!(outcome.is_ok(), "the shared composite checks at {depth:?}");
        }
        let unshared = verdicts_agree(|arena| unshared_composite(arena, DIFFERENTIAL_DEPTH));
        assert!(unshared.is_ok(), "the unshared composite checks");
    }

    /// A definition whose body does not check: the memoized run must refuse it
    /// for the same reason the memoless run does.
    ///
    /// Failure paths matter more than success paths here, because a memo that
    /// wrongly answers turns a refusal into an acceptance — the direction that
    /// costs soundness rather than speed.
    #[test]
    fn memoized_checking_agrees_on_refusals()
    {
        let outcome = verdicts_agree(|arena| {
            // Declared `Unit + Unit`, body `()`. The unit falls through to
            // synthesis, synthesizes `Unit`, and the mode switch refuses the
            // conversion against a sum.
            let unit_type = arena.value_type_unit();
            let sum = arena.value_type_sum(unit_type, unit_type);
            let body = arena.value_unit();
            (sum, body)
        });
        assert!(
            outcome.is_err(),
            "a unit does not check against a sum, with or without a memo"
        );
    }

    /// A shared subterm sitting under **different binders**: the case the
    /// binder slice exists for.
    ///
    /// The body is a pair of two thunked identity functions over different
    /// domains, sharing one closed inner value. The shared value reads no
    /// binder, so its support carries an empty slice at both occurrences and
    /// collapses across the two binder positions — while the variable bodies,
    /// which do read a binder, keep their contexts apart.
    #[test]
    fn a_closed_subterm_shared_under_different_binders_still_agrees()
    {
        let outcome = verdicts_agree(|arena| {
            let unit_type = arena.value_type_unit();
            let sum = arena.value_type_sum(unit_type, unit_type);

            // A closed value used under both binders.
            let shared_unit = arena.value_unit();
            let shared_return = arena.computation_return(shared_unit);

            // `U (Unit -> F Unit)` inhabited by `thunk (\_. return ())`.
            let returner = arena.comp_type_returner(unit_type);
            let unit_arrow = arena.comp_type_arrow(unit_type, returner);
            let unit_thunk_type = arena.value_type_thunk(unit_arrow);
            let unit_lambda = arena.computation_lambda(shared_return);
            let unit_thunk = arena.value_thunk(unit_lambda);

            // `U ((Unit + Unit) -> F Unit)` inhabited by the same body.
            let sum_arrow = arena.comp_type_arrow(sum, returner);
            let sum_thunk_type = arena.value_type_thunk(sum_arrow);
            let sum_lambda = arena.computation_lambda(shared_return);
            let sum_thunk = arena.value_thunk(sum_lambda);

            let declared = arena.value_type_product(unit_thunk_type, sum_thunk_type);
            let body = arena.value_pair(unit_thunk, sum_thunk);
            (declared, body)
        });
        assert!(outcome.is_ok(), "both thunks check: {outcome:?}");
    }

    /// A shared subterm that **does** read a binder, under two binders bound to
    /// different types: the support must keep them apart.
    ///
    /// Both branches are `thunk (\x. return x)`, sharing the inner
    /// `return (var 0)` computation. That computation's answer depends on what
    /// `var 0` is bound to, which differs between the two occurrences — so a
    /// memo keyed without the binder slice would answer the second occurrence
    /// with the first's type. The differential is what would catch that, and
    /// it is asserted here directly.
    #[test]
    fn a_binder_reading_subterm_shared_under_different_types_still_agrees()
    {
        let outcome = verdicts_agree(|arena| {
            let unit_type = arena.value_type_unit();
            let sum = arena.value_type_sum(unit_type, unit_type);

            // The shared body `return (var 0)`, whose type is whatever the
            // enclosing binder bound.
            let variable = arena.value_variable(crate::term::DeBruijnIndex::from(0));
            let shared_body = arena.computation_return(variable);

            let unit_returner = arena.comp_type_returner(unit_type);
            let unit_arrow = arena.comp_type_arrow(unit_type, unit_returner);
            let unit_thunk_type = arena.value_type_thunk(unit_arrow);
            let unit_lambda = arena.computation_lambda(shared_body);
            let unit_thunk = arena.value_thunk(unit_lambda);

            let sum_returner = arena.comp_type_returner(sum);
            let sum_arrow = arena.comp_type_arrow(sum, sum_returner);
            let sum_thunk_type = arena.value_type_thunk(sum_arrow);
            let sum_lambda = arena.computation_lambda(shared_body);
            let sum_thunk = arena.value_thunk(sum_lambda);

            let declared = arena.value_type_product(unit_thunk_type, sum_thunk_type);
            let body = arena.value_pair(unit_thunk, sum_thunk);
            (declared, body)
        });
        assert!(
            outcome.is_ok(),
            "each identity checks at its own domain: {outcome:?}"
        );
    }

    /// **The differential's teeth.** A memo carrying one entry the checker
    /// would never have recorded must change the verdict, and the differential
    /// must see it.
    ///
    /// The poisoned entry claims that a value **already checked** against a
    /// type it in fact refuses. This is not an arbitrary corruption: it is the
    /// exact shape of the stale-entry hazard the memo's one-call lifetime
    /// exists to prevent. The choke point truncates the arena after every
    /// verdict, so an entry surviving into a later declaration would name ids
    /// that had been re-allocated to different nodes — and would answer a
    /// question about one term with the answer to another, which is what is
    /// staged here.
    ///
    /// The assertion is that the two runs **disagree**. A differential that
    /// stayed green here would be proving nothing.
    #[test]
    fn a_poisoned_memo_changes_the_verdict()
    {
        // `()` against `Unit + Unit` — a refusal, established first so the
        // poison is known to be turning a real refusal into an acceptance.
        let build = |arena: &mut TermArena| -> (ValueTypeId, ValueId) {
            let unit_type = arena.value_type_unit();
            let sum = arena.value_type_sum(unit_type, unit_type);
            let body = arena.value_unit();
            (sum, body)
        };

        let (mut honest_arena, honest_declaration) = stage(build);
        let mut honest: LiveMemo = VerdictMemo::new();
        let honest_verdict = check_with(&mut honest_arena, &honest_declaration, &mut honest);
        assert!(
            honest_verdict.is_err(),
            "the unpoisoned run refuses, so the poison has something to overturn"
        );

        let (mut poisoned_arena, poisoned_declaration) = stage(build);
        let mut poisoned: LiveMemo = VerdictMemo::new();
        let crate::decl::DeclarationContent::Def { declared, body } =
            *poisoned_declaration.content()
        else {
            panic!("the fixture stages a definition")
        };
        // The body is closed, so its support carries no binder slice — the
        // same key the checker itself would build for this goal.
        poisoned.remember(
            NodeSupport::new(
                SupportGoal::CheckValue(body, declared),
                &[],
                LooseDepth::from(0),
            ),
            NodeOutcome::Checked,
        );
        let poisoned_verdict =
            check_with(&mut poisoned_arena, &poisoned_declaration, &mut poisoned);

        assert!(
            poisoned_verdict.is_ok(),
            "a consulted poison must change the answer, or the differential proves nothing"
        );
        assert_ne!(
            honest_verdict.is_ok(),
            poisoned_verdict.is_ok(),
            "the differential separates the poisoned run from the honest one"
        );
    }

    /// **Rung (b), edit locality — a measurement, not an adoption.**
    ///
    /// A depth-`d` edit to a shared term re-mints its spine and nothing else.
    /// The prediction is that checking the edited spelling beside the original
    /// costs a number of extra goal expansions **linear in `d`**, against an
    /// occurrence count exponential in it — the same `O(d)`-spine bound the
    /// storage tier's locality theorem has for bytes, now measured for
    /// checking work.
    ///
    /// Measured: the edited spelling adds exactly `d + 2` supports — one check
    /// per spine level plus the fresh leaf's synthesis. The type plane adds
    /// nothing at all, because the edit changes no type.
    ///
    /// The `+ 2` rather than `+ 1` is the leaf: the edited leaf is a *distinct
    /// arena node* from the original's, so it is checked and synthesized on its
    /// own account even though it is structurally identical. That is the
    /// id-keyed memo's conservatism showing up as a number — a content-keyed
    /// memo would collapse the two, at the cost of hashing every node.
    #[test]
    fn an_edit_re_checks_its_spine_and_nothing_else()
    {
        for depth in PINNED_DEPTHS {
            let (mut plain_arena, plain_declaration) =
                stage(|arena| shared_composite_pair(arena, depth));
            let mut plain: LiveMemo = VerdictMemo::new();
            probe::begin();
            let plain_verdict = check_with(&mut plain_arena, &plain_declaration, &mut plain);
            let plain_work = probe::end();
            assert!(plain_verdict.is_ok(), "the unedited pair checks");

            let (mut edited_arena, edited_declaration) =
                stage(|arena| edited_composite(arena, depth));
            let mut edited: LiveMemo = VerdictMemo::new();
            probe::begin();
            let edited_verdict = check_with(&mut edited_arena, &edited_declaration, &mut edited);
            let edited_work = probe::end();
            assert!(edited_verdict.is_ok(), "the edited pair checks");

            let extra = u64::from(edited_work.expansions())
                .saturating_sub(u64::from(plain_work.expansions()));
            let predicted = u64::from(u32::from(depth)).saturating_add(2);
            assert_eq!(
                predicted, extra,
                "a depth-{depth:?} edit re-checks its spine plus its fresh leaf, and nothing else"
            );

            assert_eq!(
                plane_count(&plain, SupportPlane::Type),
                plane_count(&edited, SupportPlane::Type),
                "the edit changes no type, so the type plane does no extra work"
            );

            // The bound is worth stating against what it is bounding: the
            // edited term's occurrence count is exponential in the depth, and
            // the extra work is linear.
            let occurrences = u64::from(two_to_the(depth))
                .saturating_mul(2)
                .saturating_sub(1);
            assert!(
                extra < occurrences,
                "the spine is a vanishing fraction of the {occurrences} occurrences it sits in"
            );
        }
    }

    /// **Rung (c), consulted definitional unfoldings — the verdict is that at
    /// this kernel subset there are none to record.**
    ///
    /// The support of a check would have to include the definitional unfoldings
    /// the conversion consulted, if conversion consulted any. Here it does not:
    /// a constant reference synthesizes the referent's **declared type**, never
    /// its body, and type conversion decides by id-equality then structure over
    /// canonical levels. No rule reads a definition's value, so no unfolding
    /// enters any support.
    ///
    /// This pins that architecturally rather than by inspection: two
    /// declarations with the same declared type and **different bodies** are
    /// interchangeable as far as a dependent's checking is concerned — same
    /// verdict, same goal count, same supports on both planes. Were an
    /// unfolding consulted anywhere, this is the assertion that would break.
    #[test]
    fn a_referent_body_is_never_consulted_so_no_unfolding_enters_a_support()
    {
        /// Which body the referent gets — same declared type, different
        /// computation.
        #[derive(Clone, Copy)]
        enum BaseBody
        {
            /// `thunk (return ())`.
            Direct,
            /// `thunk (bind _ <- return (); return ())`.
            Sequenced,
        }

        /// Admit `def base : U (F Unit) = <body>`, then a dependent naming it,
        /// reporting the dependent's checking work.
        ///
        /// The two bodies compute differently and share a declared type:
        /// `thunk (return ())` against `thunk (bind _ <- return (); return
        /// ())`.
        fn dependent_work(shape: BaseBody) -> probe::ProbeReport
        {
            let mut environment = Environment::new();
            let base = {
                let mut builder = environment.stage();
                let arena = builder.arena();
                let unit_type = arena.value_type_unit();
                let returner = arena.comp_type_returner(unit_type);
                let declared = arena.value_type_thunk(returner);
                let inner = arena.value_unit();
                let returned = arena.computation_return(inner);
                let computation = match shape {
                    | BaseBody::Direct => returned,
                    | BaseBody::Sequenced => {
                        let other = arena.value_unit();
                        let bound = arena.computation_return(other);
                        arena.computation_bind(bound, returned)
                    },
                };
                let body = arena.value_thunk(computation);
                builder.def(LevelSignature::monomorphic(), declared, body)
            };
            let _admitted = environment
                .add_decl(base)
                .expect("the base definition admits");

            let declaration = {
                let mut builder = environment.stage();
                let arena = builder.arena();
                let unit_type = arena.value_type_unit();
                let returner = arena.comp_type_returner(unit_type);
                let declared = arena.value_type_thunk(returner);
                let referent = arena.value_constant(ConstantIndex::from(0_usize));
                builder.def(LevelSignature::monomorphic(), declared, referent)
            };

            probe::begin();
            let admitted = environment.add_decl(declaration);
            let work = probe::end();
            assert!(
                admitted.is_ok(),
                "the dependent checks against the referent's declared type: {admitted:?}"
            );
            work
        }

        let with_plain_body = dependent_work(BaseBody::Direct);
        let with_other_body = dependent_work(BaseBody::Sequenced);

        assert_eq!(
            u64::from(with_plain_body.expansions()),
            u64::from(with_other_body.expansions()),
            "no rule consults the referent's body, so it changes no checking work"
        );
        assert_eq!(
            u64::from(with_plain_body.hits()),
            u64::from(with_other_body.hits()),
            "and nothing about which of the dependent's supports recur"
        );
    }

    /// **The type plane's teeth.** The memo serves two machines, and the
    /// type-formation half needs its own answer-changing poison — otherwise
    /// its correctness rides on the term plane's assertions, which cannot see
    /// it.
    ///
    /// The declaration is `Lift Unit to level 1`, inhabited by the matching
    /// lifted unit. It admits: `Unit` forms at level zero, and a lift must
    /// raise strictly, so `0 < 1` holds. The poison claims `Unit` forms at
    /// level **two**, which no checker would record — and then `2 < 1` fails
    /// and the declaration is refused.
    ///
    /// The assertion is that the two runs **disagree**, in the direction that
    /// matters least for soundness and most for demonstrating consultation:
    /// the entry is provably read, on the type plane, and provably decides the
    /// verdict.
    #[test]
    fn a_poisoned_type_formation_entry_changes_the_verdict()
    {
        let build = |arena: &mut TermArena| -> (ValueTypeId, ValueId) {
            let inner = arena.value_type_unit();
            let one = Level::constant(LevelConstant::from(1_u64));
            let declared = arena.value_type_lift(inner, one.clone());
            let unit = arena.value_unit();
            let body = arena.value_lift(one, unit);
            (declared, body)
        };

        let (mut honest_arena, honest_declaration) = stage(build);
        let mut honest: LiveMemo = VerdictMemo::new();
        let honest_verdict = check_with(&mut honest_arena, &honest_declaration, &mut honest);
        assert!(
            honest_verdict.is_ok(),
            "the lift raises strictly, so it admits: {honest_verdict:?}"
        );
        assert!(
            u64::from(plane_count(&honest, SupportPlane::Type)) > 0,
            "the honest run recorded type-formation supports at all"
        );

        let (mut poisoned_arena, poisoned_declaration) = stage(build);
        let crate::decl::DeclarationContent::Def { declared, .. } = *poisoned_declaration.content()
        else {
            panic!("the fixture stages a definition")
        };
        // The lift's inner type is the node minted immediately before the
        // lift, and a type support reads no binder.
        let crate::types::ValueType::Lift { inner, .. } = *poisoned_arena
            .value_type(declared)
            .expect("the declared type resolves")
        else {
            panic!("the fixture declares a lift")
        };
        let mut poisoned: LiveMemo = VerdictMemo::new();
        poisoned.remember(
            NodeSupport::closed(SupportGoal::ValueTypeLevel(inner)),
            NodeOutcome::Formed(Level::constant(LevelConstant::from(2_u64))),
        );
        let poisoned_verdict =
            check_with(&mut poisoned_arena, &poisoned_declaration, &mut poisoned);

        assert!(
            matches!(poisoned_verdict, Err(KernelError::UniverseViolation(_))),
            "the poisoned level must be consulted and must refuse the lift: {poisoned_verdict:?}"
        );
        assert_ne!(
            honest_verdict.is_ok(),
            poisoned_verdict.is_ok(),
            "the differential separates the poisoned type plane from the honest one"
        );
    }

    /// A type-formation support carrying a **term** answer is declined rather
    /// than trusted.
    ///
    /// One memo serves both machines, so an entry's shape can disagree with
    /// the plane asking for it. The type walk accepts only a formed level; any
    /// other outcome is treated as a miss and recomputed. That costs one
    /// recomputation and cannot fabricate a level — the analogue, on the type
    /// side, of the binder-slice non-service below.
    #[test]
    fn a_type_formation_support_carrying_a_term_answer_is_not_served()
    {
        let build = |arena: &mut TermArena| -> (ValueTypeId, ValueId) {
            let inner = arena.value_type_unit();
            let one = Level::constant(LevelConstant::from(1_u64));
            let declared = arena.value_type_lift(inner, one.clone());
            let unit = arena.value_unit();
            let body = arena.value_lift(one, unit);
            (declared, body)
        };
        let (mut arena, declaration) = stage(build);
        let crate::decl::DeclarationContent::Def { declared, .. } = *declaration.content()
        else {
            panic!("the fixture stages a definition")
        };
        let crate::types::ValueType::Lift { inner, .. } = *arena
            .value_type(declared)
            .expect("the declared type resolves")
        else {
            panic!("the fixture declares a lift")
        };
        let mut memo: LiveMemo = VerdictMemo::new();
        memo.remember(
            NodeSupport::closed(SupportGoal::ValueTypeLevel(inner)),
            NodeOutcome::Checked,
        );
        let verdict = check_with(&mut arena, &declaration, &mut memo);
        assert!(
            verdict.is_ok(),
            "the wrong-shaped entry is declined and the level recomputed: {verdict:?}"
        );
    }

    /// **The binder slice is load-bearing in the key**, proved the only way it
    /// can be: by a poison that the key's binder component is what refuses.
    ///
    /// The entry carries the right node and the right direction and expected
    /// type — everything but the context. It says a value *already checked*
    /// against a type it in fact refuses, under a one-slot binder context. The
    /// declaration's body is closed and is checked under the empty context, so
    /// a key that dropped the binder component would match this entry and the
    /// refusal would flip to an acceptance.
    ///
    /// The assertion is that the verdict stays a refusal. That is the same
    /// poison as the teeth check, differing only in the component being
    /// probed, and it is the reason the key cannot be narrowed to the node and
    /// the type alone.
    #[test]
    fn a_poison_differing_only_in_its_binder_slice_is_refused()
    {
        let build = |arena: &mut TermArena| -> (ValueTypeId, ValueId) {
            let unit_type = arena.value_type_unit();
            let sum = arena.value_type_sum(unit_type, unit_type);
            let body = arena.value_unit();
            (sum, body)
        };
        let (mut arena, declaration) = stage(build);
        let crate::decl::DeclarationContent::Def { declared, body } = *declaration.content()
        else {
            panic!("the fixture stages a definition")
        };
        let mut memo: LiveMemo = VerdictMemo::new();
        memo.remember(
            NodeSupport::new(
                SupportGoal::CheckValue(body, declared),
                &[declared],
                LooseDepth::from(1),
            ),
            NodeOutcome::Checked,
        );
        let verdict = check_with(&mut arena, &declaration, &mut memo);
        assert!(
            verdict.is_err(),
            "the entry's binder slice does not match the goal's, so it must not be served"
        );
    }
}
