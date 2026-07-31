# spec-reboot

Your task is to establish a new discipline for the gandr language specification documentation.

This task is focused on the specs, not the entire gandr documentation.

The broader docs directory is at "./docs" and the gandr language specs are at "./docs/gandr/spec".

There is current one spec file:

* "proposal-metatheory-consolidated.md"

Move this file to a temporary location out of the tree since it will be replaced.

What I suggest doing is creating three separate tracks for the specs, which will roughly correspond to top-level docs and probably subdirectories which you will file individual sub-topics under:

* `implementation`
* `metatheory`
* `proof-engineering`

The `implementation` track should contain anything related to the actual implementation of the language in terms of the Rust code.

The `metatheory` track should contain anything related to the gandr language metatheory which is _both_ mathematical in nature _and_ specific to gandr's semantic model (either the CwF or the L-machine or the type system, broadly speaking).
It's not enough for it to just be mathematical in nature since a lot of that will fall under `proof-engineering`.
Sometimes the line is a little blurred, so this requires judgement.
For example, discussion of how we define categories is not specific enough for this part.
But discussion of the details about a particular category (category of circuit algebras, or maybe the category PROF in relation to our CwF) might be.

The `proof-engineering` track should contain anything specific to Agda programming, proof-organization, discussion of the organization or definition specific mathematical structures (which are not gandr specific).
Examples would be how we organize our structures by defining everything over infinity graphs and how we project setoids out of groupoids rather than building on top of the other, etc.

You will be processing a lot of documents to reconstruct these specs, many of them will not be well organized, and some are not specifications but rather records of research sessions or analysis of various artifacts or brainstorming sessions or other things of that nature.
So you will need to discern what goes into each of these tracks as you go.

I also want this process of building the new specs tracks to be auditable, since it will likely take multiple passes, and will involve some adversarial verification after things are in place to help mitigate any mistakes, either from the migration or from the original sources (which are not entirely reliable, so keep that in mind too).

I suggest you keep a log as you go as well:

* record each file you read
* record each idea or section you import from that document, along with a reason why, and a confidence rating for how applicable it appears to be to current gandr

As you ingest the content from these docs and synthesize the new specs, do the following:

Create and maintain a hayagriva bibliography.
The main source for input will be the ~/Documents/research/managed/library.bib.
All of the referenced papers from there live in ~/Documents/research/managed/*.pdf.
Don't reference that `library.bib` directly, copy the entries used in the gandr specs into the local file.
Note we will be using typst for the broader documentation eventually.

Note that these documents will be written in Obsidian markdown style.
Use inline typst for the math, according to the conventions of <https://github.com/azyarashi/obsidian-typst-mate>.
All mathematical notation should use typst, inline or block style.
Make extensive use of typst for readability, including commutative diagrams, which should be written for the latest version of fletcher.
For diagram style I suggest using "Monoidal context theory.pdf" and "Fundamentals of compositional rewriting theory.pdf" and "Higher-order circuits.pdf" and "Operads of wiring diagrams.pdf" for reference where appropriate.
Obsidian will handle the dependencies automatically.

The "./docs" dir is an Obsidian vault, so create each markdown doc with that in mind, use the appropriate headers and references and link structure expected by Obsidian.

Disambiguate all of the single letter references in the existing docs, and all of the numbered references.
Make sure everything actually links somewhere concrete.
This is currently a huge problem in trying to keep track of which is actually being referenced by what else.
Instead, use meaningful references with actual identifiable names.
And furthermore

Then, proceed by reading these two (in order):

1. ~/Development/gandr-worktrees/rabbit-metatheory/docs/gandr/spec/proposal-metatheory-consolidated.md
2. ~/Development/notes-rabbit-calculus/circuit-algebra/DOCTRINE-DELTA.md

The `proposal-metatheory-consolidated.md` is the closest we have to a big-picture baseline, but it is incomplete: it is missing historical details we need to recover, it is missing recent developments, and parts of it are now inaccurate too.

The `DOCTRINE-DELTA.md` is the most important single source delta over `proposal-metatheory-consolidated.md`; it clarifies most of the remaining work to generalize to circuit algebras, to clarifiy the relationship with the virtual doctrines, and build the rest of the machinery needed for univalence.

## references system

* ~/Development/notes-rabbit-calculus/reference-system-design.md
* ~/Development/notes-rabbit-calculus/research-map.md

### remaining (load-bearing)

1. ~/Development/notes-rabbit-calculus/circuit-algebra/
2. ~/Development/notes-rabbit-calculus/analytic-ladder/

### remaining (historical)

* ~/Development/notes-rabbit-calculus/agda-metatheory-brief.md
* ~/Development/notes-rabbit-calculus/arc-plan.md
* ~/Development/notes-rabbit-calculus/sessions/
* ~/Development/notes-rabbit-calculus/refactor/
* ~/Development/notes-rabbit-calculus/adversary/
* ~/Development/wyrd/docs/gandr/spec/
* ~/Development/internal-univalence/docs/spec/
* ~/Development/wyrd-notes/
