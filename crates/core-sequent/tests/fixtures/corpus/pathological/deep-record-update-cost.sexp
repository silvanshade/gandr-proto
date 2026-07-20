; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: pathological/deep-record-update-cost.gandr
; b3sum: f81a9e5490dbc13acb811bd0b35ff7d14052fd6795c476121b0ec7e56f62670c
; lowering: gandr_pipeline::lower::lower_source_total
; items: 2

(v
  (vrec
    ("a"
      (vrec
        ("b"
          (vrec
            ("c"
              (i 1))))))))

(c
  (bind
    (bind
      (recordproj
        (var "d")
        "a")
      "%tmp0"
      (bind
        (bind
          (bind
            (recordproj
              (var "d")
              "a")
            "%tmp1"
            (recordproj
              (var "%tmp1")
              "b"))
          "%tmp2"
          (app
            (app
              (native recordupdate)
              (var "%tmp2"))
            (vrec
              ("c"
                (i 9)))))
        "%tmp3"
        (app
          (app
            (native recordupdate)
            (var "%tmp0"))
          (vrec
            ("b"
              (var "%tmp3"))))))
    "%tmp4"
    (app
      (app
        (native recordupdate)
        (var "d"))
      (vrec
        ("a"
          (var "%tmp4"))))))
