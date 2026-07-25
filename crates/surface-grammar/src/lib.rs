//! Checked precedence-bounded grammar core for gandr.
//!
//! This crate owns the representation shared by generated surface grammar
//! tables and graph-walk adaptation code. It deliberately contains no
//! parser-specific declaration language: clients provide constant [`Rule`]
//! values over a validated [`PrecDag`], and [`Pbg::build`] performs the
//! cross-rule checks.

extern crate alloc;

mod check;
mod highlight;
mod model;
mod mold;
mod parity;
mod surface;
mod walk;

pub use check::validate_assumption_3;
pub use check::validate_operator_form;
pub use check::validate_unique_tiles;
pub use gandr_surface_syntax::MoldId;
pub use gandr_theory_graphs::Assoc;
pub use gandr_theory_graphs::Bound;
pub use gandr_theory_graphs::Dir;
pub use gandr_theory_graphs::End;
pub use gandr_theory_graphs::Prec;
pub use gandr_theory_graphs::PrecCycle;
pub use gandr_theory_graphs::PrecDag;
pub use gandr_theory_graphs::PrecIndex;
pub use gandr_theory_graphs::PrecSpec;
pub use gandr_theory_graphs::PrecSpecError;
pub use gandr_theory_graphs::SeenKeyVerdict;
pub use gandr_theory_graphs::Walk;
pub use gandr_theory_graphs::WalkBuildError;
pub use highlight::highlight;
pub use highlight::role_of;
pub use model::Adaptation;
pub use model::AdaptationReason;
pub use model::CandidateCount;
pub use model::Fixity;
pub use model::MoldCount;
pub use model::OperatorDecl;
pub use model::Pbg;
pub use model::PbgError;
pub use model::PrecName;
pub use model::PrecPresence;
pub use model::PrecTable;
pub use model::Provenance;
pub use model::Regex;
pub use model::Rule;
pub use model::RuleName;
pub use model::Sort;
pub use model::SortName;
pub use model::SurfaceForm;
pub use model::Sym;
pub use model::Tile;
pub use model::TileLabel;
pub use mold::MoldDef;
pub use mold::RCtxId;
pub use mold::RCtxStep;
pub use mold::StepSym;
pub use parity::NamedKind;
pub use parity::NamedKindEntry;
pub use parity::NamedKindRealization;
pub use parity::named_kind_parity;
pub use parity::named_kind_realization;
pub use surface::PBG_ONLY_KINDS;
pub use surface::TREE_SITTER_NAMED_KINDS;
pub use surface::built_in;
pub use surface::built_in_prec_table;
pub use walk::Comparison;
pub use walk::ComparisonRow;
pub use walk::GrammarNonterminal;
pub use walk::GrammarTile;
pub use walk::GrammarWalkSym;
pub use walk::MAX_WALK_CHAIN_LEN;
pub use walk::comparison_table;
pub use walk::reachable_molds;
pub use walk::seen_key_verdict;
pub use walk::walk_index;
