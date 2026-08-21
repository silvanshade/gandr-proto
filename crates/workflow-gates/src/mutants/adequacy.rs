//! Refusing a mutation campaign that tested nothing.
//!
//! A campaign can finish with every mutant unviable and still exit zero:
//! cargo-mutants reports "no mutants were viable" as a warning, and a driver
//! that reads only the exit status calls that a pass. It is not one. An
//! all-unviable campaign has measured nothing at all, and a gate that reports
//! success on it is the one outcome a gate must never produce.
//!
//! **The distinction this module exists to draw is between an outcome and an
//! infrastructure failure.** A surviving mutant is an outcome: the suite is
//! weaker than someone thought, and the campaign worked. Zero viable mutants
//! is not an outcome at all — the mutants never compiled, so nothing was
//! asked of the suite — and the same is true of a baseline that ran no tests.
//! Both are reported as infrastructure, in their own words, rather than as a
//! mutation result.

#![allow(
    unknown_lints,
    reason = "The local dylint library supplies primitive_signature, and the stable compiler does not know the name."
)]
#![allow(
    primitive_signature,
    reason = "A campaign's record arrives as text and a count of tests is a number; the wrappers below exist to name exactly those, so their own constructors and accessors take them."
)]

use alloc::string::String;

/// Why a campaign proved nothing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Inadequacy
{
    /// Mutants were generated and none of them ran.
    NothingViable
    {
        /// How many mutants the campaign generated.
        total: MutantCount,
        /// How many of them failed to build.
        unviable: MutantCount,
    },
    /// The baseline ran, and ran no tests.
    NoTestsExercised
    {
        /// What the baseline log showed instead.
        detail: BaselineDetail,
    },
    /// The campaign's own record could not be read.
    Unreadable
    {
        /// What could not be read, and why.
        detail: BaselineDetail,
    },
}

/// A number of mutants.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(super) struct MutantCount(u64);

impl core::fmt::Display for MutantCount
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        write!(f, "{}", self.0)
    }
}

/// What a campaign record said, for a message.
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub(super) struct BaselineDetail(String);

impl core::fmt::Display for BaselineDetail
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        f.write_str(&self.0)
    }
}

impl core::fmt::Display for Inadequacy
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        match *self {
            | Self::NothingViable {
                ref total,
                ref unviable,
            } => write!(
                f,
                "the campaign generated {total} mutants and ran none of them ({unviable} would \
                 not build), so it measured nothing. This is an infrastructure failure rather \
                 than a mutation result: read mutants.out/log/ for the build errors, which are \
                 the same for every mutant when the cause is the toolchain or the package scope."
            ),
            | Self::NoTestsExercised { ref detail } => write!(
                f,
                "the campaign's baseline ran no tests, so every mutant would have been reported \
                 as caught by a suite that never ran ({detail}). This is an infrastructure \
                 failure rather than a mutation result."
            ),
            | Self::Unreadable { ref detail } => write!(
                f,
                "the campaign's own record could not be read, so its adequacy cannot be \
                 established ({detail}). A campaign that cannot be checked does not pass."
            ),
        }
    }
}

/// A campaign's `outcomes.json`, as text.
#[repr(transparent)]
pub(super) struct OutcomesJson<'campaign>(&'campaign str);

impl<'campaign> OutcomesJson<'campaign>
{
    /// Names a campaign's `outcomes.json` text.
    pub(super) const fn new(text: &'campaign str) -> Self
    {
        Self(text)
    }
}

/// A campaign's baseline log, as text.
#[repr(transparent)]
pub(super) struct BaselineLog<'campaign>(&'campaign str);

impl<'campaign> BaselineLog<'campaign>
{
    /// Names a campaign's baseline log text.
    pub(super) const fn new(text: &'campaign str) -> Self
    {
        Self(text)
    }
}

/// A count of tests a baseline ran.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
struct TestCount(u64);

/// The counts a campaign's `outcomes.json` carries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Counts
{
    /// Mutants generated.
    total: u64,
    /// Mutants a test caught.
    caught: u64,
    /// Mutants no test caught.
    missed: u64,
    /// Mutants whose run exceeded the timeout.
    timeout: u64,
    /// Mutants that would not build.
    unviable: u64,
}

impl Counts
{
    /// Mutants that actually ran against the suite.
    const fn viable(&self) -> u64
    {
        self.caught
            .saturating_add(self.missed)
            .saturating_add(self.timeout)
    }
}

/// Reads the five counts from a campaign's `outcomes.json`.
fn counts(outcomes: &OutcomesJson<'_>) -> Result<Counts, Inadequacy>
{
    let parsed: serde_json::Value =
        serde_json::from_str(outcomes.0).map_err(|error| Inadequacy::Unreadable {
            detail: BaselineDetail(alloc::format!("outcomes.json is not JSON: {error}")),
        })?;
    let field = |name: &str| -> Result<u64, Inadequacy> {
        parsed
            .get(name)
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| Inadequacy::Unreadable {
                detail: BaselineDetail(alloc::format!("outcomes.json carries no {name}")),
            })
    };
    Ok(Counts {
        total: field("total_mutants")?,
        caught: field("caught")?,
        missed: field("missed")?,
        timeout: field("timeout")?,
        unviable: field("unviable")?,
    })
}

/// How many tests the baseline ran, from the runner's own summary line.
///
/// The number is taken from `nextest`'s summary rather than from a phase
/// duration, because a `Test` phase that ran nothing still reports a duration
/// and still succeeds. An absent summary is itself a refusal: a baseline whose
/// test count cannot be read has not been shown to have run any.
fn baseline_tests(baseline: &BaselineLog<'_>) -> Option<TestCount>
{
    for line in baseline.0.lines() {
        let Some(after) = line.split_once("Summary [")
        else {
            continue;
        };
        let Some((_, tail)) = after.1.split_once(']')
        else {
            continue;
        };
        let mut words = tail.split_whitespace();
        let Some(count) = words.next()
        else {
            continue;
        };
        // nextest renders a filtered run as `N/M tests run`; the first number
        // is what actually ran.
        let count = count.split('/').next().unwrap_or(count);
        if let Ok(parsed) = count.parse::<u64>() {
            return Some(TestCount(parsed));
        }
    }
    None
}

/// Refuses a campaign that measured nothing.
///
/// # Contract
/// - requires: `outcomes_json` is the campaign's own `outcomes.json` and
///   `baseline_log` its `log/baseline.log`.
/// - ensures: `Ok` exactly when the campaign generated no mutants at all, or
///   generated mutants of which at least one ran against a baseline that
///   exercised at least one test.
/// - provides: the one place a campaign's adequacy is decided, so the four
///   entry points cannot disagree about what counts as a pass.
/// - fails: [`Inadequacy`], which names an infrastructure failure rather than a
///   mutation outcome.
/// - panics: none.
///
/// # Errors
/// The variants of [`Inadequacy`].
///
/// # Adequacy
/// - hypothesis: L3 pointwise — a healthy campaign, an all-unviable campaign, a
///   zero-mutant campaign, a zero-test baseline, and an unreadable record each
///   have their own witness, and the all-unviable case is the one this module
///   was written for.
/// - witness: `mutants::adequacy::tests::a_campaign_that_ran_mutants_is_adequate`
/// - witness: `mutants::adequacy::tests::an_all_unviable_campaign_is_refused_as_infrastructure`
/// - witness: `mutants::adequacy::tests::a_baseline_that_ran_no_tests_is_refused`
/// - witness: `mutants::adequacy::tests::a_campaign_with_no_mutants_is_not_a_failure`
/// - witness: `mutants::adequacy::tests::an_unreadable_record_is_refused`
pub(super) fn assess(
    outcomes: &OutcomesJson<'_>,
    baseline: &BaselineLog<'_>,
) -> Result<(), Inadequacy>
{
    let counts = counts(outcomes)?;

    // A campaign that selected no mutants measured nothing and claims nothing.
    // That is the ordinary shape of a diff campaign over a change with no Rust
    // in it, and the caller decides whether selecting none was legitimate.
    if counts.total == 0 {
        return Ok(());
    }

    let Some(tests) = baseline_tests(baseline)
    else {
        return Err(Inadequacy::NoTestsExercised {
            detail: BaselineDetail(String::from(
                "the baseline log carries no test-runner summary at all",
            )),
        });
    };
    if tests == TestCount(0) {
        return Err(Inadequacy::NoTestsExercised {
            detail: BaselineDetail(String::from(
                "the baseline's own summary reports 0 tests run",
            )),
        });
    }

    if counts.viable() == 0 {
        return Err(Inadequacy::NothingViable {
            total: MutantCount(counts.total),
            unviable: MutantCount(counts.unviable),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests
{
    use super::*;

    /// The four counts a fixture record carries.
    #[derive(Clone, Copy)]
    struct Fixture
    {
        /// Mutants generated.
        total: u64,
        /// Mutants a test caught.
        caught: u64,
        /// Mutants no test caught.
        missed: u64,
        /// Mutants that would not build.
        unviable: u64,
    }

    /// A campaign record with the given counts.
    fn record(fixture: Fixture) -> String
    {
        let Fixture {
            total,
            caught,
            missed,
            unviable,
        } = fixture;
        alloc::format!(
            r#"{{"total_mutants":{total},"caught":{caught},"missed":{missed},"timeout":0,"unviable":{unviable},"success":true}}"#
        )
    }

    /// A baseline log reporting a run of `count` tests.
    fn baseline(count: TestCount) -> String
    {
        let count = count.0;
        alloc::format!("*** baseline\n     Summary [   6.7s] {count} tests run: {count} passed\n")
    }

    /// A campaign that ran mutants against a real suite passes.
    #[test]
    fn a_campaign_that_ran_mutants_is_adequate()
    {
        assert_eq!(
            assess(
                &OutcomesJson::new(&record(Fixture {
                    total: 10,
                    caught: 8,
                    missed: 1,
                    unviable: 1
                })),
                &BaselineLog::new(&baseline(TestCount(141)))
            ),
            Ok(())
        );
    }

    /// The defect this module exists for: every mutant unviable, exit zero.
    ///
    /// The message must name infrastructure rather than a mutation result,
    /// because the two send a reader to different places.
    #[test]
    fn an_all_unviable_campaign_is_refused_as_infrastructure()
    {
        let refused = assess(
            &OutcomesJson::new(&record(Fixture {
                total: 93,
                caught: 0,
                missed: 0,
                unviable: 93,
            })),
            &BaselineLog::new(&baseline(TestCount(141))),
        )
        .expect_err("an all-unviable campaign measured nothing");
        assert_eq!(refused, Inadequacy::NothingViable {
            total: MutantCount(93),
            unviable: MutantCount(93),
        });
        let rendered = refused.to_string();
        assert!(rendered.contains("measured nothing"), "{rendered}");
        assert!(
            rendered.contains("infrastructure failure rather than a mutation result"),
            "{rendered}"
        );
        assert!(rendered.contains("93"), "{rendered}");
    }

    /// A baseline that ran no tests is refused before the counts are read.
    ///
    /// Every mutant would be reported as caught by a suite that never ran,
    /// which is a green campaign proving nothing.
    #[test]
    fn a_baseline_that_ran_no_tests_is_refused()
    {
        let refused = assess(
            &OutcomesJson::new(&record(Fixture {
                total: 10,
                caught: 10,
                missed: 0,
                unviable: 0,
            })),
            &BaselineLog::new(&baseline(TestCount(0))),
        )
        .expect_err("a baseline that ran no tests measured nothing");
        assert!(matches!(refused, Inadequacy::NoTestsExercised { .. }));
        assert!(
            refused
                .to_string()
                .contains("caught by a suite that never ran"),
            "{refused}"
        );

        // A log with no summary at all is the same refusal: a test count that
        // cannot be read has not been shown to be nonzero.
        let absent = assess(
            &OutcomesJson::new(&record(Fixture {
                total: 10,
                caught: 10,
                missed: 0,
                unviable: 0,
            })),
            &BaselineLog::new("*** baseline\n*** result: Success\n"),
        )
        .expect_err("an unreadable test count is not evidence of tests");
        assert!(matches!(absent, Inadequacy::NoTestsExercised { .. }));
    }

    /// Selecting no mutants is a legitimate outcome, not a failure.
    #[test]
    fn a_campaign_with_no_mutants_is_not_a_failure()
    {
        assert_eq!(
            assess(
                &OutcomesJson::new(&record(Fixture {
                    total: 0,
                    caught: 0,
                    missed: 0,
                    unviable: 0
                })),
                &BaselineLog::new("")
            ),
            Ok(())
        );
    }

    /// A record that cannot be read is refused rather than assumed adequate.
    #[test]
    fn an_unreadable_record_is_refused()
    {
        assert!(matches!(
            assess(
                &OutcomesJson::new("not json"),
                &BaselineLog::new(&baseline(TestCount(1)))
            ),
            Err(Inadequacy::Unreadable { .. })
        ));
        assert!(matches!(
            assess(
                &OutcomesJson::new(r#"{"caught":1}"#),
                &BaselineLog::new(&baseline(TestCount(1)))
            ),
            Err(Inadequacy::Unreadable { .. })
        ));
    }

    /// A filtered run's summary reports `N/M`, and `N` is what ran.
    #[test]
    fn a_filtered_baseline_summary_reads_the_number_that_ran()
    {
        let log = "     Summary [   0.2s] 1866/2923 tests run: 1865 passed, 1 failed\n";
        assert_eq!(
            baseline_tests(&BaselineLog::new(log)),
            Some(TestCount(1866))
        );
        assert_eq!(
            baseline_tests(&BaselineLog::new("     Summary [   0.2s] 0 tests run")),
            Some(TestCount(0))
        );
        assert_eq!(baseline_tests(&BaselineLog::new("nothing here")), None);
    }
}
