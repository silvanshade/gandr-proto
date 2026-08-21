//! The slice-one test plan for the document algebra, the builder, and the
//! arenas. **The tests themselves are slice one's to write; this file states
//! what each one must establish.**
//!
//! Every entry below is a claim, not a description of an implementation. A test
//! that passes without separating the claim from its negation has not
//! discharged its entry.
//!
//! # Exact node semantics
//!
//! - `empty_emits_nothing_and_moves_no_column`
//! - `text_emits_at_the_current_column`
//! - `text_rejects_a_carriage_return_a_line_feed_and_a_tab`
//! - `concat_resolves_the_right_at_the_left_ending_column`
//! - `nest_raises_indentation_by_a_checked_amount`
//! - `nest_reports_overflow_rather_than_wrapping_the_indentation`
//! - `align_sets_indentation_to_the_current_column`
//! - `flatten_turns_a_line_into_one_space`
//! - `flatten_leaves_a_hard_line_alone`
//! - `flatten_leaves_verbatim_bytes_and_indentation_alone`
//! - `group_is_choice_of_the_unflattened_form_then_the_flattened_form`
//!
//! # Verbatim fragments and endings
//!
//! One test per shape, each proving the exact bytes, the first fragment's
//! incremental width, the absolute widths of middle fragments, the ending
//! column, and the stored fragment count:
//!
//! - `verbatim_with_no_ending_extends_the_incoming_column`
//! - `verbatim_with_a_trailing_ending_stores_an_empty_final_fragment`
//! - `verbatim_with_several_middle_lines_stores_absolute_widths`
//! - `verbatim_preserves_line_feed_endings_byte_for_byte`
//! - `verbatim_preserves_carriage_return_line_feed_endings_byte_for_byte`
//! - `verbatim_preserves_a_mixed_ending_sequence_byte_for_byte`
//! - `verbatim_rejects_a_bare_carriage_return`
//!
//! # Identity and arena sealing
//!
//! - `a_handle_from_another_arena_is_refused_before_lookup`
//! - `an_out_of_range_handle_is_refused`
//! - `identities_are_dense_insertion_ordinals_that_never_move`
//! - `a_builder_with_a_node_ceiling_below_three_refuses_immediately`
//! - `an_exhausted_arena_key_counter_is_reported_rather_than_reused`
//!
//! # Flatten finalization and the interner
//!
//! - `flattening_is_idempotent`
//! - `finalization_appends_at_most_one_image_per_node`
//! - `finalization_reuses_the_original_identity_when_nothing_changes`
//! - `finalization_growth_is_linear_in_the_node_count`
//! - `finalization_is_deterministic_across_runs`
//! - `a_ceiling_reached_during_finalization_yields_no_partial_arena`
//!
//! # Build accounting
//!
//! - `a_second_edge_to_a_shared_handle_charges_no_new_node`
//! - `a_second_edge_to_a_shared_handle_charges_no_new_text_bytes`
//! - `every_finalization_visit_edge_and_probe_charges_a_build_step`
//! - `each_build_ceiling_refuses_exactly_at_its_boundary`
//! - `a_refused_charge_leaves_the_counter_unchanged`
//! - `build_usage_is_monotone_across_a_whole_document`
//!
//! # Totality
//!
//! - `deep_left_spine_construction_uses_a_heap_work_stack`
//! - `deep_right_spine_construction_uses_a_heap_work_stack`
//! - `a_wide_shared_graph_finalizes_without_native_stack_growth`
//! - `every_checked_arithmetic_site_reports_its_own_operation`
//! - `an_allocation_failure_reports_its_own_store`
//!
//! # Properties
//!
//! Property tests over generated documents, run through `proptest`:
//!
//! - concatenation is associative up to the rendered node sequence;
//! - the empty node is a left and a right unit of concatenation;
//! - finalization is idempotent;
//! - stored node count never exceeds the sum of constructor calls and distinct
//!   flattened images;
//! - no constructor sequence within its ceilings ever returns an error other
//!   than one this crate names.
