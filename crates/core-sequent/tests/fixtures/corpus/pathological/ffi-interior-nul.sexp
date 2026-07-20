; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: pathological/ffi-interior-nul.gandr
; b3sum: 7d57eca6df45555f92e726af3ed53c03db91ae62d924f66af1a4d8f44124d6af
; lowering: gandr_pipeline::lower::lower_source_total
; items: 1

(c
  (perform
    (sig
      "testlib"
      (op
        "gandr_test_strlen"
        (trec
          ("s"
            (tatom "String")))
        (tatom "u64")))
    "gandr_test_strlen"
    (vrec
      ("s"
        (s "a\u{0}b")))))
