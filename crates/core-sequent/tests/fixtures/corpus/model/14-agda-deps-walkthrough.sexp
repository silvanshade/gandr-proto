; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/14-agda-deps-walkthrough.gandr
; b3sum: 2854a56e80f6cffcf3f010afae15ce87117bee4a52c19832e4864c1480a2b8f1
; lowering: gandr_pipeline::lower::lower_source_total
; items: 1

(c
  (bind
    (perform
      (sig
        "Exec"
        (op
          "exec"
          (trec
            ("args"
              (tlist
                (tatom "String")))
            ("mode"
              (tatom "String"))
            ("program"
              (tatom "String")))
          (trec
            ("exit_code"
              (tatom "Integer"))
            ("stderr"
              (tatom "String"))
            ("stdout"
              (tatom "String")))))
      "exec"
      (vrec
        ("args"
          (vlist
            (s "-c")
            (s "set -eu; vendor=\"$PWD/metatheory/vendor\"; mkdir -p \"$vendor\"; stdlib=\"$vendor/agda-stdlib\"; stdlib_lib=\"$stdlib/standard-library.agda-lib\"; if [ ! -f \"$stdlib_lib\" ]; then printf \"cloning agda-stdlib v2.4 ...\\n\"; git clone --depth 1 --branch v2.4 https://github.com/agda/agda-stdlib.git \"$stdlib\"; fi; printf \"%s\\n\" \"$stdlib_lib\" > \"$PWD/metatheory/libraries\"; printf \"agda deps ready: stdlib v2.4\\n\"")))
        ("mode"
          (s "captured"))
        ("program"
          (s "sh"))))
    "%tmp0"
    (ret
      (var "%tmp0"))))
