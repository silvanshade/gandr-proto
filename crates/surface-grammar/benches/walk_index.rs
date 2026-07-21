// Miri cannot run Criterion benchmarks (wall-clock measurement is meaningless
// under interpretation), so `main` is a `cfg(miri)` stub below; that stub
// leaves every bench-only item unused when Miri's driver compiles this target
// (the Miri-ignore workspace convention).
#![cfg_attr(
    miri,
    expect(
        unused_imports,
        dead_code,
        reason = "Criterion harness is stubbed out under Miri, so bench-only items are unused"
    )
)]
// `harness = false` makes this a separate compilation target where `cfg(test)`
// is false, so the crate-level `#![cfg_attr(test, allow(...))]` override cannot
// reach it; relax `expect_used` here at file scope. `expect` (not `allow`) is
// required because `allow_attributes` is denied; clippy fulfils the expectation
// on the `.expect()` site below.
#![expect(
    clippy::expect_used,
    reason = "the benchmark asserts the built-in grammar assembles (docs/workflow/rust.md)"
)]

use core::hint::black_box;

use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use gandr_surface_grammar::built_in;
use gandr_surface_grammar::walk_index;

/// Benchmark the full index build: `Pbg` + mold/`RCtx` tables + `WalkIndex`.
///
/// This is the index-build budget: the whole pipeline from the
/// built-in surface rules through the checked PBG (mold and interned-context
/// tables) to the generative walk index, end to end in every sample, targeting
/// `<= 50 ms`.
fn bench_index_build(c: &mut Criterion)
{
    c.bench_function("full_index_build", |b| {
        b.iter(|| {
            let pbg = built_in().expect("built-in grammar assembles");
            let index = walk_index(&pbg).expect("walk index builds");
            return black_box(index.fingerprint());
        });
    });
}

#[cfg(not(miri))]
criterion_group!(benches, bench_index_build);
#[cfg(not(miri))]
criterion_main!(benches);
#[cfg(miri)]
fn main()
{
}
