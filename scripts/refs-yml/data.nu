# Curated citation-derived fields for each register REFERENCE row.
# IDs + locators are NOT here (they come verbatim from the register extraction);
# these are TITLE/AUTHOR/DATE/VENUE/TYPE transcribed from the register Citation
# column. Cross-reference placeholders J-22 and J-25d are intentionally absent.
#
# Fields: id type title [author] [date] [genre] [publisher] [org] [venue vtype] [note]

export def curated-entries []: nothing -> list {
    [
    # A. CBPV foundations, effects, control, the sequent kernel
    {id: "A-1a" type: "article" title: "Call-By-Push-Value: A Subsuming Paradigm" author: "Levy" date: 1999 venue: "TLCA" vtype: "proceedings"}
    {id: "A-1b" type: "book" title: "Call-By-Push-Value: A Functional/Imperative Synthesis" author: "Levy" date: 2003 publisher: "Kluwer" note: "Semantic Structures in Computation 2"}
    {id: "A-2" type: "thesis" title: "Call-By-Push-Value" author: "Levy, Paul Blain" date: 2001 genre: "PhD thesis" org: "Queen Mary, University of London"}
    {id: "A-3" type: "article" title: "A Mixed Linear and Non-Linear Logic" author: "Benton" date: 1994}
    {id: "A-4" type: "article" title: "Parametric Effect Monads and Semantics of Effect Systems" author: "Katsumata" date: 2014 venue: "POPL" vtype: "proceedings"}
    {id: "A-5" type: "article" title: "Syntax and Semantics of Focalisation with Relative Monads and Comonads" author: "Mangel, Melliès & Munch-Maccagnoni" date: 2026 venue: "SD" vtype: "proceedings"}
    {id: "A-6a" type: "article" title: "Notions of Computation Determine Monads" author: "Plotkin & Power" date: 2002 venue: "FoSSaCS" vtype: "proceedings"}
    {id: "A-6b" type: "article" title: "Handling Algebraic Effects" author: "Plotkin & Pretnar" date: 2013 venue: "LMCS" vtype: "periodical" note: "LMCS 9(4); ESOP 2009 ancestor: Handlers of Algebraic Effects"}
    {id: "A-7a" type: "article" title: "An Introduction to Algebraic Effects and Handlers" author: "Pretnar" date: 2015 venue: "MFPS" vtype: "proceedings" note: "tutorial"}
    {id: "A-7b" type: "article" title: "Programming with Algebraic Effects and Handlers" author: "Bauer & Pretnar" date: 2015 venue: "JLAMP" vtype: "periodical" note: "Eff"}
    {id: "A-8a" type: "article" title: "Type Directed Compilation of Row-Typed Algebraic Effects" author: "Leijen" date: 2017 venue: "POPL" vtype: "proceedings" note: "Koka"}
    {id: "A-8b" type: "article" title: "Koka: Programming with Row Polymorphic Effect Types" author: "Leijen" date: 2014 venue: "MSFP" vtype: "proceedings"}
    {id: "A-9" type: "article" title: "Do Be Do Be Do" author: "Lindley, McBride & McLaughlin" date: 2017 venue: "POPL" vtype: "proceedings" note: "Frank"}
    {id: "A-10" type: "article" title: "On the expressive power of user-defined effects: effect handlers, monadic reflection, delimited control" author: "Forster, Kammar, Lindley & Pretnar" date: 2017}
    {id: "A-11a" type: "article" title: "Continuation Passing Style for Effect Handlers" author: "Hillerström, Lindley, Atkey & Sivaramakrishnan" date: 2017 venue: "FSCD" vtype: "proceedings"}
    {id: "A-11b" type: "article" title: "Effect Handlers via Generalised Continuations" author: "Hillerström, Lindley & Atkey" date: 2020 venue: "JFP" vtype: "periodical"}
    {id: "A-12a" type: "article" title: "Abstracting Control" author: "Danvy & Filinski" date: 1990 venue: "LFP" vtype: "proceedings" note: "answer types"}
    {id: "A-12b" type: "article" title: "The Theory and Practice of First-Class Prompts" author: "Felleisen" date: 1988 venue: "POPL" vtype: "proceedings"}
    {id: "A-12c" type: "article" title: "Subtyping Delimited Continuations" author: "Materzok & Biernacki" date: 2011 venue: "ICFP" vtype: "proceedings"}
    {id: "A-13" type: "article" title: "Representing control in the presence of one-shot continuations" author: "Bruggeman, Waddell & Dybvig" date: 1996}
    {id: "A-14" type: "article" title: "Asynchronous effects" author: "Ahman & Pretnar" date: 2021}
    {id: "A-15a" type: "article" title: "Models of a Non-Associative Composition" author: "Munch-Maccagnoni" date: 2014 venue: "FoSSaCS" vtype: "proceedings" note: "duploids"}
    {id: "A-15b" type: "article" title: "Formulae-as-Types for an Involutive Negation" author: "Munch-Maccagnoni" date: 2014 venue: "CSL-LICS" vtype: "proceedings"}
    {id: "A-16" type: "thesis" title: "Syntax and Models of a non-Associative Composition of Programs and Proofs" author: "Munch-Maccagnoni" date: 2013 genre: "PhD thesis" org: "Univ. Paris Diderot–Paris 7"}
    {id: "A-17" type: "article" title: "A theory of effects and resources: adjunction models and polarised calculi" author: "Curien, Fiore & Munch-Maccagnoni" date: 2016 venue: "POPL" vtype: "proceedings"}
    {id: "A-18" type: "article" title: "Linear effects, exceptions, and resource safety: a Curry-Howard correspondence for destructors" author: "Congard, Munch-Maccagnoni & Douence" date: 2026}
    {id: "A-19" type: "article" title: "The Duality of Computation" author: "Curien & Herbelin" date: 2000 venue: "ICFP" vtype: "proceedings" note: "λμμ̃"}
    {id: "A-20" type: "article" title: "Grokking the Sequent Calculus" author: "Binder, Tzschentke, Müller & Ostermann" date: 2024 venue: "ICFP" vtype: "proceedings" note: "art. 250"}
    {id: "A-21" type: "article" title: "Sequent Core" author: "Downen, Maurer, Ariola & Peyton Jones" date: 2016 venue: "ICFP" vtype: "proceedings"}
    {id: "A-22a" type: "thesis" title: "Reflections of Closures" author: "Sullivan" date: 2023 genre: "PhD thesis" org: "University of Oregon"}
    {id: "A-22b" type: "article" title: "Closure Conversion in Little Pieces" author: "Sullivan, Downen & Ariola" date: 2023 venue: "PPDP" vtype: "proceedings"}
    {id: "A-23" type: "article" title: "Under Control: Compositionally Correct Closure Conversion with Mutable State" author: "Mates, Perconti & Ahmed" date: 2019 venue: "PPDP" vtype: "proceedings"}
    {id: "A-24" type: "article" title: "Compiling Effect Handlers in Capability-Passing Style" author: "Schuster, Brachthäuser & Ostermann" date: 2020 venue: "ICFP" vtype: "proceedings" note: "art. 93"}
    {id: "A-25" type: "article" title: "Generalized Evidence Passing for Effect Handlers" author: "Xie & Leijen" date: 2021 venue: "ICFP" vtype: "proceedings" note: "art. 71"}
    {id: "A-26a" type: "article" title: "Handle with Care: Relational Interpretation of Algebraic Effects and Handlers" author: "Biernacki, Piróg, Polesiuk & Sieczkowski" date: 2018 venue: "POPL" vtype: "proceedings" note: "art. 8"}
    {id: "A-26b" type: "article" title: "Abstracting Algebraic Effects" author: "Biernacki, Piróg, Polesiuk & Sieczkowski" date: 2019 venue: "POPL" vtype: "proceedings" note: "art. 6"}
    {id: "A-27" type: "article" title: "A General Fine-Grained Reduction Theory for Effect Handlers" author: "Sieczkowski, Pyzik & Biernacki" date: 2023 venue: "ICFP" vtype: "proceedings" note: "art. 234"}
    {id: "A-28" type: "article" title: "\"Blaze\"" date: 2026 venue: "POPL" vtype: "proceedings" note: "relational handler compilation template"}
    {id: "A-29" type: "article" title: "Closure Conversion Is Safe for Space" author: "Appel et al."}
    {id: "A-30" type: "article" title: "Typed Closure Conversion for the Calculus of Constructions" author: "Bowman & Ahmed" date: 2018}
    {id: "A-31" type: "article" title: "S4 modal sequent calculus as intermediate logic and intermediate language" author: "Caspar & Munch-Maccagnoni" date: 2026 venue: "PEPM" vtype: "proceedings"}
    {id: "A-32" type: "article" title: "Controlling Copatterns" author: "Downen" date: 2025 note: "copatterns ≡ delimited control"}
    {id: "A-33" type: "article" title: "Rows and Capabilities as Modal Effects" author: "Tang & Lindley"}
    {id: "A-34" type: "article" title: "Type, Ability, and Effect Systems" author: "Bao & Rompf"}
    {id: "A-35" type: "article" title: "Classical Notions of Computation and the Hasegawa–Thielecke Theorem" author: "Mangel, Melliès & Munch-Maccagnoni" date: 2026 venue: "POPL" vtype: "proceedings"}
    {id: "A-36" type: "article" title: "Compiling Adjoint Natural Deduction to the Semi-Axiomatic Sequent Calculus" author: "Boyland & Pfenning" date: 2025 venue: "TLLA" vtype: "proceedings" note: "workshop abstract"}
    {id: "A-37" type: "article" title: "Normalization by Evaluation for Call-By-Push-Value and Polarized Lambda Calculus" author: "Abel & Sattler" date: 2019 venue: "PPDP" vtype: "proceedings"}
    {id: "A-38" type: "article" title: "A Calculus of Delayed Reductions" author: "van Bakel, Tye & Wu" date: 2023 venue: "PPDP" vtype: "proceedings"}
    {id: "A-39" type: "article" title: "Dependent Type Theory in Polarised Sequent Calculus" author: "Miquey, Montillet & Munch-Maccagnoni" date: 2020 venue: "TYPES" vtype: "proceedings" note: "L_dep, abstract"}
    {id: "A-40" type: "article" title: "Polarised Intermediate Representation of Lambda Calculus with Sums" author: "Munch-Maccagnoni & Scherer" date: 2015 venue: "LICS" vtype: "proceedings"}
    {id: "A-41" type: "article" title: "Resource Polymorphism" author: "Munch-Maccagnoni" date: 2018}
    {id: "A-42" type: "article" title: "Modal Effect Types" author: "Tang et al." date: 2024}

    # B. Bidirectional typing, inference, algebraic subtyping
    {id: "B-1" type: "article" title: "Local Type Inference" author: "Pierce & Turner" date: 2000 venue: "TOPLAS" vtype: "periodical"}
    {id: "B-2" type: "article" title: "Complete and Easy Bidirectional Typechecking for Higher-Rank Polymorphism" author: "Dunfield & Krishnaswami" date: 2013 venue: "ICFP" vtype: "proceedings"}
    {id: "B-3" type: "article" title: "A Mechanical Formalization of Higher-Ranked Polymorphic Type Inference" author: "Zhao, Oliveira & Schrijvers" date: 2019 venue: "ICFP" vtype: "proceedings"}
    {id: "B-4" type: "article" title: "Bidirectional Typing" author: "Dunfield & Krishnaswami" date: 2021 venue: "ACM Comput. Surveys" vtype: "periodical"}
    {id: "B-5" type: "article" title: "Polymorphism, Subtyping, and Type Inference in MLsub" author: "Dolan & Mycroft" date: 2017 venue: "POPL" vtype: "proceedings"}
    {id: "B-6" type: "article" title: "The Simple Essence of Algebraic Subtyping" author: "Parreaux" date: 2020 venue: "ICFP" vtype: "proceedings"}
    {id: "B-7" type: "article" title: "MLstruct" author: "Parreaux & Chau" date: 2022 venue: "OOPSLA" vtype: "proceedings"}
    {id: "B-8" type: "article" title: "Bidirectional Higher-Rank Polymorphism with Intersection and Union Types" author: "Jiang, Cui & Oliveira" date: 2025 venue: "POPL" vtype: "proceedings"}
    {id: "B-9" type: "thesis" title: "Algebraic Subtyping" author: "Dolan" date: 2016 genre: "PhD thesis" org: "University of Cambridge"}
    {id: "B-10" type: "thesis" title: "Type Inference, Haskell and Dependent Types" author: "Gundry" date: 2013 genre: "PhD thesis" org: "University of Strathclyde"}
    {id: "B-11" type: "article" title: "Generic Bidirectional Typing for Dependent Type Theories" author: "Felicissimo" date: 2023}
    {id: "B-12" type: "thesis" title: "Dependent Types in Haskell: Theory and Practice" author: "Eisenberg" date: 2016 genre: "PhD thesis" org: "University of Pennsylvania"}
    {id: "B-13" type: "article" title: "Filling the Gaps of Polarity: Implementing Dependent Data and Codata Types with Implicit Arguments" author: "Liesnikov et al." date: 2025}

    # C. Unions, intersections, refinements, polarity
    {id: "C-1" type: "article" title: "Tridirectional Typechecking" author: "Dunfield & Pfenning" date: 2004 venue: "POPL" vtype: "proceedings"}
    {id: "C-2" type: "article" title: "Elaborating Intersection and Union Types" author: "Dunfield" date: 2012 venue: "ICFP" vtype: "proceedings"}
    {id: "C-3" type: "article" title: "Intersection Types and Computational Effects" author: "Davies & Pfenning" date: 2000 venue: "ICFP" vtype: "proceedings"}
    {id: "C-4" type: "article" title: "Disjoint Intersection Types" author: "Oliveira, Shi & Alpuim" date: 2016 venue: "ICFP" vtype: "proceedings"}
    {id: "C-5" type: "article" title: "On the Unity of Duality" author: "Zeilberger" date: 2008 venue: "APAL" vtype: "periodical"}
    {id: "C-6" type: "article" title: "Focusing and Higher-Order Abstract Syntax" author: "Zeilberger" date: 2008 venue: "POPL" vtype: "proceedings"}
    {id: "C-7" type: "thesis" title: "The Logical Basis of Evaluation Order and Pattern-Matching" author: "Zeilberger" date: 2009 genre: "PhD thesis" org: "Carnegie Mellon University" note: "CMU-CS-09-122"}
    {id: "C-8" type: "article" title: "Focusing on Pattern Matching" author: "Krishnaswami" date: 2009 venue: "POPL" vtype: "proceedings"}

    # D. Session types, sharing, multiparty
    {id: "D-1" type: "article" title: "Language Primitives and Type Discipline for Structured Communication-Based Programming" author: "Honda, Vasconcelos & Kubo" date: 1998 venue: "ESOP" vtype: "proceedings"}
    {id: "D-2" type: "article" title: "Subtyping for Session Types in the π-Calculus" author: "Gay & Hole" date: 2005 venue: "Acta Inf." vtype: "periodical"}
    {id: "D-3a" type: "article" title: "Session Types as Intuitionistic Linear Propositions" author: "Caires & Pfenning" date: 2010 venue: "CONCUR" vtype: "proceedings"}
    {id: "D-3b" type: "article" title: "Propositions as Sessions" author: "Wadler" date: 2014 venue: "JFP" vtype: "periodical" note: "also ICFP 2012"}
    {id: "D-4" type: "article" title: "Multiparty Asynchronous Session Types" author: "Honda, Yoshida & Carbone" date: 2008 venue: "POPL" vtype: "proceedings"}
    {id: "D-5" type: "article" title: "Less Is More: Multiparty Session Types Revisited" author: "Scalas & Yoshida" date: 2019 venue: "POPL" vtype: "proceedings"}
    {id: "D-6" type: "article" title: "Manifest Sharing with Session Types" author: "Balzer & Pfenning" date: 2017 venue: "ICFP" vtype: "proceedings"}
    {id: "D-7" type: "article" title: "Manifest Deadlock-Freedom for Shared Session Types" author: "Balzer, Toninho & Pfenning" date: 2019 venue: "ESOP" vtype: "proceedings"}
    {id: "D-8" type: "article" title: "Session Coalgebras" author: "Keizer, Basold & Pérez" date: 2021 venue: "ESOP" vtype: "proceedings"}
    {id: "D-9" type: "article" title: "Statically Verified Refinements for Multiparty Protocols" author: "Zhou et al." date: 2020 venue: "OOPSLA" vtype: "proceedings"}
    {id: "D-10" type: "article" title: "A Message-Passing Interpretation of Adjoint Logic" author: "Pruiksma & Pfenning" date: 2019 venue: "PLACES" vtype: "proceedings"}
    {id: "D-11" type: "thesis" title: "Adjoint Logic with Applications" author: "Pruiksma" date: 2024 genre: "PhD thesis" org: "Carnegie Mellon University" note: "CMU-CS-24-103"}
    {id: "D-12" type: "article" title: "Formalizing π-Calculus in Guarded Cubical Agda" author: "Veltri & Vezzosi" date: 2020 venue: "CPP" vtype: "proceedings"}

    # E. Defunctionalization, abstract machines, compilation techniques
    {id: "E-1" type: "article" title: "Definitional Interpreters for Higher-Order Programming Languages" author: "Reynolds" date: 1972}
    {id: "E-2" type: "article" title: "Defunctionalization at Work" author: "Danvy & Nielsen" date: 2001 venue: "PPDP" vtype: "proceedings"}
    {id: "E-3" type: "article" title: "A Functional Correspondence between Evaluators and Abstract Machines" author: "Ager, Biernacki, Danvy & Midtgaard" date: 2003 venue: "PPDP" vtype: "proceedings"}
    {id: "E-4" type: "article" title: "From Interpreter to Compiler and Virtual Machine: A Functional Derivation" author: "Ager, Biernacki, Danvy & Midtgaard" date: 2003 note: "BRICS RS-03-14"}
    {id: "E-5" type: "article" title: "Warnings for Pattern Matching" author: "Maranget" date: 2007 venue: "JFP" vtype: "periodical"}
    {id: "E-6" type: "article" title: "Compiling Pattern Matching" author: "Augustsson" date: 1985 venue: "FPCA" vtype: "proceedings"}
    {id: "E-7" type: "book" title: "The Implementation of Functional Programming Languages" author: "Peyton Jones" date: 1987 publisher: "Prentice Hall" note: "ch. 5"}
    {id: "E-8" type: "article" title: "Calculating Correct Compilers" author: "Bahr & Hutton" date: 2015 venue: "JFP" vtype: "periodical"}
    {id: "E-9" type: "article" title: "Calculating Compilers for Concurrency" author: "Bahr & Hutton" date: 2023 venue: "ICFP" vtype: "proceedings"}
    {id: "E-10" type: "article" title: "Calculating Dependently-Typed Compilers" author: "Pickard & Hutton" date: 2021 venue: "ICFP" vtype: "proceedings" note: "Functional Pearl"}

    # F. Grades, coeffects, quantitative
    {id: "F-1" type: "article" title: "Coeffects: A Calculus of Context-Dependent Computation" author: "Petricek, Orchard & Mycroft" date: 2014 venue: "ICFP" vtype: "proceedings"}
    {id: "F-2" type: "article" title: "A Core Quantitative Coeffect Calculus" author: "Brunel, Gaboardi, Mazza & Zdancewic" date: 2014 venue: "ESOP" vtype: "proceedings"}
    {id: "F-3" type: "article" title: "Combining Effects and Coeffects via Grading" author: "Gaboardi, Katsumata, Orchard, Breuvart & Uustalu" date: 2016 venue: "ICFP" vtype: "proceedings"}
    {id: "F-4" type: "article" title: "Quantitative Program Reasoning with Graded Modal Types" author: "Orchard, Liepelt & Eades" date: 2019 venue: "ICFP" vtype: "proceedings" note: "Granule"}
    {id: "F-5" type: "article" title: "Syntax and Semantics of Quantitative Type Theory" author: "Atkey" date: 2018 venue: "LICS" vtype: "proceedings"}
    {id: "F-6" type: "article" title: "Distance Makes the Types Grow Stronger" author: "Reed & Pierce" date: 2010 venue: "ICFP" vtype: "proceedings"}
    {id: "F-7" type: "article" title: "Effects and Coeffects in Call-by-Push-Value" author: "Torczon et al." date: 2024 venue: "OOPSLA" vtype: "proceedings"}
    {id: "F-8" type: "misc" title: "locally graded categories" author: "Levy"}
    {id: "F-9" type: "misc" title: "Two-Sided Graded Type Theory, Formalized in Agda" author: "Eriksson" date: 2026 venue: "TYPES" vtype: "proceedings" note: "talk, abstract 39"}

    # G. Modal types, worlds, distribution (ML5), phasing
    {id: "G-1" type: "article" title: "A Judgmental Reconstruction of Modal Logic" author: "Pfenning & Davies" date: 2001 venue: "MSCS" vtype: "periodical"}
    {id: "G-2" type: "article" title: "A Symmetric Modal Lambda Calculus for Distributed Computing" author: "Murphy, Crary, Harper & Pfenning" date: 2004 venue: "LICS" vtype: "proceedings"}
    {id: "G-3" type: "article" title: "Type-Safe Distributed Programming with ML5" author: "Murphy, Crary & Harper" date: 2007 venue: "TGC" vtype: "proceedings"}
    {id: "G-4" type: "thesis" title: "Modal Types for Mobile Code" author: "Murphy" date: 2008 genre: "PhD thesis" org: "CMU"}
    {id: "G-5" type: "article" title: "A Modal Analysis of Staged Computation" author: "Davies & Pfenning" date: 2001 venue: "JACM" vtype: "periodical"}
    {id: "G-6" type: "article" title: "Contextual modal type theory" author: "Nanevski, Pfenning & Pientka" date: 2008 venue: "TOCL" vtype: "periodical"}
    {id: "G-7" type: "article" title: "Mœbius" author: "Jang, Gélineau, Monnier & Pientka" date: 2022 venue: "POPL" vtype: "proceedings"}
    {id: "G-8" type: "article" title: "Multimodal Dependent Type Theory" author: "Gratzer, Kavvos, Nuyts & Birkedal" date: 2020 venue: "LICS/LMCS" vtype: "proceedings" note: "2020/21"}

    # H. Module systems & Alice ML
    {id: "H-1" type: "article" title: "F-ing Modules" author: "Rossberg, Russo & Dreyer" date: 2014 venue: "JFP" vtype: "periodical"}
    {id: "H-2" type: "article" title: "1ML — Core and Modules United" author: "Rossberg" date: 2015 venue: "ICFP" vtype: "proceedings" note: "JFP 2018"}
    {id: "H-3" type: "article" title: "Fulfilling OCaml Modules with Transparency" author: "Blaudeau, Radanne & Rémy" date: 2024 note: "Mω"}
    {id: "H-4" type: "article" title: "Modular Implicits" author: "White, Bour & Yallop" date: 2014 venue: "ML Workshop" vtype: "proceedings"}
    {id: "H-5" type: "article" title: "Modular Type Classes" author: "Dreyer, Harper, Chakravarty & Keller" date: 2007 venue: "POPL" vtype: "proceedings"}
    {id: "H-6" type: "thesis" title: "Typed Open Programming" author: "Rossberg" date: 2007 genre: "PhD thesis" org: "Saarland"}
    {id: "H-7" type: "article" title: "Alice Through the Looking Glass" author: "Rossberg, Le Botlan, Tack, Brunklaus & Smolka" date: 2004 venue: "TFP" vtype: "proceedings"}

    # I. Incremental typing, structure editors, typed holes
    {id: "I-1" type: "article" title: "A Co-contextual Formulation of Type Rules …" author: "Erdweg, Bračevac, Kuci, Krebs & Mezini" date: 2015 venue: "OOPSLA" vtype: "proceedings"}
    {id: "I-2" type: "article" title: "Adapton" author: "Hammer, Phang, Hicks & Foster" date: 2014 venue: "PLDI" vtype: "proceedings"}
    {id: "I-3a" type: "article" title: "Hazelnut: A Bidirectionally Typed Structure Editor Calculus" author: "Omar, Voysey, Hilton, Aldrich & Hammer" date: 2017 venue: "POPL" vtype: "proceedings"}
    {id: "I-3b" type: "article" title: "Live Functional Programming with Typed Holes" author: "Omar, Voysey, Chugh & Hammer" date: 2019 venue: "POPL" vtype: "proceedings"}
    {id: "I-4" type: "article" title: "Total Type Error Localization and Recovery with Holes" author: "Zhao et al." date: 2024 venue: "POPL" vtype: "proceedings"}
    {id: "I-5" type: "article" title: "Incremental Bidirectional Typing via Order Maintenance" author: "Porter, Kirisame, Wei, Panchekha & Omar"}
    {id: "I-6" type: "article" title: "Pantograph: A Fluid and Typed Structure Editor" author: "Prinz, Blanchette & Lampropoulos" date: 2025 venue: "POPL" vtype: "proceedings"}
    {id: "I-7a" type: "article" title: "Syntactic Completions with Material Obligations" author: "Moon, Blinn, Porter & Omar" date: 2025 venue: "OOPSLA" vtype: "proceedings" note: "\"tylr\""}
    {id: "I-7b" type: "thesis" title: "Syntactic Completions with Material Obligations" author: "Moon" date: 2025 genre: "PhD dissertation" org: "University of Michigan" note: "title-page verified, identical title to I-7a"}
    {id: "I-8" type: "article" title: "Grove" author: "Adams et al." date: 2025 venue: "POPL" vtype: "proceedings"}
    {id: "I-9" type: "article" title: "Incremental Context-Dependent Analysis for Language-Based Editors" author: "Reps et al." date: 1983 venue: "TOPLAS" vtype: "periodical"}
    {id: "I-10" type: "article" title: "Two Simplified Algorithms for Maintaining Order in a List" author: "Bender et al." date: 2002}
    {id: "I-11" type: "repository" title: "tylr source (hazelgrove/tylr)"}
    {id: "I-12" type: "repository" title: "petgraph 0.8.3"}
    {id: "I-13" type: "article" title: "Spineless Traversal for Layout Invalidation" author: "Kirisame, Wang & Panchekha" date: 2025 venue: "PLDI" vtype: "proceedings"}

    # J. Higher-dimensional rewriting, polygraphs, PROPs, string diagrams, feedback/trace
    {id: "J-1" type: "book" title: "Polygraphs: From Rewriting to Higher Categories" author: "Ara, Burroni, Guiraud, Malbos, Métayer & Mimram" date: 2025 publisher: "Cambridge University Press" note: "CUP LMS LNS 495; 2023/2025"}
    {id: "J-2" type: "article" title: "Substructural Abstract Syntax with Variable Binding and Single-Variable Substitution" author: "Fiore & Ranchod" date: 2025 venue: "LICS" vtype: "proceedings"}
    {id: "J-3" type: "book" title: "A Foundation for PROPs, Algebras, and Modules" author: "Yau & Johnson"}
    {id: "J-4" type: "article" title: "Higher-Dimensional Algebra via Colored PROPs" author: "Yau"}
    {id: "J-5" type: "article" title: "Wheeled PROPs, Graph Complexes and the Master Equation" author: "Markl, Merkulov & Shadrin"}
    {id: "J-6" type: "article" title: "Invariant Theory and Wheeled PROPs" author: "Derksen & Makam"}
    {id: "J-7a" type: "article" title: "Nominal String Diagrams" author: "Balco & Kurz"}
    {id: "J-7b" type: "article" title: "Completeness of Nominal PROPs" author: "Balco & Kurz"}
    {id: "J-8" type: "article" title: "String Diagram Rewrite Theory I–III" author: "Bonchi, Gadducci, Kissinger, Sobociński & Zanasi" date: 2020 note: "2020–2022"}
    {id: "J-9" type: "article" title: "Traced Monoidal Categories" author: "Joyal, Street & Verity" date: 1996 venue: "Math. Proc. Camb. Phil. Soc." vtype: "periodical"}
    {id: "J-10" type: "article" title: "Feedback, Trace and Fixed-Point Semantics" author: "Katis, Sabadini & Walters" date: 2002 venue: "RAIRO ITA" vtype: "periodical"}
    {id: "J-11" type: "article" title: "Monoidal Streams for Dataflow Programming" author: "Di Lavore, de Felice & Román" date: 2022 venue: "LICS" vtype: "proceedings"}
    {id: "J-12" type: "article" title: "Recursion from Cyclic Sharing" author: "Hasegawa" date: 1997 venue: "TLCA" vtype: "proceedings"}
    {id: "J-13" type: "article" title: "A Practical Type Theory for Symmetric Monoidal Categories" author: "Shulman"}
    {id: "J-14" type: "article" title: "Semi-Axiomatic Sequent Calculus" author: "DeYoung, Pfenning & Pruiksma" date: 2020 venue: "FSCD" vtype: "proceedings" note: "SAX"}
    {id: "J-15" type: "article" title: "Cellular Monads from Positive GSOS Specifications" author: "Hirschowitz" date: 2019 venue: "EXPRESS/SOS" vtype: "proceedings"}
    {id: "J-16" type: "article" title: "A folk model structure on ω-Cat" author: "Lafont, Métayer & Worytkiewicz" date: 2010 venue: "Adv. Math." vtype: "periodical"}
    {id: "J-17" type: "article" title: "Resolutions by Polygraphs" author: "Métayer" date: 2003 venue: "TAC" vtype: "periodical"}
    {id: "J-18" type: "article" title: "The category of 3-computads is not cartesian closed" author: "Makkai & Zawadowski"}
    {id: "J-19" type: "article" title: "Convergent presentations and polygraphic resolutions of associative algebras" author: "Guiraud, Hoffbeck & Malbos" date: 2019 venue: "Math. Z." vtype: "periodical"}
    {id: "J-20" type: "article" title: "Higher-dimensional categories with finite derivation type" author: "Guiraud & Malbos" date: 2009 venue: "TAC" vtype: "periodical"}
    {id: "J-21" type: "article" title: "A Homotopical Completion Procedure with Applications to Coherence of Monoids" author: "Guiraud, Malbos & Mimram" date: 2013 venue: "RTA" vtype: "proceedings"}
    {id: "J-23" type: "article" title: "Computads for weak ω-categories as an inductive type" author: "Dean, Finster, Markakis, Reutter & Vicary"}
    {id: "J-24" type: "article" title: "Coherent confluence modulo" author: "Dupont & Malbos"}
    {id: "J-25a" type: "article" title: "Simple Word Problems in Universal Algebras" author: "Knuth & Bendix" date: 1970}
    {id: "J-25b" type: "article" title: "Word Problems and a Homological Finiteness Condition for Monoids" author: "Squier" date: 1987 venue: "JPAA" vtype: "periodical"}
    {id: "J-25c" type: "article" title: "A Finiteness Condition for Rewriting Systems" author: "Squier, Otto & Kobayashi" date: 1994 venue: "TCS" vtype: "periodical"}
    {id: "J-26" type: "article" title: "Fundamentals of Compositional Rewriting Theory" author: "Behr, Harmer & Krivine" date: 2022}
    {id: "J-27" type: "article" title: "Convolution Products on Double Categories and Categorification of Rule Algebras" author: "Behr–Melliès–Zeilberger"}
    {id: "J-28" type: "thesis" title: "Rewriting methods in higher algebra" author: "Guiraud" date: 2019 genre: "Habilitation" org: "Univ. Paris 7"}
    {id: "J-29" type: "thesis" title: "Cubical categories for homotopy and rewriting" author: "Lucas" date: 2017 genre: "PhD thesis" org: "Univ. Paris Diderot"}
    {id: "J-30" type: "article" title: "Circuit Algebras are Wheeled Props" author: "Dancso, Halacheva & Robertson" date: 2021 venue: "JPAA" vtype: "periodical" note: "preliminary version titled: Wheeled PROPs and Circuit Algebras"}
    {id: "J-31" type: "article" title: "Extracting a Proof of Coherence for Monoidal Categories from a Proof of Normalization for Monoids" author: "Beylin & Dybjer" date: 1996 venue: "TYPES" vtype: "proceedings" note: "LNCS 1158"}
    {id: "J-32" type: "thesis" title: "Coherent Presentation of Groups in Homotopy Type Theory" author: "Oleon" date: 2025 genre: "PhD thesis" org: "Institut Polytechnique de Paris" note: "NNT 2025IPPAX139"}
    {id: "J-33" type: "article" title: "An Implementation of Polygraphs" author: "Lucas" date: 2019}
    {id: "J-34" type: "article" title: "Polygraphic Programs and Polynomial-Time Functions" author: "Bonfante & Guiraud" date: 2009 venue: "LMCS" vtype: "periodical"}
    {id: "J-35" type: "thesis" title: "Computational Descriptions of Higher Categories" author: "Forest" date: 2021 genre: "PhD thesis" org: "Institut Polytechnique de Paris" note: "NNT 2021IPPAX003"}
    {id: "J-36" type: "thesis" title: "Computational Aspects of Rewriting in Higher-Dimensional Diagrams" author: "Kessler" date: 2025 genre: "PhD thesis" org: "Tallinn University of Technology"}
    {id: "J-37" type: "thesis" title: "A Computational Approach to Higher Categories" author: "Tataru" date: 2024 genre: "PhD thesis" org: "University of Cambridge"}
    {id: "J-38" type: "thesis" title: "Automated Rewriting for Higher Categories and Applications to Quantum Theory" author: "Bar" date: 2016 genre: "DPhil thesis" org: "University of Oxford"}
    {id: "J-39" type: "article" title: "Cofibrant Complexes are Free" author: "Métayer" date: 2007}
    {id: "J-40" type: "article" title: "Towards 3-Dimensional Rewriting Theory" author: "Mimram" date: 2014 venue: "LMCS" vtype: "periodical"}
    {id: "J-41" type: "article" title: "String Diagrams for Non-Strict Monoidal Categories" author: "Wilson et al." date: 2023 venue: "CSL" vtype: "proceedings"}
    {id: "J-42" type: "article" title: "The Free Bifibration on a Functor" author: "Clarke et al." date: 2025}
    {id: "J-43" type: "article" title: "Certified Normalization of Generalized Traces" author: "Maarand & Uustalu" date: 2019 venue: "ISSE" vtype: "periodical" note: "Innovations in Systems and Software Engineering, vol. 15"}

    # K. Nominal sets & names
    {id: "K-1" type: "book" title: "Nominal Sets: Names and Symmetry in Computer Science" author: "Pitts" date: 2013 publisher: "Cambridge University Press"}
    {id: "K-2" type: "article" title: "Supported Sets — A New Foundation for Nominal Sets and Automata" author: "Wißmann" date: 2022}
    {id: "K-3" type: "article" title: "Nominal Unification" author: "Urban, Pitts & Gabbay" date: 2004 venue: "TCS" vtype: "periodical"}
    {id: "K-4" type: "article" title: "Graded Monads in the Semantics of Nominal Automata" author: "Schulze, Schröder & Cengiz" date: 2025}
    {id: "K-5" type: "article" title: "Nominal Automata with Name Deallocation" author: "Prucker, Milius & Schröder"}
    {id: "K-6" type: "article" title: "Nominal Tree Automata with Name Allocation" author: "Prucker & Schröder"}
    {id: "K-7" type: "article" title: "Nominal Automata with Name Binding" author: "Schröder, Kozen, Milius & Wißmann" date: 2017 venue: "FoSSaCS" vtype: "proceedings" note: "RNNA"}
    {id: "K-8" type: "article" title: "A Robust Class of Data Languages and an Application to Learning" author: "Bollig, Habermehl, Leucker & Monmege" date: 2014 venue: "LMCS" vtype: "periodical" note: "session automata"}
    {id: "K-9" type: "article" title: "Deciding Equivalence of Finite Tree Automata" author: "Seidl" date: 1990 venue: "SIAM J. Comput." vtype: "periodical"}
    {id: "K-10" type: "article" title: "Tree Automata over Infinite Alphabets" author: "Kaminski & Tan" date: 2008}
    {id: "K-11" type: "article" title: "A Linear-Time Nominal μ-Calculus with Name Allocation" author: "Hausmann, Milius & Schröder" date: 2021 venue: "MFCS" vtype: "proceedings"}
    {id: "K-12" type: "article" title: "Scalar and Vectorial μ-Calculus with Atoms" author: "Klin & Łełyk" date: 2019 venue: "LMCS" vtype: "periodical"}
    {id: "K-13" type: "article" title: "Alternating Nominal Automata with Name Allocation" author: "Frank, Hausmann, Milius, Schröder & Urbat" date: 2024}
    {id: "K-14" type: "article" title: "Nominal Sets in Rocq" author: "Paranhos & Ventura" date: 2025 venue: "LSFA" vtype: "proceedings"}
    {id: "K-15" type: "misc" title: "Advanced Nominal Techniques" author: "Gabbay" date: 2019 venue: "FoPSS" vtype: "proceedings" note: "summer-school slides"}

    # L. Dependent / erasible evidence, cost-aware LF, metatheory, unfolding
    {id: "L-1" type: "article" title: "calf: A Cost-Aware Logical Framework" author: "Niu, Sterling, Grodin & Harper" date: 2022 venue: "POPL" vtype: "proceedings"}
    {id: "L-2" type: "article" title: "decalf: A Directed, Effectful Cost-Aware Logical Framework" author: "Grodin, Niu, Sterling & Harper" date: 2024 venue: "POPL" vtype: "proceedings"}
    {id: "L-3" type: "article" title: "Focusing on Refinement Typing" author: "Economou, Krishnaswami & Dunfield" date: 2023 venue: "TOPLAS" vtype: "periodical"}
    {id: "L-4" type: "thesis" title: "First Steps in Synthetic Tait Computability" author: "Sterling" date: 2021 genre: "PhD thesis" org: "CMU" note: "ch. 8"}
    {id: "L-5" type: "article" title: "The Lisp in the Cellar: Dependent Types that Live Upstairs" author: "Dagand & Peschanski" venue: "ELS" vtype: "proceedings"}
    {id: "L-6" type: "article" title: "Controlling Unfolding in Type Theory" author: "Gratzer, Sterling, Angiuli, Coquand & Birkedal"}
    {id: "L-7" type: "article" title: "The Fire Triangle: How to Mix Substitution, Dependent Elimination, and Effects" author: "Pédrot & Tabareau" date: 2020 venue: "POPL" vtype: "proceedings"}
    {id: "L-8" type: "article" title: "Mechanizing Synthetic Tait Computability in Istari" author: "Li, Yao & Harper" date: 2026 venue: "CPP" vtype: "proceedings"}
    {id: "L-9" type: "article" title: "Generic Level Polymorphic N-ary Functions" author: "Allais et al." date: 2021}

    # M. Universe stratification
    {id: "M-1" type: "article" title: "Loop-checking and the uniform word problem for join-semilattices with an inflationary endomorphism" author: "Bezem & Coquand" date: 2022 venue: "TCS" vtype: "periodical"}
    {id: "M-2a" type: "article" title: "Type Theory with Explicit Universe Polymorphism" author: "Bezem, Coquand, Dybjer & Escardó"}
    {id: "M-2b" type: "article" title: "A Generalized Algebraic Theory for Type Theory with Explicit Universe Polymorphism" author: "Bezem, Coquand, Dybjer & Escardó" date: 2026}
    {id: "M-3" type: "article" title: "An Order-Theoretic Analysis of Universe Polymorphism" author: "Hou (Favonia), Angiuli & Mullanix" date: 2023 venue: "POPL" vtype: "proceedings" note: "displacement algebras"}
    {id: "M-4" type: "article" title: "StraTT" author: "Chan & Weirich" note: "stratified type theory"}
    {id: "M-5" type: "article" title: "Normalisation for First-Class Universe Levels" author: "Danielsson, Favier & Kubánek" date: 2026 venue: "POPL" vtype: "proceedings"}
    {id: "M-6" type: "article" title: "Fast Computations on Ordered Nominal Sets" author: "Venhoek, Moerman & Rot" date: 2018 venue: "ICTAC" vtype: "proceedings" note: "TCS 2022"}
    {id: "M-7" type: "misc" title: "Algebraic Universes and Variances For All" author: "Sozeau & Bezem" date: 2025 venue: "TYPES" vtype: "proceedings" note: "TYPES 2025 abstract + Rocq branch"}
    {id: "M-8" type: "article" title: "All Your Base" author: "Poiret et al." date: 2025 venue: "POPL" vtype: "proceedings" note: "prenex sort polymorphism"}
    {id: "M-9" type: "article" title: "Definitional Proof-Irrelevance without K" author: "Gilbert, Cockx, Sozeau & Tabareau" date: 2019 venue: "POPL" vtype: "proceedings" note: "SProp"}
    {id: "M-10" type: "article" title: "Bounded First-Class Universe Levels in Dependent Type Theory" author: "Chan" date: 2025}
    {id: "M-11a" type: "article" title: "Generalized Universe Hierarchies and First-Class Universe Levels" author: "Kovács" date: 2022 venue: "CSL" vtype: "proceedings"}
    {id: "M-11b" type: "article" title: "Canonicity for Indexed Inductive-Recursive Types" author: "Kovács" date: 2026 venue: "POPL" vtype: "proceedings"}
    {id: "M-12a" type: "article" title: "Coq Coq Correct!" author: "Sozeau, Boulier, Forster, Tabareau & Winterhalter" date: 2020 venue: "POPL" vtype: "proceedings" note: "MetaCoq verified checker; universe-constraint checking via longest-simple-paths"}
    {id: "M-12b" type: "article" title: "Lean4Lean" author: "Carneiro"}
    {id: "M-13" type: "article" title: "Normalization by Evaluation for Non-cumulativity" author: "Jiang, Hu & Oliveira" date: 2025 venue: "ICFP" vtype: "proceedings"}
    {id: "M-14" type: "article" title: "Type Universes as Kripke Worlds" author: "Koronkevich & Bowman" date: 2025}
    {id: "M-15" type: "article" title: "Hofmann–Streicher Lifting of Fibred Categories" author: "Slattery et al." date: 2025}

    # N. Levitation, containers, descriptions, induction-recursion, aggregation
    {id: "N-1a" type: "article" title: "The Gentle Art of Levitation" author: "Chapman, Dagand, McBride & Morris" date: 2010 venue: "ICFP" vtype: "proceedings"}
    {id: "N-1b" type: "thesis" title: "A Cosmology of Datatypes: Reusability and Dependent Types" author: "Dagand" date: 2013 genre: "PhD thesis" org: "University of Strathclyde"}
    {id: "N-2" type: "article" title: "Containers" author: "Abbott, Altenkirch & Ghani" date: 2005 venue: "TCS" vtype: "periodical"}
    {id: "N-3" type: "article" title: "Functorial Aggregation" author: "Spivak et al." date: 2021 note: "2021–2025"}
    {id: "N-4a" type: "article" title: "Internal Type Theory" author: "Dybjer" date: 1996 venue: "TYPES" vtype: "proceedings" note: "LNCS 1158"}
    {id: "N-4b" type: "article" title: "A General Formulation of Simultaneous Inductive-Recursive Definitions in Type Theory" author: "Dybjer" date: 2000 venue: "JSL" vtype: "periodical"}
    {id: "N-4c" type: "article" title: "A Finite Axiomatization of Inductive-Recursive Definitions" author: "Dybjer & Setzer" date: 1999 venue: "TLCA" vtype: "proceedings"}
    {id: "N-4d" type: "article" title: "Induction-Recursion and Initial Algebras" author: "Dybjer & Setzer" date: 2003 venue: "APAL" vtype: "periodical"}
    {id: "N-5" type: "thesis" title: "Inductive-inductive definitions" author: "Nordvall Forsberg" date: 2013 genre: "PhD thesis" org: "Swansea University"}
    {id: "N-6" type: "article" title: "General Recursion via Coinductive Types" author: "Capretta" date: 2005 venue: "LMCS" vtype: "periodical"}
    {id: "N-7" type: "article" title: "Container Combinatorics: Monads and Lax Monoidal Functors" author: "Uustalu" date: 2017 venue: "TTCS" vtype: "proceedings"}
    {id: "N-8" type: "article" title: "Directed Containers as Categories" author: "Ahman & Uustalu" date: 2016}
    {id: "N-9" type: "article" title: "Monoid Structures on Indexed Containers" author: "De Pascalis et al." date: 2025}

    # O. Codata, copatterns, sized types
    {id: "O-1a" type: "article" title: "Copatterns: Programming Infinite Structures by Observations" author: "Abel, Pientka, Thibodeau & Setzer" date: 2013 venue: "POPL" vtype: "proceedings"}
    {id: "O-1b" type: "article" title: "Well-Founded Recursion with Copatterns and Sized Types" author: "Abel & Pientka" date: 2016 venue: "JFP" vtype: "periodical"}
    {id: "O-2" type: "article" title: "NbE for Sized Dependent Types" author: "Abel, Vezzosi & Winterhalter" date: 2017 venue: "ICFP" vtype: "proceedings" note: "art. 33"}
    {id: "O-3a" type: "article" title: "Elaborating Dependent (Co)pattern Matching" author: "Cockx & Abel" date: 2018 venue: "ICFP" vtype: "proceedings" note: "art. 75"}
    {id: "O-3b" type: "thesis" title: "Dependent Pattern Matching and Proof-Relevant Unification" author: "Cockx" date: 2017 genre: "PhD thesis" org: "KU Leuven"}
    {id: "O-4a" type: "article" title: "Infinite Objects in Type Theory" author: "Coquand" date: 1994 venue: "TYPES" vtype: "proceedings"}
    {id: "O-4b" type: "article" title: "Codifying Guarded Definitions with Recursive Schemes" author: "Giménez" date: 1995 venue: "TYPES" vtype: "proceedings" note: "LNCS 996"}
    {id: "O-5" type: "article" title: "A Computational Understanding of Classical (Co)Recursion" author: "Downen & Ariola" date: 2020 venue: "PPDP" vtype: "proceedings"}
    {id: "O-6" type: "article" title: "MiniAgda: Integrating Sized and Dependent Types" author: "Abel" date: 2010 venue: "PAR" vtype: "proceedings"}

    # P. Metaprogramming, staging, hygiene
    {id: "P-1" type: "article" title: "Binding as Sets of Scopes" author: "Flatt" date: 2016 venue: "POPL" vtype: "proceedings"}
    {id: "P-2" type: "article" title: "Beyond Notations" author: "Ullrich & de Moura" date: 2020 venue: "IJCAR" vtype: "proceedings" note: "Lean 4 hygiene"}
    {id: "P-3a" type: "article" title: "Environment Classifiers" author: "Taha & Nielsen" date: 2003 venue: "POPL" vtype: "proceedings"}
    {id: "P-3b" type: "article" title: "The Design and Implementation of BER MetaOCaml" author: "Kiselyov" date: 2014 venue: "FLOPS" vtype: "proceedings"}
    {id: "P-4a" type: "article" title: "MacroCaml" author: "Xie, White, Nicole & Yallop" date: 2023 venue: "ICFP" vtype: "proceedings"}
    {id: "P-4b" type: "article" title: "Staging with Class: A Specification for Typed Template Haskell" author: "Xie et al." date: 2022 venue: "POPL" vtype: "proceedings"}
    {id: "P-5" type: "article" title: "Staged Compilation with Two-Level Type Theory" author: "Kovács" date: 2022 venue: "ICFP" vtype: "proceedings"}
    {id: "P-6a" type: "article" title: "Elaborator Reflection: Extending Idris in Idris" author: "Christiansen & Brady" date: 2016 venue: "ICFP" vtype: "proceedings"}
    {id: "P-6b" type: "web" title: "Agda TC monad" note: "reflection machinery, Agda documentation"}
    {id: "P-7" type: "thesis" title: "Tactics in Agda Using Reflection" author: "van der Stel" date: 2022 genre: "MSc thesis" org: "TU Delft"}

    # Q. Identity, univalence, directed TT, VDC reflection
    {id: "Q-1" type: "article" title: "An Internal Logic of Virtual Double Categories" author: "Nasu" note: "FVDblTT; SECONDARY reference — Q-2 is the primary FVDblTT reference (owner decision)"}
    {id: "Q-2" type: "thesis" title: "Logical Aspects of Virtual Double Categories" author: "Nasu" date: 2025 genre: "Master's thesis" note: "PRIMARY FVDblTT reference (owner decision)"}
    {id: "Q-3" type: "article" title: "Di- is for Directed" author: "Laretto, Loregian & Veltri" date: 2026 venue: "POPL" vtype: "proceedings" note: "directed type theory via dinaturality"}
    {id: "Q-4" type: "article" title: "Pattern Matching without K" author: "Cockx, Devriese & Piessens"}
    {id: "Q-5" type: "article" title: "VETT" author: "New & Licata" date: 2023 venue: "FoSSaCS" vtype: "proceedings" note: "hyperdoctrines of virtual equipments"}
    {id: "Q-6a" type: "article" title: "A Type Theory for Cartesian Closed Bicategories" author: "Fiore & Saville" date: 2019 venue: "LICS" vtype: "proceedings"}
    {id: "Q-6b" type: "article" title: "Coherence for Bicategorical Cartesian Closed Structure" author: "Fiore & Saville" date: 2021 venue: "MSCS" vtype: "periodical" note: "LiCS 2020 conference ancestor doi:10.1145/3373718.3394769"}
    {id: "Q-7a" type: "article" title: "A Higher-Order Calculus for Categories" author: "Cáccamo & Winskel" date: 2001 venue: "CSL" vtype: "proceedings" note: "BRICS RS-01-27"}
    {id: "Q-7b" type: "thesis" title: "A Formal Calculus for Categories" author: "Cáccamo" date: 2003 genre: "PhD dissertation" org: "Aarhus" note: "BRICS DS-03-7"}
    {id: "Q-8" type: "article" title: "A Unified Framework for Generalized Multicategories" author: "Cruttwell & Shulman" date: 2010 venue: "TAC" vtype: "periodical" note: "virtual double categories"}
    {id: "Q-9" type: "article" title: "Enriched Indexed Categories" author: "Shulman" note: "corrected title (owner round-1 reversal); arXiv:1212.3914 is the work used at the citation site"}
    {id: "Q-10" type: "article" title: "The structure of multiplicatives" author: "Danos & Regnier"}
    {id: "Q-11" type: "article" title: "Univalent Double Categories" author: "van der Weide, Rasekh, Ahrens & North"}
    {id: "Q-12" type: "article" title: "The formal theory of relative monads" author: "Arkor & McDermott"}
    {id: "Q-13" type: "article" title: "Framed Bicategories and Monoidal Fibrations" author: "Shulman" note: "split from Q-9 (title/id conflation), owner round-1 decision"}
    {id: "Q-14" type: "article" title: "Symmetries in Reversible Programming: From Symmetric Rig Groupoids to Reversible Programming Languages" author: "Choudhury, Karwowski & Sabry" date: 2022 venue: "POPL" vtype: "proceedings"}
    {id: "Q-15" type: "article" title: "Directed Univalence in Simplicial Homotopy Type Theory" author: "Gratzer, Weinberger & Buchholtz" date: 2024}
    {id: "Q-16" type: "thesis" title: "On Multiple ∞-Categories and Formal Category Theory via ∞-Equipments" author: "Ruit" date: 2025 genre: "PhD thesis" org: "Utrecht University"}
    {id: "Q-17" type: "article" title: "Formal Category Theory in ∞-Equipments I" author: "Ruit" date: 2023}
    {id: "Q-18" type: "article" title: "Double Categories of Profunctors" author: "Kawase et al." date: 2025}
    {id: "Q-19" type: "article" title: "Coend Calculus" author: "Loregian et al."}
    {id: "Q-20" type: "article" title: "Univalent Typoids" author: "Petrakis et al." date: 2022}
    {id: "Q-21" type: "thesis" title: "Cartesian Closed Bicategories: Type Theory and Coherence" author: "Saville" date: 2020 genre: "PhD thesis" org: "University of Cambridge"}
    {id: "Q-22" type: "book" title: "Categorical Logic and Type Theory" author: "Jacobs" date: 1999 publisher: "Elsevier" note: "Studies in Logic 141"}

    # R. Internal-univalence manual bibliography (iu:docs/manual/refs.yml)
    {id: "R-1" type: "book" title: "Foundations of Constructive Analysis" author: "Bishop" date: 1967 publisher: "McGraw-Hill"}
    {id: "R-2" type: "chapter" title: "Constructive mathematics and computer programming" author: "Martin-Löf" date: 1982 note: "Logic, Methodology and Philosophy of Science VI, North-Holland 1982, pp.153-175"}
    {id: "R-3" type: "article" title: "Type Theory and its Meaning Explanations" author: "Sterling"}
    {id: "R-4" type: "chapter" title: "Program Testing and the Meaning Explanations of Intuitionistic Type Theory" author: "Dybjer" date: 2012 note: "in Epistemology versus Ontology, Springer 2012, pp. 215–241"}
    {id: "R-5" type: "chapter" title: "Internal Type Theory" author: "Dybjer" date: 1996 note: "TYPES 1995 / LNCS 1996"}
    {id: "R-6" type: "book" title: "Homotopy Type Theory: Univalent Foundations of Mathematics" author: "The Univalent Foundations Program"}
    {id: "R-7" type: "article" title: "A general formulation of simultaneous inductive-recursive definitions in type theory" author: "Dybjer" date: 2000 note: "JSL 2000"}
    {id: "R-8" type: "article" title: "A Finite Axiomatization of Inductive-Recursive Definitions" author: "Dybjer & Setzer" date: 1999 note: "TLCA 1999"}
    {id: "R-9" type: "article" title: "Induction-recursion and initial algebras" author: "Dybjer & Setzer" date: 2003 note: "Annals of Pure and Applied Logic 2003"}
    {id: "R-10" type: "thesis" title: "Inductive-Inductive Definitions" author: "Nordvall Forsberg" date: 2013 genre: "PhD thesis" org: "Swansea"}
    {id: "R-11" type: "article" title: "General Recursion via Coinductive Types" author: "Capretta" date: 2005 note: "LMCS 2005"}
    {id: "R-12" type: "article" title: "A Syntactical Approach to Weak ω-Groupoids" author: "Altenkirch & Rypáček" date: 2012 note: "CSL 2012, LIPIcs"}
    {id: "R-13" type: "article" title: "Some constructions on ω-groupoids" author: "Altenkirch, Li & Rypáček" date: 2014 note: "LFMTP 2014"}
    {id: "R-14" type: "thesis" title: "Quotient Types in Type Theory" author: "Nuo Li" date: 2015 genre: "PhD thesis" org: "Nottingham"}
    {id: "R-15" type: "article" title: "Types are weak ω-groupoids" author: "van den Berg & Garner"}
    {id: "R-16" type: "article" title: "Weak ω-categories from intensional type theory" author: "Lumsdaine"}
    {id: "R-17" type: "article" title: "Martin-Löf Complexes" author: "Awodey, Hofstra & Warren"}
    {id: "R-18" type: "chapter" title: "The groupoid interpretation of type theory" author: "Hofmann & Streicher" date: 1998 note: "Twenty-Five Years of Constructive Type Theory, OUP 1998"}
    {id: "R-19" type: "article" title: "A Type-Theoretical Definition of Weak ω-Categories" author: "Finster & Mimram" date: 2017 note: "LICS 2017"}
    {id: "R-20" type: "article" title: "A Type Theory for Strictly Unital ∞-Categories" author: "Finster, Reutter, Rice & Vicary" date: 2022 note: "LICS 2022"}
    {id: "R-21" type: "article" title: "A Syntax for Strictly Associative and Unital ∞-Categories" author: "Finster, Rice & Vicary" date: 2024 note: "LICS 2024"}
    {id: "R-22" type: "article" title: "A type-theoretic approach to semistrict higher categories" author: "Rice"}
    {id: "R-23" type: "article" title: "A type theory for invertibility in weak ω-categories" author: "Benjamin, Champin & Markakis"}
    {id: "R-24" type: "article" title: "A cellular type theory" author: "Leclerc & Mimram" note: "draft, 2026-01-22"}
    {id: "R-25" type: "repository" title: "The agda-unimath library" author: "Rijke, Stenholm, Prieto-Cubides, Bakke, Štěpančík" date: 2025 note: "web"}
    {id: "R-26" type: "article" title: "On Hofmann–Streicher universes" author: "Awodey" date: 2022 note: "article"}
    {id: "R-27" type: "repository" title: "Narya: a proof assistant for higher-dimensional type theory" author: "Shulman" date: 2025 note: "web"}
    {id: "R-28" type: "thesis" title: "Rewriting methods in higher algebra" author: "Guiraud" date: 2019 genre: "HDR thesis" org: "Université Paris 7"}
    {id: "R-29" type: "book" title: "Polygraphs: from Rewriting to Higher Categories" author: "Ara, Burroni, Guiraud, Malbos, Métayer, Mimram" date: 2025 publisher: "Cambridge University Press" note: "book"}
    {id: "R-30" type: "thesis" title: "Cubical categories for homotopy and rewriting" author: "Lucas" date: 2017 genre: "PhD thesis" org: "Université Paris Diderot"}
    {id: "R-31" type: "thesis" title: "Extensional concepts in intensional type theory" author: "Hofmann" date: 1995 genre: "PhD thesis" org: "University of Edinburgh" note: "ECS-LFCS-95-327"}
    {id: "R-32" type: "article" title: "Extensional equality in intensional type theory" author: "Altenkirch" date: 1999 note: "LICS 1999, IEEE"}
    {id: "R-33" type: "article" title: "Setoid type theory: a syntactic translation" author: "Altenkirch, Boulier, Kaposi, Tabareau" date: 2019 note: "MPC 2019, Springer LNCS 11825"}
    {id: "R-34" type: "article" title: "Constructing a universe for the setoid model" author: "Altenkirch, Boulier, Kaposi, Sattler, Sestini" date: 2021 note: "FoSSaCS 2021, Springer LNCS 12650"}
    {id: "R-35" type: "article" title: "Proof-relevance of families of setoids and identity in type theory" author: "Palmgren" date: 2012 note: "Archive for Mathematical Logic, vol 51, pp 35–47"}
    {id: "R-36" type: "article" title: "Locally cartesian closed categories without chosen constructions" author: "Palmgren" note: "Theory and Applications of Categories, vol 20, pp 5–17"}
    {id: "R-37" type: "report" title: "Groupoids and local cartesian closure" author: "Palmgren" date: 2003 note: "report, Uppsala University Dept of Mathematics; U.U.D.M. Report 2003:21"}
    {id: "R-38" type: "article" title: "The Interpretation of Intuitionistic Type Theory in Locally Cartesian Closed Categories: an Intuitionistic Perspective" author: "Buisse, Dybjer" note: "ENTCS, vol 218, pp 21–32"}
    {id: "R-39" type: "web" title: "Setoids are not an LCCC" author: "Altenkirch, Kraus" date: 2012 note: "web"}
    {id: "R-40" type: "article" title: "Univalent categories and the Rezk completion" author: "Ahrens, Kapulkin, Shulman" date: 2015 note: "Math. Struct. Comp. Sci. 25(5), 2015"}
    {id: "R-41" type: "article" title: "Galois: a theory development project" author: "Peter Aczel" date: 1993 note: "Turin meeting report, 1993"}
    {id: "R-42" type: "article" title: "An E-bicategory of E-categories: exemplifying a type-theoretic approach to bicategories" author: "K. O. Wilander" date: 2005 note: "Uppsala U.U.D.M. report 2005:48"}
    {id: "R-43" type: "article" title: "Yet another category of setoids with equality on objects" author: "Erik Palmgren"}
    {id: "R-44" type: "article" title: "On equality of objects in categories in constructive type theory" author: "Erik Palmgren" date: 2017 note: "TYPES 2017, LIPIcs"}
    {id: "R-45" type: "article" title: "Substitution up to Isomorphism" author: "P.-L. Curien" date: 1993 note: "Fundamenta Informaticae, 1993"}
    {id: "R-46" type: "article" title: "On the interpretation of type theory in locally cartesian closed categories" author: "Martin Hofmann" note: "CSL, LNCS"}
    {id: "R-47" type: "article" title: "The local universes model: an overlooked coherence construction for dependent type theories" author: "Lumsdaine & Warren"}
    {id: "R-48" type: "article" title: "The Biequivalence of Locally Cartesian Closed Categories and Martin-Löf Type Theories" author: "Clairambault & Dybjer"}
    {id: "R-49" type: "article" title: "Displayed Categories" author: "Ahrens & Lumsdaine" date: 2019 note: "LMCS 2019"}
    {id: "R-50" type: "article" title: "Categorical structures for type theory in univalent foundations" author: "Ahrens, Lumsdaine & Voevodsky" date: 2018 note: "LMCS 2018"}
    {id: "R-51" type: "article" title: "Natural models of homotopy type theory" author: "Steve Awodey" note: "MSCS"}
    {id: "R-52" type: "article" title: "Cubical Type Theory: a constructive interpretation of the univalence axiom" author: "Cohen, Coquand, Huber & Mörtberg" date: 2015 note: "TYPES 2015, LIPIcs"}
    {id: "R-53" type: "article" title: "Normalization for Cubical Type Theory" author: "Sterling & Angiuli" date: 2021 note: "LICS 2021"}
    {id: "R-54" type: "thesis" title: "First Steps in Synthetic Tait Computability: The Objective Metatheory of Cubical Type Theory" author: "Jonathan Sterling" date: 2021 genre: "PhD thesis" org: "CMU"}
    {id: "R-55" type: "article" title: "A Cubical Language for Bishop Sets" author: "Sterling, Angiuli & Gratzer" note: "XTT"}
    {id: "R-56" type: "article" title: "The category of iterative sets in homotopy type theory and univalent foundations" author: "Gratzer, Gylterud, Mörtberg & Stenholm" date: 2024 note: "MSCS 2024"}
    {id: "R-57" type: "thesis" title: "The Category of Iterative Sets in Cubical Agda" author: "Fabian Lukas Grubmüller" date: 2026 genre: "MSc degree project" org: "KTH" note: "refs.yml says \"Stockholm MSc\"; verification places it at KTH Mathematics, presented 2026-02-06"}
    {id: "R-58" type: "article" title: "Canonicity for Indexed Inductive-Recursive Types" author: "András Kovács" date: 2026 note: "PACMPL, POPL 2026"}
    {id: "R-59" type: "article" title: "Normalization and the Yoneda embedding" author: "Čubrić, Dybjer & Scott" date: 1998 note: "MSCS 1998"}
    {id: "R-60" type: "article" title: "Normalization by gluing for free λ-theories" author: "Sterling & Spitters"}

    # S. iu-c2h1 lit distillates
    {id: "S-1" type: "article" title: "Context-Free Languages of String Diagrams" author: "Earnshaw & Román"}
    {id: "S-2" type: "article" title: "Collages of String Diagrams" author: "Braithwaite & Román" date: 2023 note: "ACT 2023"}
    {id: "S-3" type: "misc" title: "Presentations of Premonoidal Categories by Devices" author: "Earnshaw, Nester & Román" date: 2023 note: "NWPT 2023 slides"}
    {id: "S-4" type: "article" title: "Monoidal categories graded by partial commutative monoids" author: "Earnshaw, Nester & Román"}
    {id: "S-5" type: "article" title: "Dialogue Categories and Chiralities" author: "Melliès"}
    {id: "S-6" type: "thesis" title: "Syntax and Models of a non-Associative Composition of Programs and Proofs" author: "Munch-Maccagnoni" date: 2013 genre: "PhD thesis" note: "same work as A-16 (Syntax and Models of a non-Associative Composition of Programs and Proofs)"}
    {id: "S-7" type: "misc" title: "Polar Shuffles" author: "Román et al." date: 2024 note: "slides (2024) / Polar Interleavings for Deadlock-Free Message Passing"}
    {id: "S-8" type: "article" title: "The Produoidal Algebra of Process Decomposition" author: "Earnshaw, Hefford & Román" date: 2024 note: "CSL 2024"}
    {id: "S-9" type: "thesis" title: "Monoidal Context Theory" author: "Román" date: 2023 genre: "PhD thesis" org: "TalTech"}

    # T. Toolchain facets, pretty-printing, self-hosting, solver/SMT, parsing
    {id: "T-1" type: "article" title: "A Pretty Expressive Printer" author: "Porncharoenwase, Pombrio & Torlak" date: 2023 venue: "OOPSLA" vtype: "proceedings"}
    {id: "T-2" type: "thesis" title: "Fully Countering Trusting Trust through Diverse Double-Compiling" author: "Wheeler" date: 2009 genre: "PhD thesis" org: "George Mason University" note: "cite the 2009 dissertation as primary; ACSAC 2005 paper (doi:10.1109/CSAC.2005.17) is the conference ancestor"}
    {id: "T-3" type: "article" title: "Reflections on Trusting Trust" author: "Thompson" date: 1984}
    {id: "T-4" type: "article" title: "Liquid Types" author: "Rondon, Kawaguchi & Jhala" date: 2008 venue: "PLDI" vtype: "proceedings" note: "cited jointly with Vazou et al. — Liquid Haskell"}
    {id: "T-5" type: "article" title: "Z3" author: "de Moura & Bjørner" date: 2008}
    {id: "T-6" type: "web" title: "MoonBit 0.9: Introducing First-Class Formal Verification" date: 2026 note: "MoonBit 0.9 release notes"}
    {id: "T-7" type: "web" title: "GHC typechecker plugins" note: "GHC User's Guide §\"Typechecker plugins\""}
    {id: "T-8" type: "article" title: "Parsing Mixfix Operators" author: "Danielsson & Norell" date: 2011}
    {id: "T-9" type: "web" title: "Gleam"}
    {id: "T-10" type: "article" title: "egg: Fast and Extensible Equality Saturation" author: "Willsey, Nandi, Wang, Flatt, Tatlock & Panchekha" date: 2021 venue: "POPL" vtype: "proceedings"}
    {id: "T-11" type: "article" title: "Better Together: Unifying Datalog and Equality Saturation" author: "Zhang, Wang, Flatt, Cao, Zucker, Rosenthal, Tatlock & Willsey" date: 2023 venue: "PLDI" vtype: "proceedings" note: "egglog; the earlier POPL 2022 Relational E-Matching (doi:10.1145/3498696) is a distinct work conflated in the source shorthand"}
    {id: "T-12" type: "book" title: "The Definition of Standard ML (Revised)" author: "Milner, Tofte, Harper & MacQueen" date: 1997 publisher: "MIT Press"}
    {id: "T-13" type: "article" title: "Boxroot: Fast Movable GC Roots for a Better FFI" author: "Munch-Maccagnoni & Scherer" date: 2022 venue: "ML Workshop" vtype: "proceedings"}
    {id: "T-14" type: "article" title: "Perceus: Garbage Free Reference Counting with Reuse" author: "Reinking, Xie, de Moura & Leijen" date: 2021 venue: "PLDI" vtype: "proceedings"}

    # U. Ludics & Geometry of Interaction
    {id: "U-1" type: "article" title: "Introduction to Linear Logic and Ludics, Part I" author: "Curien" date: 2005}
    {id: "U-2" type: "article" title: "Introduction to Linear Logic and Ludics, Part II" author: "Curien" date: 2005}
    {id: "U-3" type: "article" title: "Computational Ludics" author: "Terui" date: 2011 venue: "TCS" vtype: "periodical" note: "TCS 412"}
    {id: "U-4" type: "article" title: "Towards Ludics Programming: Interactive Proof Search" author: "Saurin" date: 2008 note: "ICLP/LNCS 5366"}
    {id: "U-5" type: "misc" title: "An Introduction to Ludics" author: "Vaux" date: 2011 note: "CLMPS/LOCI symposium slides"}
    {id: "U-6" type: "article" title: "Ludics and its Applications to Natural Language Semantics" author: "Lecomte & Quatrini" date: 2009 note: "WoLLIC/LNAI 5514"}
    {id: "U-7" type: "article" title: "On Dialogue Games and Graph Games" author: "Jacq & Melliès" date: 2018 note: "ENTCS 336"}
    {id: "U-8" type: "article" title: "Asynchronous Template Games and the Gray Tensor Product of 2-Categories" author: "Melliès" date: 2021 venue: "LICS" vtype: "proceedings"}
    {id: "U-9" type: "article" title: "Towards a Rosetta Stone of Interactive and Quantitative Semantics" author: "Clairambault et al." date: 2026 note: "CSL invited"}

    # V. Stone duality & Abstract Stone Duality (ASD)
    {id: "V-1" type: "article" title: "Programming Interfaces and Basic Topology" author: "Hancock & Hyvernat" date: 2006 venue: "APAL" vtype: "periodical" note: "APAL 137"}
    {id: "V-2" type: "chapter" title: "Foundations for Computable Topology" author: "Taylor" date: 2011 note: "in Foundational Theories of Classical and Constructive Mathematics, Springer"}
    {id: "V-3" type: "article" title: "A Lambda Calculus for Real Analysis" author: "Taylor" date: 2010 venue: "JLA" vtype: "periodical" note: "JLA 2(5); LAMCRA"}
    {id: "V-4" type: "article" title: "Efficient Computation with Dedekind Reals" author: "Bauer" date: 2008 venue: "CCA" vtype: "proceedings" note: "extended abstract, joint work with Taylor"}
    {id: "V-5" type: "article" title: "The Dedekind Reals in Abstract Stone Duality" author: "Bauer & Taylor" date: 2009 venue: "MSCS" vtype: "periodical" note: "MSCS 19(4)"}
    {id: "V-6" type: "article" title: "Synthetic Topology of Data Types and Classical Spaces" author: "Escardó" date: 2004 venue: "ENTCS" vtype: "periodical" note: "ENTCS 87"}
    {id: "V-7" type: "thesis" title: "Formal Topology in Univalent Foundations" author: "Tosun" date: 2020 genre: "MSc thesis" org: "Chalmers/Gothenburg" note: "Cubical Agda, --safe"}
    {id: "V-8" type: "thesis" title: "Synthetic Topology and Constructive Metric Spaces" author: "Lešnik" date: 2021 genre: "PhD thesis" org: "Ljubljana" note: "arXiv posting 2021"}
    {id: "V-9" type: "article" title: "A Foundation for Synthetic Stone Duality" author: "Cherubini et al." date: 2024}
    ]
}
