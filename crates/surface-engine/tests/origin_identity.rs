//! CST merkle-identity acceptance (the predecessor design record): the origin
//! snapshot's `cst_hash` column is reproducible across runs *and* processes —
//! the property the freed tree-sitter node address (a heap pointer) provably
//! lacked — and the structural CST diff the pipeline consumes leaves every
//! unedited item's root matched.
//!
//! The cross-process check re-invokes this test binary in "probe" mode (the
//! `gandr-surface-syntax` subprocess-probe pattern): the child emits the
//! snapshot framed by unique markers and the parent compares it byte-for-byte
//! to its own.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]
#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::panic,
        clippy::unwrap_used,
        reason = "the standard test-allow set keeps the merkle-identity acceptance test readable (docs/workflow/rust.md)"
    )
)]

/// CST merkle-identity acceptance tests.
#[cfg(test)]
mod tests
{
    use alloc::collections::BTreeSet;
    use std::process::Command;

    use gandr_surface_engine::lower::lower_source_total;
    use gandr_surface_engine::synnode::SynTree;
    use gandr_surface_syntax::Material;
    use gandr_surface_syntax::NodeId;

    /// Set in the child process to make [`origin_snapshot_probe`] emit.
    const PROBE_ENV: &str = "GANDR_SURFACE_ENGINE_ORIGIN_PROBE";
    /// Frame markers around the emitted snapshot in the child's stdout.
    const SNAPSHOT_BEGIN: &str = "<<<ORIGIN_SNAPSHOT_BEGIN>>>";
    const SNAPSHOT_END: &str = "<<<ORIGIN_SNAPSHOT_END>>>";
    /// The fixed program whose provenance snapshot is compared across
    /// processes.
    const PROBE_SOURCE: &str = "def square(x: Integer) -> F Integer {\n  ret (x * x)\n}\n";
    /// The probe: in probe mode, emit the origin snapshot framed by markers so
    /// a parent process can compare it byte-for-byte. A no-op otherwise (so
    /// a normal test run passes trivially).
    #[test]
    fn origin_snapshot_probe() -> Result<(), String>
    {
        if std::env::var_os(PROBE_ENV).is_none() {
            return Ok(());
        }
        let snapshot = probe_snapshot()?;
        println!("{SNAPSHOT_BEGIN}");
        print!("{snapshot}");
        println!("{SNAPSHOT_END}");
        Ok(())
    }
    /// Acceptance (b), same process: the snapshot — now carrying the merkle
    /// `cst_hash` column — is deterministic across repeated lowerings.
    #[test]
    fn origin_snapshot_is_reproducible_across_runs() -> Result<(), String>
    {
        let first = probe_snapshot()?;
        let second = probe_snapshot()?;
        assert_eq!(
            first, second,
            "the origin snapshot is deterministic across runs"
        );
        assert!(!first.is_empty(), "the probe program records origins");
        assert!(
            first.lines().all(|line| line.contains(" #")),
            "every snapshot line carries a `#cst_hash` column: {first:?}"
        );
        Ok(())
    }

    /// The origin snapshot of [`PROBE_SOURCE`] (total lowering always
    /// succeeds).
    fn probe_snapshot() -> Result<String, String>
    {
        lower_source_total(PROBE_SOURCE.into())
            .map(|lowered| lowered.origin.snapshot())
            .map_err(|error| format!("the probe source must lower totally: {error}"))
    }
    /// Acceptance (b), the load-bearing half: the snapshot is identical across
    /// *processes*. A tree-sitter node id (a heap address) would vary with ASLR
    /// and allocation order; the merkle `cst_hash` does not.
    #[test]
    fn origin_snapshot_is_reproducible_across_processes() -> Result<(), String>
    {
        let in_process = probe_snapshot()?;
        let child_a = child_snapshot()?;
        let child_b = child_snapshot()?;
        assert_eq!(
            in_process, child_a,
            "a child process reproduces the in-process origin snapshot"
        );
        assert_eq!(child_a, child_b, "two independent child processes agree");
        assert!(
            child_a.contains(" #"),
            "the reproduced snapshot carries `cst_hash` columns"
        );
        Ok(())
    }

    /// The snapshot a fresh child process of this test binary computes for
    /// [`PROBE_SOURCE`], extracted from between the frame markers.
    fn child_snapshot() -> Result<String, String>
    {
        let exe = std::env::current_exe().map_err(|error| format!("current_exe: {error}"))?;
        let output = Command::new(exe)
            .arg("--exact")
            .arg("origin_identity::tests::origin_snapshot_probe")
            .arg("--nocapture")
            .env(PROBE_ENV, "1")
            .output()
            .map_err(|error| format!("probe spawn failed: {error}"))?;
        if !output.status.success() {
            return Err(format!("probe process exited with {}", output.status));
        }
        let stdout =
            String::from_utf8(output.stdout).map_err(|error| format!("probe stdout: {error}"))?;
        let (_, rest) = stdout
            .split_once(SNAPSHOT_BEGIN)
            .ok_or_else(|| "probe output missing the begin marker".to_owned())?;
        let rest = rest.strip_prefix('\n').unwrap_or(rest);
        let (body, _) = rest
            .split_once(SNAPSHOT_END)
            .ok_or_else(|| "probe output missing the end marker".to_owned())?;
        Ok(body.to_owned())
    }
    /// Acceptance (c): editing one statement in a multi-item program leaves
    /// every *other* item's root matched by `diff()`, while the edited item's
    /// root is unmatched on both sides.
    #[test]
    fn editing_one_item_leaves_every_other_matched() -> Result<(), String>
    {
        // A clean three-item program; only the MIDDLE item's body changes.
        let old = SynTree::parse("def a = 1;\ndef b = 2;\ndef c = 3;\n")
            .map_err(|error| format!("old parse: {error:?}"))?;
        let new = SynTree::parse("def a = 1;\ndef b = 20;\ndef c = 3;\n")
            .map_err(|error| format!("new parse: {error:?}"))?;

        let old_items = item_roots(&old)?;
        let new_items = item_roots(&new)?;
        assert_eq!(3, old_items.len(), "the old program has three items");
        assert_eq!(3, new_items.len(), "the new program has three items");

        let diff = old.diff(&new);
        let matched_old: BTreeSet<u32> = diff
            .matches()
            .iter()
            .map(|matched| matched.old_root().slot().into())
            .collect();
        let unmatched_old: BTreeSet<u32> = diff
            .unmatched_old()
            .iter()
            .map(|id| id.slot().into())
            .collect();
        let unmatched_new: BTreeSet<u32> = diff
            .unmatched_new()
            .iter()
            .map(|id| id.slot().into())
            .collect();

        let a = {
            let to_owned = old_items.first().ok_or_else(|| "item a".to_owned())?;
            core::convert::identity(to_owned)
        }
        .slot()
        .into();
        let b = {
            let found = old_items.get(1).ok_or_else(|| "item b".to_owned())?;
            core::convert::identity(found)
        }
        .slot()
        .into();
        let c = {
            let found = old_items.get(2).ok_or_else(|| "item c".to_owned())?;
            core::convert::identity(found)
        }
        .slot()
        .into();
        let b_new = {
            let found = new_items.get(1).ok_or_else(|| "item b'".to_owned())?;
            core::convert::identity(found)
        }
        .slot()
        .into();

        // The unedited items `a` and `c` are matched (their subtrees pruned).
        assert!(
            matched_old.contains(&a),
            "the unedited first item's root is matched: {matched_old:?}"
        );
        assert!(
            matched_old.contains(&c),
            "the unedited third item's root is matched: {matched_old:?}"
        );
        // The edited item's root is matched on neither side — its hash changed.
        assert!(
            !matched_old.contains(&b),
            "the edited item's old root is not matched"
        );
        assert!(
            unmatched_old.contains(&b),
            "the edited item's old root is reported unmatched: {unmatched_old:?}"
        );
        assert!(
            unmatched_new.contains(&b_new),
            "the edited item's new root is reported unmatched: {unmatched_new:?}"
        );
        Ok(())
    }

    /// The file root's significant (non-space) children — the item roots, in
    /// source order (the granularity `gandr_surface_syntax::diff` aligns on).
    fn item_roots(tree: &SynTree) -> Result<Vec<NodeId>, String>
    {
        let cst = tree.cst();
        let root = cst.root();
        let children = cst
            .children(root)
            .map_err(|error| format!("root children: {error:?}"))?;
        let mut roots = Vec::new();
        for &child in children {
            let view = cst
                .node(child)
                .map_err(|error| format!("node: {error:?}"))?;
            if view.material() != Material::Space {
                roots.push(child);
            }
        }
        Ok(roots)
    }
}
