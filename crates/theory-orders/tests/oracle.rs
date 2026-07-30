//! Reference-oracle property tests for [`OrderMaintenance`] over the public
//! API: a random sequence of insertions and removals is replayed against a
//! naive `Vec` model, and after every operation the structure's iteration
//! order, length, and handle sequence must match the model — and, at the end,
//! O(1) comparison must agree with list order for every pair.
//!
//! The narrow-universe relabel and capacity paths are exercised by the crate's
//! in-module unit tests (which can reach the test-only narrow constructor);
//! this file complements them with a full-universe relabel stress test and the
//! broad randomized cross-check.

/// Reference-oracle and stress properties for the order-maintenance structure.
#[cfg(test)]
mod tests
{
    use gandr_theory_orders::OrderMaintenance;
    use gandr_theory_orders::Pos;
    use proptest::prelude::*;

    /// Semantic payload observed by the oracle value extractor.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct OracleValue(u64);

    /// One edit applied to both the structure and the reference model. Index
    /// fields are reduced modulo the current length when applied, so they
    /// always name a live element of a non-empty structure.
    #[derive(Clone, Debug)]
    enum Op
    {
        /// Append at the end.
        PushBack(OracleValue),
        /// Prepend at the front.
        PushFront(OracleValue),
        /// Insert after the element at the (reduced) index.
        InsertAfter(usize, OracleValue),
        /// Insert before the element at the (reduced) index.
        InsertBefore(usize, OracleValue),
        /// Remove the element at the (reduced) index.
        Remove(usize),
    }

    /// A generator for a single [`Op`].
    fn op_strategy() -> impl Strategy<Value = Op>
    {
        prop_oneof![
            any::<u64>().prop_map(|value| Op::PushBack(OracleValue(value))),
            any::<u64>().prop_map(|value| Op::PushFront(OracleValue(value))),
            (any::<usize>(), any::<u64>())
                .prop_map(|(index, value)| Op::InsertAfter(index, OracleValue(value))),
            (any::<usize>(), any::<u64>())
                .prop_map(|(index, value)| Op::InsertBefore(index, OracleValue(value))),
            any::<usize>().prop_map(Op::Remove),
        ]
    }

    /// Replays `ops` against the structure and a `Vec` model, checking
    /// agreement after each step and pairwise comparison at the end.
    fn replay(ops: &[Op])
    {
        let mut order: OrderMaintenance<OracleValue> =
            OrderMaintenance::new().expect("structure id allocation succeeds in oracle test");
        let mut model: Vec<OracleValue> = Vec::new();
        let mut handles: Vec<Pos> = Vec::new();
        for op in ops {
            match *op {
                | Op::PushBack(value) => {
                    let pos = order
                        .push_back(value)
                        .expect("push_back succeeds at full universe");
                    model.push(value);
                    handles.push(pos);
                },
                | Op::PushFront(value) => {
                    let pos = order
                        .push_front(value)
                        .expect("push_front succeeds at full universe");
                    model.insert(0, value);
                    handles.insert(0, pos);
                },
                | Op::InsertAfter(raw_index, value) => {
                    if model.is_empty() {
                        continue;
                    }
                    let index = raw_index
                        .checked_rem(model.len())
                        .expect("the model is non-empty");
                    let anchor = *handles.get(index).expect("index is in range");
                    let pos = order
                        .insert_after(anchor, value)
                        .expect("insert_after succeeds");
                    let after = index.checked_add(1).expect("index + 1 fits");
                    model.insert(after, value);
                    handles.insert(after, pos);
                },
                | Op::InsertBefore(raw_index, value) => {
                    if model.is_empty() {
                        continue;
                    }
                    let index = raw_index
                        .checked_rem(model.len())
                        .expect("the model is non-empty");
                    let anchor = *handles.get(index).expect("index is in range");
                    let pos = order
                        .insert_before(anchor, value)
                        .expect("insert_before succeeds");
                    model.insert(index, value);
                    handles.insert(index, pos);
                },
                | Op::Remove(raw_index) => {
                    if model.is_empty() {
                        continue;
                    }
                    let index = raw_index
                        .checked_rem(model.len())
                        .expect("the model is non-empty");
                    let target = *handles.get(index).expect("index is in range");
                    let expected = *model.get(index).expect("index is in range");
                    assert_eq!(
                        order.remove(target),
                        Some(expected),
                        "remove returns the modelled payload"
                    );
                    model.remove(index);
                    handles.remove(index);
                },
            }
            assert_eq!(values(&order), model, "iteration order matches the model");
            assert_eq!(
                usize::from(order.len()),
                model.len(),
                "length matches the model"
            );
            assert_eq!(
                handles_of(&order),
                handles,
                "the handle sequence matches the model"
            );
        }
        let final_handles = handles_of(&order);
        for (left_rank, &left) in final_handles.iter().enumerate() {
            for (right_rank, &right) in final_handles.iter().enumerate() {
                assert_eq!(
                    order.cmp(left, right),
                    Some(left_rank.cmp(&right_rank)),
                    "O(1) comparison agrees with list order"
                );
            }
        }
    }
    #[test]
    fn relabel_stress_at_full_universe()
    {
        // Inserting many elements after one fixed anchor halves the local label
        // gap each time, so the full (2^62) universe still relabels after a few
        // dozen insertions — this drives the relabel path without the test-only
        // narrow constructor.
        let mut order: OrderMaintenance<OracleValue> =
            OrderMaintenance::new().expect("structure id allocation succeeds in oracle test");
        let anchor = order.push_back(OracleValue(0)).expect("push_back succeeds");
        order
            .push_back(OracleValue(u64::MAX))
            .expect("push_back succeeds");
        let count: u64 = 300;
        for value in 1 ..= count {
            order
                .insert_after(anchor, OracleValue(value))
                .expect("insert_after succeeds under relabel");
        }
        let capacity = usize::try_from(count)
            .expect("count fits in usize")
            .checked_add(2)
            .expect("count + 2 fits in usize");
        let mut expected: Vec<OracleValue> = Vec::with_capacity(capacity);
        expected.push(OracleValue(0));
        expected.extend((1 ..= count).rev().map(OracleValue));
        expected.push(OracleValue(u64::MAX));
        assert_eq!(
            values(&order),
            expected,
            "full-universe relabeling preserves order"
        );

        let final_handles = handles_of(&order);
        for (left_rank, &left) in final_handles.iter().enumerate() {
            for (right_rank, &right) in final_handles.iter().enumerate() {
                assert_eq!(
                    order.cmp(left, right),
                    Some(left_rank.cmp(&right_rank)),
                    "comparison stays consistent through relabeling"
                );
            }
        }
    }
    /// The payloads of `order` in list order.
    fn values(order: &OrderMaintenance<OracleValue>) -> Vec<OracleValue>
    {
        order.iter().map(|(_, &value)| value).collect()
    }

    /// The handles of `order` in list order.
    fn handles_of(order: &OrderMaintenance<OracleValue>) -> Vec<Pos>
    {
        order.iter().map(|(pos, _value)| pos).collect()
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        /// A random op sequence keeps the structure in lock-step with the model.
        #[test]
        fn matches_reference_model(ops in prop::collection::vec(op_strategy(), 0..80))
        {
            replay(&ops);
        }
    }
}
