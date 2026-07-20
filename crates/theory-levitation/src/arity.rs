//! Multi-out **arities** — the bridge diagram `A ←s— J —π→ I —t→ B`
//! (proposal-levitation.md §4.2, V4).
//!
//! A multi-output operation `(X_a)_{a∈A} ↦ (Y_b)_{b∈B}` with
//! `Y_b = Σ_{i∈I_b} Π_{j∈J_i} X_{s(j)}` is presented by four finite sets and
//! three maps, computed as `Σ_t ∘ Π_π ∘ Δ_s`. The encoding separates the
//! **Π-layer** (one operation's named result tuple) from the **Σ-layer**
//! (destination aggregation, which independently requires a commutative monoid
//! — ADR-49's "Σ-zone" firewall). It is finite sets + maps: content-addressable
//! and first-order.

use crate::boundary::MonomialCount;
use crate::code::Name;

/// A named **port** of an operation's input (`A`) or output (`B`) tuple.
///
/// Stage 0 keeps the port's *sort* symbolic (its name), like the rest of the
/// stage-0 type surface; a port maps to a value type once the arity feeds the
/// value decoder.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "a stage-0 port is exactly its {name, symbolic sort}; the elaborator constructs bridge arities from op members"
)]
pub struct SortRef
{
    /// The port's name (`q`, `r` in `-> (q: …, r: …)`).
    pub name: Name,
    /// The port's symbolic sort spelling.
    pub sort: Name,
}

impl SortRef
{
    /// A port of the given name and symbolic sort.
    #[inline]
    #[must_use]
    pub fn new<N, S>(
        name: N,
        sort: S,
    ) -> Self
    where
        N: Into<Name>,
        S: Into<Name>,
    {
        Self {
            name: name.into(),
            sort: sort.into(),
        }
    }
}

/// A multi-out **arity** as the bridge diagram `A ←s— J —π→ I —t→ B`
/// (proposal §4.2).
///
/// `inputs` are the input ports `A`; `outputs` are the output ports `B`;
/// `factors[i]` is how many factors monomial `i` has (the `J —π→ I` fibers);
/// `source[j]` is `s : J → A` (which input each factor reads); `dest[i]` is
/// `t : I → B` (which output port each monomial feeds). Aggregating several
/// monomials into one output port is the Σ-layer's monoid-gated combine.
///
/// # Contract
/// - requires: the maps compose — every `source[j] < inputs.len()`, every
///   `dest[i] < outputs.len()`, and `factors.len() == dest.len()` (one factor
///   count per monomial, which equals the number of monomials `|I|`). These are
///   host-checked by [`crate::check_desc`]; the constructor stores them as
///   given so an ill-formed arity is *representable and rejected*, not
///   unconstructible (proposal §8's `desc-arity-mismatch` golden).
/// - ensures: preserves the four sets and three maps exactly.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "the bridge diagram is exactly its four finite sets and three maps (V4); the elaborator builds it and the well-formedness pass validates composition"
)]
pub struct BridgeArity
{
    /// `A` — the input ports, named.
    pub inputs: Box<[SortRef]>,
    /// `π : J → I` fibers, given as the factor count of each monomial `i ∈ I`
    /// (`factors.len() == |I|`).
    pub factors: Box<[u32]>,
    /// `s : J → A` — which input each of the `Σ factors` factors reads.
    pub source: Box<[u32]>,
    /// `t : I → B` — which output port each monomial feeds (`dest.len() ==
    /// |I|`).
    pub dest: Box<[u32]>,
    /// `B` — the output ports, named.
    pub outputs: Box<[SortRef]>,
}

impl BridgeArity
{
    /// A bridge arity from its four sets and three maps.
    #[inline]
    #[must_use]
    pub fn new<In, Fa, So, De, Ou>(
        inputs: In,
        factors: Fa,
        source: So,
        dest: De,
        outputs: Ou,
    ) -> Self
    where
        In: Into<Box<[SortRef]>>,
        Fa: Into<Box<[u32]>>,
        So: Into<Box<[u32]>>,
        De: Into<Box<[u32]>>,
        Ou: Into<Box<[SortRef]>>,
    {
        Self {
            inputs: inputs.into(),
            factors: factors.into(),
            source: source.into(),
            dest: dest.into(),
            outputs: outputs.into(),
        }
    }

    /// A **single-output** arity whose one monomial is the product of the given
    /// input ports (the common `op f(a, b, …) -> R` Π-layer shape, ADR-54
    /// §3.3): `|I| = 1`, `|B| = 1`, `factors = [|A|]`, `source = [0, 1, …]`,
    /// `dest = [0]`.
    ///
    /// # Contract
    /// - ensures: the resulting arity is well-formed (composition holds) for
    ///   any input/output ports.
    #[inline]
    #[must_use]
    pub fn single_output<In>(
        inputs: In,
        output: SortRef,
    ) -> Self
    where
        In: Into<Box<[SortRef]>>,
    {
        let inputs = inputs.into();
        let factor_count = u32::try_from(inputs.len()).unwrap_or(u32::MAX);
        let source: Box<[u32]> = (0 .. factor_count).collect();
        Self {
            inputs,
            factors: Box::from([factor_count]),
            source,
            dest: Box::from([0u32]),
            outputs: Box::from([output]),
        }
    }

    /// The number of monomials `|I|` (equivalently `factors.len()`).
    #[inline]
    #[must_use]
    pub fn monomials(&self) -> MonomialCount
    {
        self.factors.len().into()
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn single_output_builds_a_composing_arity()
    {
        let arity = BridgeArity::single_output(
            [SortRef::new("m", "NatDiv"), SortRef::new("n", "NatDiv")],
            SortRef::new("q", "NatDiv"),
        );
        assert_eq!(MonomialCount::from(1), arity.monomials(), "one monomial");
        assert_eq!(&[2u32], &*arity.factors, "the monomial has two factors");
        assert_eq!(
            &[0u32, 1u32],
            &*arity.source,
            "factors read inputs in order"
        );
        assert_eq!(&[0u32], &*arity.dest, "the monomial feeds the sole output");
        assert_eq!(1, arity.outputs.len(), "one output port");
    }
}
