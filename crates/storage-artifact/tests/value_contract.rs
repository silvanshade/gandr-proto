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
    use gandr_storage_artifact::value::chunk::chunk_body;
    use gandr_storage_artifact::value::chunk::frame_chunk;
    use gandr_storage_artifact::value::chunk::verify_chunk_image;
    use gandr_storage_artifact::value::index_base::ChildIndexBase;
    use gandr_storage_chunker::TokenCount;
    use gandr_storage_chunker::TypedChunkerParams;
    use gandr_storage_prolly_trees::BlockStore as _;
    use gandr_storage_prolly_trees::InMemoryBlockStore;
    use gandr_storage_prolly_trees::ProllyTree;
    use gandr_storage_prolly_trees::RecordKey;
    use gandr_storage_prolly_trees::RecordRef;
    use gandr_storage_prolly_trees::RecordValue;
    use gandr_storage_prolly_trees::TreeParams;

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

    /// A count of fixture leaves.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct LeafCount(usize);

    /// The number of leaves a fixture carries.
    fn leaf_count(value: &Fixture) -> LeafCount
    {
        let mut pending = alloc::vec![value];
        let mut leaves = 0_usize;
        while let Some(node) = pending.pop() {
            match *node {
                | Fixture::Leaf { .. } => leaves = leaves.saturating_add(1_usize),
                | Fixture::Node {
                    ref left,
                    ref right,
                } => {
                    pending.push(left.as_ref());
                    pending.push(right.as_ref());
                },
            }
        }
        return LeafCount(leaves);
    }

    /// Replaces the leftmost leaf's word, which is an edit early in the stream.
    ///
    /// Written as a descent with an explicit spine rather than by recursion,
    /// so it needs no termination argument: walk down the left edge collecting
    /// the right siblings, edit the leaf, then rebuild upward.
    fn edit_first_leaf(value: &Fixture) -> Fixture
    {
        let mut siblings = alloc::vec::Vec::new();
        let mut cursor = value;
        let word = loop {
            match *cursor {
                | Fixture::Leaf { word } => break word,
                | Fixture::Node {
                    ref left,
                    ref right,
                } => {
                    siblings.push(right.clone());
                    cursor = left.as_ref();
                },
            }
        };
        let mut rebuilt = Fixture::Leaf {
            word: word.wrapping_add(0xFFFF_u64),
        };
        while let Some(right) = siblings.pop() {
            rebuilt = Fixture::Node {
                left: alloc::boxed::Box::new(rebuilt),
                right,
            };
        }
        return rebuilt;
    }

    /// The first child record in a chunk body, as a pointer.
    fn first_child_record(body: ChunkBody<'_>) -> Option<ContentPtr>
    {
        let body: &[u8] = body.into();
        let mut cursor = 0_usize;
        while let Some(&kind) = body.get(cursor) {
            let advance = match kind {
                | 0x01_u8 => 2_usize,
                | 0x02_u8 => 9_usize,
                | 0x05_u8 => 1_usize,
                | 0x04_u8 => {
                    let digest = body
                        .get(cursor.saturating_add(1_usize) .. cursor.saturating_add(33_usize))?;
                    let offset = body
                        .get(cursor.saturating_add(33_usize) .. cursor.saturating_add(37_usize))?;
                    let digest = ChunkDigest::try_from(digest).ok()?;
                    let offset: [u8; 4] = offset.try_into().ok()?;
                    return Some(ContentPtr::new(digest, u32::from_be_bytes(offset).into()));
                },
                | 0x03_u8 => {
                    let len =
                        body.get(cursor.saturating_add(1_usize) .. cursor.saturating_add(9_usize))?;
                    let len: [u8; 8] = len.try_into().ok()?;
                    9_usize.saturating_add(usize::try_from(u64::from_be_bytes(len)).ok()?)
                },
                | _ => return None,
            };
            cursor = cursor.saturating_add(advance);
        }
        return None;
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
    fn a_prolly_node_image_is_refused_as_a_chunk()
    {
        // A real encoded prolly node, offered under its own BLAKE3. Without
        // the magic inside the hashed preimage this is a perfectly well-formed
        // byte string that a chunk validator would accept and a value decoder
        // would then misread.
        let key: [u8; 8] = 0_u64.to_be_bytes();
        let value: [u8; 3] = [1_u8, 2_u8, 3_u8];
        let record = RecordRef::new(
            RecordKey::from(key.as_slice()),
            RecordValue::from(value.as_slice()),
        );
        let mut blocks = InMemoryBlockStore::new();
        let tree = ProllyTree::build(&[record], TreeParams::default(), &mut blocks)
            .expect("the tree builds");
        let node = blocks
            .load(tree.root_node_hash())
            .expect("the root node is stored");
        let node_bytes: &[u8] = node.bytes().into();

        let claimed = ChunkDigest::from(*blake3::hash(node_bytes).as_bytes());
        let refusal = verify_chunk_image(StoredChunkRef::new(claimed, node_bytes.into()))
            .expect_err("a prolly node is not a chunk");

        // The refusal must be about THE MAGIC, not merely about the frame.
        // Checking only that some rejection happened proves nothing here: a
        // node image also fails the version and length checks, so a reader
        // with no domain separation at all still refuses it -- for a reason
        // that would not hold for a node image whose bytes happened to parse.
        // Naming the field is what makes this witness separate.
        let ValueError::MalformedChunk { context } = refusal
        else {
            panic!("the refusal is about the frame rather than the digest: {refusal:?}");
        };
        assert!(
            context.contains("magic"),
            "the refusal names the domain magic rather than a downstream field: {context}"
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
    fn an_interior_pointer_derefs_to_its_own_subtree()
    {
        // Commit a value large enough to cut, then read a child record out of
        // the root chunk's own body and follow it. Under the wrong reading of
        // an offset -- an index into the whole value rather than into the
        // chunk -- a root round trip still passes, because the root offset is
        // zero either way. This is where the two readings separate.
        let value = balanced(Depth(5_u32), Seed(3_u64));
        let mut store = InMemoryChunkStore::new();
        let root = cam_commit(&mut store, &profile(), ChildIndexBase::Absolute, &value)
            .expect("the fixture commits");

        let chunk = store.load(root.digest()).expect("the root chunk is stored");
        let body = chunk_body(chunk).expect("the root chunk verifies");
        let interior = first_child_record(body).expect("the root body carries a child record");

        let subtree: Fixture = cam_deref(&store, interior).expect("the interior pointer derefs");
        assert_ne!(
            subtree, value,
            "an interior pointer names a subtree, never the whole value"
        );
        assert!(
            leaf_count(&subtree) < leaf_count(&value),
            "the subtree is strictly smaller than the value that contains it"
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
    /// **Cut from this rung by lead ruling and left named rather than
    /// removed.** The bound is an expectation over the rolling hash, so what
    /// confirms or refutes it is a measured *distribution* over a corpus of
    /// edits, not a single edit — which is a corpus harness rather than a
    /// test. The rung's other exit, the format-adoption ruling, is what
    /// another lane waits on, so this is the piece that was spent.
    ///
    /// What it needs when it returns: a corpus of depth-varied edits, one
    /// `LocalityMeasurement` recorded per edit, and the distribution read
    /// against `expected_chunk_bound`. The same run yields the
    /// structural-sharing numbers, so it lands once and serves both.
    #[test]
    #[ignore = "gandr-8tou.4: cut from the rung; the locality distribution is a corpus harness"]
    fn measured_chunk_counts_sit_inside_the_locality_bound()
    {
        // Deliberately empty. An ignored test with no body is a placeholder a
        // reader can see; a deleted test is a claim nobody knows was dropped.
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
    fn an_early_edit_moves_only_its_own_chunk_under_chunk_local_bases()
    {
        // The evaluation this rung owes, answered by what the encoding is
        // rather than by choosing between two representations.
        //
        // A chunk-local index base is REFUSED, because this token stream has no
        // child indices to re-base: children are nested in place between their
        // parent's open and close records. Accepting the mode would let a
        // manifest claim a representation that does not exist.
        let value = balanced(Depth(5_u32), Seed(3_u64));
        let mut refusing = InMemoryChunkStore::new();
        let refusal = cam_commit(
            &mut refusing,
            &profile(),
            ChildIndexBase::ChunkLocal,
            &value,
        )
        .expect_err("a chunk-local base is not representable here");
        assert!(
            matches!(refusal, ValueError::UnsupportedIndexBase),
            "the refusal names the representation rather than failing late: {refusal:?}"
        );

        // And the property chunk-local bases were proposed to RECOVER already
        // holds: an edit early in the value leaves the chunks it does not touch
        // byte-identical, so committing the edited value into the same store
        // adds far fewer chunks than it contains.
        let mut store = InMemoryChunkStore::new();
        let _first = cam_commit(&mut store, &profile(), ChildIndexBase::Absolute, &value)
            .expect("the original commits");
        let before = usize::from(store.chunk_count());

        let edited = edit_first_leaf(&value);
        let _second = cam_commit(&mut store, &profile(), ChildIndexBase::Absolute, &edited)
            .expect("the edited value commits");
        let after = usize::from(store.chunk_count());
        let added = after.saturating_sub(before);

        assert!(
            added < before,
            "an early edit must reuse most chunks: {before} held, {added} added"
        );
    }
}
