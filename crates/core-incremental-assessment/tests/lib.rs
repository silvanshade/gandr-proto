//! The assessment's test target.
//!
//! `differential` is the correctness gate: both measured paths are compared
//! against from-scratch typing, never against each other, because a comparison
//! between two implementations of one rule cannot see a defect in the rule.
//! `floor` asserts the work counts against the workload's known structure, so a
//! measurement whose inputs never reached the code under test fails instead of
//! reporting favourable numbers. `firewall` pins the mechanism the engine
//! path's whole advantage rests on. `mediated` is the case that separates a
//! model of the adoption rule from a model of the convenient half of it.
//! `cycles` probes what the engine does with a mutual dependency. `confinement`
//! keeps the engine inside this crate. `support` carries the fixtures and the
//! generator witnesses, and `table` reports the measurement.

extern crate alloc;

#[cfg(test)]
mod confinement;
#[cfg(test)]
mod cycles;
#[cfg(test)]
mod differential;
#[cfg(test)]
mod firewall;
#[cfg(test)]
mod floor;
#[cfg(test)]
mod mediated;
#[cfg(test)]
mod support;
#[cfg(test)]
mod table;
