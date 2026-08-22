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
    use gandr_storage_artifact::CanonicalU64;
    use gandr_storage_artifact::ValueError;
    use gandr_storage_artifact::value::CanonicalValue;
    use gandr_storage_artifact::value::ChunkBody;
    use gandr_storage_artifact::value::ChunkDigest;
    use gandr_storage_artifact::value::ChunkStore as _;
    use gandr_storage_artifact::value::ConstructorTag;
    use gandr_storage_artifact::value::ContentPtr;
    use gandr_storage_artifact::value::InMemoryChunkStore;
    use gandr_storage_artifact::value::StoredChunkRef;
    use gandr_storage_artifact::value::TokenReader;
    use gandr_storage_artifact::value::TokenSink;
    use gandr_storage_artifact::value::cam_commit;
    use gandr_storage_artifact::value::cam_deref;
    use gandr_storage_artifact::value::chunk::VALUE_CHUNK_MAGIC;
    use gandr_storage_artifact::value::chunk::frame_chunk;
    use gandr_storage_artifact::value::chunk::verify_chunk_image;
    use gandr_storage_artifact::value::index_base::ChildIndexBase;
    use gandr_storage_chunker::TokenCount;
    use gandr_storage_chunker::TypedChunkerParams;

    /// A fixture value: a binary tree of canonical words.
    ///
    /// Small enough to reason about by hand and recursive enough to have
    /// depth, siblings, and shareable subtrees — the three things every claim
    /// in this file needs. Its codec is the implementor's deliverable; the
    /// shape is fixed here so the claims can be stated against it.
    #[derive(Clone, Debug, Eq, PartialEq)]
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

    /// The fixture's leaf tag.
    const LEAF_TAG: u8 = 0x11;
    /// The fixture's node tag.
    const NODE_TAG: u8 = 0x12;

    impl CanonicalValue for Fixture
    {
        /// # Termination
        /// - reason: each descent emits one constructor of a strictly smaller
        ///   subtree.
        /// - measure: the constructors of the value not yet emitted.
        /// - boundedness: a fixture is a finite tree by construction.
        /// - input recursion: none.
        fn emit_tokens<Sink>(
            &self,
            sink: &mut Sink,
        ) -> Result<(), ValueError>
        where
            Sink: TokenSink + ?Sized,
        {
            match *self {
                | Self::Leaf { word } => {
                    sink.open(ConstructorTag::from(LEAF_TAG))?;
                    sink.word(CanonicalU64::from(word))?;
                },
                | Self::Node {
                    ref left,
                    ref right,
                } => {
                    sink.open(ConstructorTag::from(NODE_TAG))?;
                    left.emit_tokens(sink)?;
                    right.emit_tokens(sink)?;
                },
            }
            return sink.close();
        }

        /// # Termination
        /// - reason: each descent consumes at least the open record of a
        ///   strictly shorter remaining stream.
        /// - measure: the unread records of the chunk DAG.
        /// - boundedness: a chunk body is finite and a descent never re-reads a
        ///   record, so the stream is exhausted after finitely many steps.
        /// - input recursion: none.
        fn decode_tokens(reader: &mut TokenReader<'_>) -> Result<Self, ValueError>
        {
            let tag = u8::from(reader.read_tag()?);
            let value = match tag {
                | LEAF_TAG => Self::Leaf {
                    word: u64::from(reader.read_word()?),
                },
                | NODE_TAG => {
                    let left = alloc::boxed::Box::new(Self::decode_tokens(reader)?);
                    let right = alloc::boxed::Box::new(Self::decode_tokens(reader)?);
                    Self::Node { left, right }
                },
                | _ => {
                    return Err(ValueError::UnexpectedToken {
                        expected: "a fixture tag",
                        found: "an unknown constructor",
                        position: u32::from(reader.position()),
                    });
                },
            };
            reader.read_close()?;
            return Ok(value);
        }
    }

    /// The depth of a generated fixture tree.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug)]
    struct Depth(u32);

    /// The seed a generated fixture's leaf words are derived from.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug)]
    struct Seed(u64);

    /// Builds a balanced fixture of the given depth with distinct leaf words.
    ///
    /// Built bottom-up rather than by recursion: the leaves are laid out
    /// left to right and folded in pairs until one node remains. Distinct leaf
    /// words matter — a fixture whose leaves repeat would share subtrees it
    /// was not meant to share, and the sharing claim would then be measuring
    /// the fixture rather than the traversal.
    fn balanced(
        depth: Depth,
        seed: Seed,
    ) -> Fixture
    {
        let width = 1_u64 << u64::from(depth.0.min(16_u32));
        let base = seed.0.wrapping_mul(width);
        let mut level: alloc::vec::Vec<Fixture> = (0_u64 .. width)
            .map(|index| Fixture::Leaf {
                word: base.wrapping_add(index),
            })
            .collect();
        while level.len() > 1_usize {
            let mut next = alloc::vec::Vec::with_capacity(level.len().div_ceil(2_usize));
            let mut pairs = level.into_iter();
            while let Some(left) = pairs.next() {
                let Some(right) = pairs.next()
                else {
                    next.push(left);
                    break;
                };
                next.push(Fixture::Node {
                    left: alloc::boxed::Box::new(left),
                    right: alloc::boxed::Box::new(right),
                });
            }
            level = next;
        }
        return level.pop().unwrap_or(Fixture::Leaf { word: seed.0 });
    }

    /// The committed profile the contract suite runs at.
    fn profile() -> TypedChunkerParams
    {
        return TypedChunkerParams::new(
            core::num::NonZeroU32::new(4_u32).expect("kappa is nonzero"),
            core::num::NonZeroU32::new(64_u32).expect("the cap is nonzero"),
        );
    }

    /// Committing a value and dereferencing its root recovers an equal value.
    ///
    /// The round trip is the plane's whole reason to exist: a content pointer
    /// means nothing if what comes back is only similar.
    #[test]
    fn a_committed_value_derefs_back_equal()
    {
        let value = balanced(Depth(3_u32), Seed(1_u64));
        let mut store = InMemoryChunkStore::new();
        let root = cam_commit(&mut store, &profile(), ChildIndexBase::Absolute, &value)
            .expect("the fixture commits");
        let back: Fixture = cam_deref(&store, root).expect("the root derefs");
        assert_eq!(
            back, value,
            "the round trip is an equality, not a resemblance"
        );
    }

    /// The same value under the same committed constants commits to the same
    /// pointer.
    ///
    /// Determinism is what makes two independently produced commits of the
    /// same value share storage rather than duplicate it.
    #[test]
    fn the_same_value_commits_to_the_same_pointer()
    {
        let value = balanced(Depth(3_u32), Seed(1_u64));
        let mut left = InMemoryChunkStore::new();
        let mut right = InMemoryChunkStore::new();
        let first = cam_commit(&mut left, &profile(), ChildIndexBase::Absolute, &value)
            .expect("the first commit");
        let second = cam_commit(&mut right, &profile(), ChildIndexBase::Absolute, &value)
            .expect("the second commit");
        assert_eq!(
            first, second,
            "two independent commits of one value under one profile agree"
        );
    }

    /// Two values sharing a subtree share the chunks that subtree was cut into.
    ///
    /// **Separating witness.** It kills the wrong representation in which a
    /// child is inlined rather than referenced: under inlining the round trip
    /// still succeeds and the pointers are still deterministic, and the only
    /// observable difference is that the store holds two copies of the shared
    /// subtree instead of one. Counting chunks is what separates them.
    #[test]
    fn a_shared_subtree_is_stored_once()
    {
        // Two values sharing a large subtree. Committed into one store they
        // must add fewer chunks than the same two values committed apart --
        // inlining shares nothing and still round-trips, which is what this
        // separates.
        let shared = balanced(Depth(4_u32), Seed(7_u64));
        let left = Fixture::Node {
            left: alloc::boxed::Box::new(shared.clone()),
            right: alloc::boxed::Box::new(Fixture::Leaf { word: 101_u64 }),
        };
        let right = Fixture::Node {
            left: alloc::boxed::Box::new(shared),
            right: alloc::boxed::Box::new(Fixture::Leaf { word: 202_u64 }),
        };

        let mut together = InMemoryChunkStore::new();
        let _l = cam_commit(&mut together, &profile(), ChildIndexBase::Absolute, &left)
            .expect("the left commits");
        let _r = cam_commit(&mut together, &profile(), ChildIndexBase::Absolute, &right)
            .expect("the right commits");

        let mut alone_left = InMemoryChunkStore::new();
        let _a = cam_commit(&mut alone_left, &profile(), ChildIndexBase::Absolute, &left)
            .expect("the left commits alone");
        let mut alone_right = InMemoryChunkStore::new();
        let _b = cam_commit(
            &mut alone_right,
            &profile(),
            ChildIndexBase::Absolute,
            &right,
        )
        .expect("the right commits alone");

        let shared_count = usize::from(together.chunk_count());
        let apart_count =
            usize::from(alone_left.chunk_count()) + usize::from(alone_right.chunk_count());
        assert!(
            shared_count < apart_count,
            "one store holding both values must hold fewer chunks ({shared_count}) than two stores holding one each ({apart_count})"
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
    fn a_word_is_never_read_as_a_tag()
    {
        // A body whose first record is a word carrying a tag-valued low byte.
        // A reader that coerced it would return a constructor.
        let mut body = alloc::vec::Vec::new();
        // The word's LEADING byte is the fixture's leaf tag, so a reader that
        // coerced the record would read a well-formed constructor and fail
        // later and elsewhere, with a truncation rather than a wrong-kind
        // refusal. Putting the tag anywhere else would let a coercing reader
        // fail for the right reason by accident.
        body.push(0x02_u8);
        body.extend_from_slice(&0x1100_0000_0000_0000_u64.to_be_bytes());
        let (digest, image) =
            frame_chunk(ChunkBody::from(body.as_slice()), TokenCount::from(1_u64))
                .expect("the body frames");
        let mut store = InMemoryChunkStore::new();
        store
            .insert(StoredChunkRef::new(digest, image.as_ref().into()))
            .expect("the chunk stores");
        let refusal = cam_deref::<Fixture>(&store, ContentPtr::new(digest, 0_u32.into()))
            .expect_err("a word is not a tag");
        assert!(
            matches!(refusal, ValueError::UnexpectedToken { .. }),
            "the refusal names both kinds rather than reporting a truncation: {refusal:?}"
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
    fn a_value_larger_than_one_chunk_is_read_across_seams()
    {
        // Deep enough to force cuts at the committed kappa. A traversal that
        // never cut would put the whole value in one chunk, round-trip
        // perfectly, and satisfy every other claim in this file.
        let value = balanced(Depth(5_u32), Seed(3_u64));
        let mut store = InMemoryChunkStore::new();
        let root = cam_commit(&mut store, &profile(), ChildIndexBase::Absolute, &value)
            .expect("the deep fixture commits");
        assert!(
            usize::from(store.chunk_count()) > 1_usize,
            "the traversal cut at least once"
        );
        let back: Fixture = cam_deref(&store, root).expect("the root derefs across seams");
        assert_eq!(back, value, "crossing a seam is invisible to the decoder");
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
