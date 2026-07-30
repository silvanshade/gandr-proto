// Miri cannot run Criterion benchmarks (wall-clock measurement is meaningless
// under interpretation), so `main` is a `cfg(miri)` stub below; that stub
// leaves every bench-only item unused when Miri's driver compiles this target.
#![cfg_attr(
    miri,
    expect(
        unused_imports,
        dead_code,
        reason = "Criterion harness is stubbed out under Miri, so bench-only items are unused"
    )
)]
// `harness = false` makes this a separate compilation target where `cfg(test)`
// is false, so the clippy.toml in-tests overrides cannot reach it; relax
// `expect_used` here at file scope. `expect` (not `allow`) is required because
// `allow_attributes` is denied; clippy fulfils the expectation on the
// `.expect()` sites below.
#![expect(
    clippy::expect_used,
    reason = "benchmark bodies assume setup succeeds (docs/workflow/rust.md)"
)]

use core::hint::black_box;

use criterion::BatchSize;
use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use gandr_theory_orders::OrderMaintenance;

/// The element count exercised by the throughput benchmarks.
const ELEMENT_COUNT: BenchElementCount = BenchElementCount(1_000);

/// Semantic element count used by the throughput benchmarks.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct BenchElementCount(u64);

/// Semantic payload value used by the throughput benchmarks.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct BenchElementValue(u64);

/// Builds an [`OrderMaintenance`] of `count` sequential `push_back` elements.
fn built(count: BenchElementCount) -> OrderMaintenance<BenchElementValue>
{
    let mut order: OrderMaintenance<BenchElementValue> =
        OrderMaintenance::new().expect("structure id allocation succeeds during benchmarks");
    for raw_value in 0 .. count.0 {
        order
            .push_back(BenchElementValue(raw_value))
            .expect("push_back succeeds at full universe");
    }
    order
}

/// Benchmarks the core operations: sequential append, comparison, and the
/// adversarial same-spot insertion that maximizes relabeling.
fn order_benches(criterion: &mut Criterion)
{
    criterion.bench_function("push_back_sequential", |bencher| {
        bencher.iter(|| black_box(built(black_box(ELEMENT_COUNT))));
    });

    criterion.bench_function("cmp_endpoints", |bencher| {
        let order = built(ELEMENT_COUNT);
        let first = order.first().expect("the built order is non-empty");
        let last = order.last().expect("the built order is non-empty");
        bencher.iter(|| black_box(order.cmp(black_box(first), black_box(last))));
    });

    criterion.bench_function("insert_after_same_spot", |bencher| {
        bencher.iter_batched(
            || {
                let mut order: OrderMaintenance<BenchElementValue> = OrderMaintenance::new()
                    .expect("structure id allocation succeeds during benchmarks");
                let anchor = order
                    .push_back(BenchElementValue(0))
                    .expect("push_back succeeds at full universe");
                order
                    .push_back(BenchElementValue(1))
                    .expect("push_back succeeds at full universe");
                (order, anchor)
            },
            |(mut order, anchor)| {
                for raw_value in 0 .. ELEMENT_COUNT.0 {
                    black_box(
                        order
                            .insert_after(anchor, BenchElementValue(raw_value))
                            .expect("insert_after succeeds under relabel"),
                    );
                }
                black_box(order)
            },
            BatchSize::SmallInput,
        );
    });
}

#[cfg(not(miri))]
criterion_group!(benches, order_benches);
#[cfg(not(miri))]
criterion_main!(benches);

// Under Miri the Criterion harness is inert; provide a trivial entry point.
#[cfg(miri)]
fn main()
{
}
