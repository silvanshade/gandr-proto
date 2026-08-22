//! The value plane's contract suite: the commit–deref round trip, the sharing
//! and locality observables, the index-base evaluation, and the wrong-kind
//! separating witnesses.
//!
//! # Every test states its claim, and the separating ones say what they kill
//!
//! A test that passes when the representation is right and also passes when it
//! is subtly wrong is not evidence. The tests marked as separating witnesses
//! below are written to **fail** against a specific plausible wrong
//! representation, and each names the wrong inhabitant it separates from. The
//! module map in `gandr_storage_artifact::value` carries the same table from
//! the type side.

#[cfg(test)]
mod tests
{
    use gandr_storage_artifact::ValueError;
    use gandr_storage_artifact::value::CanonicalValue;
    use gandr_storage_artifact::value::ChunkBody;
    use gandr_storage_artifact::value::ChunkDigest;
    use gandr_storage_artifact::value::StoredChunkRef;
    use gandr_storage_artifact::value::TokenReader;
    use gandr_storage_artifact::value::TokenSink;
    use gandr_storage_artifact::value::chunk::VALUE_CHUNK_MAGIC;
    use gandr_storage_artifact::value::chunk::frame_chunk;
    use gandr_storage_artifact::value::chunk::verify_chunk_image;
    use gandr_storage_chunker::TokenCount;

    /// A fixture value: a binary tree of canonical words.
    ///
    /// Small enough to reason about by hand and recursive enough to have
    /// depth, siblings, and shareable subtrees — the three things every claim
    /// in this file needs. Its codec is the implementor's deliverable; the
    /// shape is fixed here so the claims can be stated against it.
    #[derive(Clone, Debug, Eq, PartialEq)]
    #[expect(
        dead_code,
        reason = "gandr-8tou.4 scaffold: the fixture is constructed by the test bodies the implementor writes"
    )]
    enum Fixture
    {
        /// A leaf carrying one canonical word.
        Leaf
        {
            /// The leaf's payload word.
            word: u64,
        },
        /// A node carrying two children.
        Node
        {
            /// The left child.
            left: alloc::boxed::Box<Self>,
            /// The right child.
            right: alloc::boxed::Box<Self>,
        },
    }

    impl CanonicalValue for Fixture
    {
        #[expect(
            clippy::todo,
            reason = "gandr-8tou.4 scaffold: the fixture codec is the implementor deliverable"
        )]
        fn emit_tokens<Sink>(
            &self,
            sink: &mut Sink,
        ) -> Result<(), ValueError>
        where
            Sink: TokenSink + ?Sized,
        {
            todo!("preorder-emit {self:?} into {sink:p}");
        }

        #[expect(
            clippy::todo,
            reason = "gandr-8tou.4 scaffold: the fixture codec is the implementor deliverable"
        )]
        fn decode_tokens(reader: &mut TokenReader<'_>) -> Result<Self, ValueError>
        {
            todo!("decode one fixture from {reader:?}");
        }
    }

    /// Committing a value and dereferencing its root recovers an equal value.
    ///
    /// The round trip is the plane's whole reason to exist: a content pointer
    /// means nothing if what comes back is only similar.
    #[test]
    #[ignore = "gandr-8tou.4: awaits the value-plane bodies"]
    #[expect(
        clippy::todo,
        reason = "gandr-8tou.4 scaffold: the test body is the implementor deliverable"
    )]
    fn a_committed_value_derefs_back_equal()
    {
        todo!("cam_commit a Fixture, cam_deref its root, assert_eq the two");
    }

    /// The same value under the same committed constants commits to the same
    /// pointer.
    ///
    /// Determinism is what makes two independently produced commits of the
    /// same value share storage rather than duplicate it.
    #[test]
    #[ignore = "gandr-8tou.4: awaits the value-plane bodies"]
    #[expect(
        clippy::todo,
        reason = "gandr-8tou.4 scaffold: the test body is the implementor deliverable"
    )]
    fn the_same_value_commits_to_the_same_pointer()
    {
        todo!("commit one Fixture twice into two fresh stores, assert the pointers are equal");
    }

    /// Two values sharing a subtree share the chunks that subtree was cut into.
    ///
    /// **Separating witness.** It kills the wrong representation in which a
    /// child is inlined rather than referenced: under inlining the round trip
    /// still succeeds and the pointers are still deterministic, and the only
    /// observable difference is that the store holds two copies of the shared
    /// subtree instead of one. Counting chunks is what separates them.
    #[test]
    #[ignore = "gandr-8tou.4: awaits the value-plane bodies"]
    #[expect(
        clippy::todo,
        reason = "gandr-8tou.4 scaffold: the test body is the implementor deliverable"
    )]
    fn a_shared_subtree_is_stored_once()
    {
        todo!(
            "build two Fixtures sharing a large subtree, commit both into one store, \
             assert chunk_count is strictly less than committing them into separate stores"
        );
    }

    /// A prolly-node image cannot enter the chunk store under any digest.
    ///
    /// **Separating witness.** It kills the wrong representation in which the
    /// two planes share a digest space without domain separation: without the
    /// chunk magic inside the hashed preimage, a node image is a perfectly
    /// well-formed byte string that a chunk validator would accept and a value
    /// decoder would then misread.
    #[test]
    #[ignore = "gandr-8tou.4: awaits the value-plane bodies"]
    #[expect(
        clippy::todo,
        reason = "gandr-8tou.4 scaffold: the test body is the implementor deliverable"
    )]
    fn a_prolly_node_image_is_refused_as_a_chunk()
    {
        todo!(
            "build a real encoded prolly node, offer it to verify_chunk_image under its own \
             BLAKE3, assert ValueError::MalformedChunk naming the magic"
        );
    }

    /// A word token standing where a constructor tag belongs is refused by
    /// name.
    ///
    /// **Separating witness.** It kills the wrong-kind inhabitant directly: a
    /// canonical word whose low byte happens to be a valid tag byte must not
    /// be coerced into a tag. The rejection names both kinds, so a future
    /// reader can tell a wrong-kind refusal from a truncation.
    #[test]
    #[ignore = "gandr-8tou.4: awaits the value-plane bodies"]
    #[expect(
        clippy::todo,
        reason = "gandr-8tou.4 scaffold: the test body is the implementor deliverable"
    )]
    fn a_word_is_never_read_as_a_tag()
    {
        todo!(
            "frame a body whose next token is a word with a tag-valued low byte, \
             assert read_tag returns ValueError::UnexpectedToken naming word and tag"
        );
    }

    /// A pointer into a non-root chunk derefs to that subtree, not the whole
    /// value.
    ///
    /// **Separating witness.** It kills the wrong reading of
    /// `ContentPtr::offset` as an index into the whole value: under that
    /// reading a root-pointer round trip still passes, because the root offset
    /// is zero either way.
    #[test]
    #[ignore = "gandr-8tou.4: awaits the value-plane bodies"]
    #[expect(
        clippy::todo,
        reason = "gandr-8tou.4 scaffold: the test body is the implementor deliverable"
    )]
    fn an_interior_pointer_derefs_to_its_own_subtree()
    {
        todo!(
            "commit a Fixture, take a pointer at an interior chunk boundary, \
             assert cam_deref returns exactly that subtree"
        );
    }

    /// A deref that should cross a chunk seam actually crosses one.
    ///
    /// **Separating witness.** It kills the case where the traversal never cut
    /// at all — one chunk holding the whole value round-trips perfectly, is
    /// deterministic, and satisfies every other claim in this file, while
    /// providing none of the sharing or locality the plane exists for. A
    /// round-trip test cannot see the difference; the reader's seam depth can.
    #[test]
    #[ignore = "gandr-8tou.4: awaits the value-plane bodies"]
    #[expect(
        clippy::todo,
        reason = "gandr-8tou.4 scaffold: the test body is the implementor deliverable"
    )]
    fn a_value_larger_than_one_chunk_is_read_across_seams()
    {
        todo!(
            "commit a Fixture deep enough to force cuts at the committed kappa, \
             assert the store holds more than one chunk and that the reader \
             reports a nonzero seam depth during the deref"
        );
    }

    /// A depth-`d` edit touches a chunk count inside the theory's bound.
    ///
    /// The bound is an expectation, so the claim is about the measured
    /// distribution over a corpus of edits rather than about any single edit.
    #[test]
    #[ignore = "gandr-8tou.4: awaits the value-plane bodies"]
    #[expect(
        clippy::todo,
        reason = "gandr-8tou.4 scaffold: the test body is the implementor deliverable"
    )]
    fn measured_chunk_counts_sit_inside_the_locality_bound()
    {
        todo!(
            "run a corpus of depth-varied edits, record LocalityMeasurement for each, \
             assert the distribution sits inside expected_chunk_bound"
        );
    }

    /// A chunk digest over a fixed body matches its committed golden.
    ///
    /// **Separating witness, and the transport fence's evidence.** The
    /// certificate layer's in-process labels are FNV-1a taken through
    /// `core::hash::Hash`, whose integer writers encode native-endian and at
    /// the target's pointer width — so the same input digests differently on
    /// two targets, and such a digest is a comparable value and never a
    /// portable address. A golden kills any implementation that routed a chunk
    /// digest through that path: the golden holds on the machine that minted it
    /// and fails everywhere else, which is exactly the failure mode a stored
    /// content pointer must not be able to have. Framing by hand over
    /// big-endian fixed widths is what makes the golden hold on every target,
    /// and the golden is what proves the framing was actually used.
    #[test]
    fn a_chunk_digest_matches_its_committed_golden()
    {
        // THE GOLDEN. Computed once against this exact body and pasted here as
        // a literal. It is not derived at runtime and must never become so: a
        // golden the test recomputes from the implementation is the
        // implementation agreeing with itself, which is precisely what this
        // assertion exists to rule out. An implementation that routed the
        // digest through a native-endian hasher would satisfy every other test
        // in this file on the machine that ran it, and fail only on a machine
        // of different endianness or pointer width -- in production and never
        // in the suite.
        const GOLDEN: [u8; 32] = [
            0x1C, 0x28, 0x4F, 0xDF, 0x9C, 0x17, 0x25, 0x3E, 0xE5, 0xFE, 0xA4, 0xA1, 0x55, 0x40,
            0x75, 0xCB, 0xF9, 0xC1, 0xB1, 0x52, 0x9A, 0xFA, 0xBD, 0xA2, 0x22, 0xCC, 0xEB, 0xEF,
            0xEA, 0x13, 0x2B, 0x21,
        ];

        // One open record carrying tag 0x2a, one word record carrying 7, one
        // close record. Three token records, twelve body bytes.
        let body: [u8; 12] = [
            0x01, 0x2A, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x05,
        ];
        let (digest, image) =
            frame_chunk(ChunkBody::from(body.as_slice()), TokenCount::from(3_u64))
                .expect("a twelve-byte body frames");

        // The frame, read back field by field rather than trusted. Doing this
        // beside the golden is what tells a future reader which of the two
        // moved when they disagree.
        let bytes: &[u8] = image.as_ref();
        let magic_len = VALUE_CHUNK_MAGIC.len();
        assert_eq!(
            bytes.get(.. magic_len),
            Some(VALUE_CHUNK_MAGIC),
            "the image opens with the value-chunk magic"
        );
        assert_eq!(
            bytes.len(),
            magic_len + 0x12_usize + body.len(),
            "magic, a u16 version, two u64 fields, then the body"
        );

        assert_eq!(
            digest,
            ChunkDigest::from(GOLDEN),
            "the framed image hashes to its committed golden"
        );

        // The frame the golden pins is the one the verifier accepts.
        verify_chunk_image(StoredChunkRef::new(digest, image.as_ref().into()))
            .expect("the framed image verifies against its own digest");
    }

    /// Chunk-local child index bases keep downstream chunks unchanged under an
    /// early edit, and absolute indices do not.
    ///
    /// **Separating witness, and the rung's format-adoption evaluation.** Both
    /// representations round-trip perfectly and both are deterministic; the
    /// only observable difference is which chunks moved after an early
    /// insertion. This test is what turns the representation question into a
    /// measurement rather than an argument.
    #[test]
    #[ignore = "gandr-8tou.4: awaits the value-plane bodies"]
    #[expect(
        clippy::todo,
        reason = "gandr-8tou.4 scaffold: the test body is the implementor deliverable"
    )]
    fn an_early_edit_moves_only_its_own_chunk_under_chunk_local_bases()
    {
        todo!(
            "commit a Fixture under both ChildIndexBase modes, insert a constructor early, \
             recommit, and record IndexBaseVerdict from the two chunk-change counts"
        );
    }
}
