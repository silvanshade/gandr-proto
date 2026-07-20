; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/21-ffi-native-call.gandr
; b3sum: e8b388d5ba8a6f4cfa35f58314d7eda8d4f143a62f28745a19b8656ca82194e2
; lowering: gandr_pipeline::lower::lower_source_total
; items: 1

(c
  (bind
    (perform
      (sig
        "testlib"
        (op
          "gandr_test_add"
          (trec
            ("a"
              (tatom "i64"))
            ("b"
              (tatom "i64")))
          (tatom "i64"))
        (op
          "gandr_test_strlen"
          (trec
            ("s"
              (tatom "String")))
          (tatom "u64")))
      "gandr_test_add"
      (vrec
        ("a"
          (n
            (i64 21)))
        ("b"
          (n
            (i64 21)))))
    "doubled"
    (bind
      (perform
        (sig
          "testlib"
          (op
            "gandr_test_add"
            (trec
              ("a"
                (tatom "i64"))
              ("b"
                (tatom "i64")))
            (tatom "i64"))
          (op
            "gandr_test_strlen"
            (trec
              ("s"
                (tatom "String")))
            (tatom "u64")))
        "gandr_test_strlen"
        (vrec
          ("s"
            (s "hello"))))
      "length"
      (ret
        (pair
          (var "doubled")
          (var "length"))))))
