; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/25-host-modules.gandr
; b3sum: 1ff2b20fd7ab2e7646e172ddaa645aca0da77f19bbfd7e31781a8031a65242a5
; lowering: gandr_pipeline::lower::lower_source_total
; items: 1

(c
  (bind
    (perform
      (sig
        "Fs"
        (op
          "cwd"
          tunit
          (tatom "String"))
        (op
          "glob"
          (tatom "String")
          (tlist
            (tatom "String")))
        (op
          "ls_files"
          (tatom "String")
          (tlist
            (tatom "String")))
        (op
          "mkdir"
          (tatom "String")
          tunit)
        (op
          "read"
          (tatom "String")
          (tatom "String"))
        (op
          "stat"
          (tatom "String")
          (trec
            ("kind"
              (tatom "String"))
            ("size"
              (tatom "Integer"))))
        (op
          "tempdir"
          tunit
          (tatom "String"))
        (op
          "write"
          (trec
            ("contents"
              (tatom "String"))
            ("path"
              (tatom "String")))
          tunit))
      "tempdir"
      u)
    "dir"
    (bind
      (app
        (app
          (force
            (var "path.join"))
          (var "dir"))
        (s "nested"))
      "nested"
      (bind
        (perform
          (sig
            "Fs"
            (op
              "cwd"
              tunit
              (tatom "String"))
            (op
              "glob"
              (tatom "String")
              (tlist
                (tatom "String")))
            (op
              "ls_files"
              (tatom "String")
              (tlist
                (tatom "String")))
            (op
              "mkdir"
              (tatom "String")
              tunit)
            (op
              "read"
              (tatom "String")
              (tatom "String"))
            (op
              "stat"
              (tatom "String")
              (trec
                ("kind"
                  (tatom "String"))
                ("size"
                  (tatom "Integer"))))
            (op
              "tempdir"
              tunit
              (tatom "String"))
            (op
              "write"
              (trec
                ("contents"
                  (tatom "String"))
                ("path"
                  (tatom "String")))
              tunit))
          "mkdir"
          (var "nested"))
        "_"
        (bind
          (app
            (app
              (force
                (var "path.join"))
              (var "nested"))
            (s "greeting.txt"))
          "file"
          (bind
            (perform
              (sig
                "Fs"
                (op
                  "cwd"
                  tunit
                  (tatom "String"))
                (op
                  "glob"
                  (tatom "String")
                  (tlist
                    (tatom "String")))
                (op
                  "ls_files"
                  (tatom "String")
                  (tlist
                    (tatom "String")))
                (op
                  "mkdir"
                  (tatom "String")
                  tunit)
                (op
                  "read"
                  (tatom "String")
                  (tatom "String"))
                (op
                  "stat"
                  (tatom "String")
                  (trec
                    ("kind"
                      (tatom "String"))
                    ("size"
                      (tatom "Integer"))))
                (op
                  "tempdir"
                  tunit
                  (tatom "String"))
                (op
                  "write"
                  (trec
                    ("contents"
                      (tatom "String"))
                    ("path"
                      (tatom "String")))
                  tunit))
              "write"
              (vrec
                ("contents"
                  (s "hello from gandr"))
                ("path"
                  (var "file"))))
            "_"
            (bind
              (perform
                (sig
                  "Fs"
                  (op
                    "cwd"
                    tunit
                    (tatom "String"))
                  (op
                    "glob"
                    (tatom "String")
                    (tlist
                      (tatom "String")))
                  (op
                    "ls_files"
                    (tatom "String")
                    (tlist
                      (tatom "String")))
                  (op
                    "mkdir"
                    (tatom "String")
                    tunit)
                  (op
                    "read"
                    (tatom "String")
                    (tatom "String"))
                  (op
                    "stat"
                    (tatom "String")
                    (trec
                      ("kind"
                        (tatom "String"))
                      ("size"
                        (tatom "Integer"))))
                  (op
                    "tempdir"
                    tunit
                    (tatom "String"))
                  (op
                    "write"
                    (trec
                      ("contents"
                        (tatom "String"))
                      ("path"
                        (tatom "String")))
                    tunit))
                "read"
                (var "file"))
              "content"
              (bind
                (bind
                  (app
                    (app
                      (force
                        (var "path.join"))
                      (var "nested"))
                    (s "*.txt"))
                  "%tmp0"
                  (perform
                    (sig
                      "Fs"
                      (op
                        "cwd"
                        tunit
                        (tatom "String"))
                      (op
                        "glob"
                        (tatom "String")
                        (tlist
                          (tatom "String")))
                      (op
                        "ls_files"
                        (tatom "String")
                        (tlist
                          (tatom "String")))
                      (op
                        "mkdir"
                        (tatom "String")
                        tunit)
                      (op
                        "read"
                        (tatom "String")
                        (tatom "String"))
                      (op
                        "stat"
                        (tatom "String")
                        (trec
                          ("kind"
                            (tatom "String"))
                          ("size"
                            (tatom "Integer"))))
                      (op
                        "tempdir"
                        tunit
                        (tatom "String"))
                      (op
                        "write"
                        (trec
                          ("contents"
                            (tatom "String"))
                          ("path"
                            (tatom "String")))
                        tunit))
                    "glob"
                    (var "%tmp0")))
                "matches"
                (bind
                  (perform
                    (sig
                      "Fs"
                      (op
                        "cwd"
                        tunit
                        (tatom "String"))
                      (op
                        "glob"
                        (tatom "String")
                        (tlist
                          (tatom "String")))
                      (op
                        "ls_files"
                        (tatom "String")
                        (tlist
                          (tatom "String")))
                      (op
                        "mkdir"
                        (tatom "String")
                        tunit)
                      (op
                        "read"
                        (tatom "String")
                        (tatom "String"))
                      (op
                        "stat"
                        (tatom "String")
                        (trec
                          ("kind"
                            (tatom "String"))
                          ("size"
                            (tatom "Integer"))))
                      (op
                        "tempdir"
                        tunit
                        (tatom "String"))
                      (op
                        "write"
                        (trec
                          ("contents"
                            (tatom "String"))
                          ("path"
                            (tatom "String")))
                        tunit))
                    "stat"
                    (var "file"))
                  "stat"
                  (bind
                    (perform
                      (sig
                        "Fs"
                        (op
                          "cwd"
                          tunit
                          (tatom "String"))
                        (op
                          "glob"
                          (tatom "String")
                          (tlist
                            (tatom "String")))
                        (op
                          "ls_files"
                          (tatom "String")
                          (tlist
                            (tatom "String")))
                        (op
                          "mkdir"
                          (tatom "String")
                          tunit)
                        (op
                          "read"
                          (tatom "String")
                          (tatom "String"))
                        (op
                          "stat"
                          (tatom "String")
                          (trec
                            ("kind"
                              (tatom "String"))
                            ("size"
                              (tatom "Integer"))))
                        (op
                          "tempdir"
                          tunit
                          (tatom "String"))
                        (op
                          "write"
                          (trec
                            ("contents"
                              (tatom "String"))
                            ("path"
                              (tatom "String")))
                          tunit))
                      "cwd"
                      u)
                    "cwd"
                    (bind
                      (perform
                        (sig
                          "Fs"
                          (op
                            "cwd"
                            tunit
                            (tatom "String"))
                          (op
                            "glob"
                            (tatom "String")
                            (tlist
                              (tatom "String")))
                          (op
                            "ls_files"
                            (tatom "String")
                            (tlist
                              (tatom "String")))
                          (op
                            "mkdir"
                            (tatom "String")
                            tunit)
                          (op
                            "read"
                            (tatom "String")
                            (tatom "String"))
                          (op
                            "stat"
                            (tatom "String")
                            (trec
                              ("kind"
                                (tatom "String"))
                              ("size"
                                (tatom "Integer"))))
                          (op
                            "tempdir"
                            tunit
                            (tatom "String"))
                          (op
                            "write"
                            (trec
                              ("contents"
                                (tatom "String"))
                              ("path"
                                (tatom "String")))
                            tunit))
                        "ls_files"
                        (s "."))
                      "tracked"
                      (bind
                        (perform
                          (sig
                            "Env"
                            (op
                              "get"
                              (tatom "String")
                              (tatom "String"))
                            (op
                              "path"
                              tunit
                              (tlist
                                (tatom "String"))))
                          "path"
                          u)
                        "path_entries"
                        (bind
                          (app
                            (force
                              (var "path.extension"))
                            (var "file"))
                          "%tmp1"
                          (bind
                            (recordproj
                              (var "stat")
                              "kind")
                            "%tmp2"
                            (bind
                              (app
                                (force
                                  (var "path.basename"))
                                (var "file"))
                              "%tmp3"
                              (ret
                                (vrec
                                  ("content"
                                    (var "content"))
                                  ("ext"
                                    (var "%tmp1"))
                                  ("kind"
                                    (var "%tmp2"))
                                  ("name"
                                    (var "%tmp3")))))))))))))))))))
