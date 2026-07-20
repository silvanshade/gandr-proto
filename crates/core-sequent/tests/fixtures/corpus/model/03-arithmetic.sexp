; gandr pre-lowered corpus fixture (B1 phase; sequent-machines exit gate)
; source: model/03-arithmetic.gandr
; b3sum: 816eb59644bdce0ca006bbb24dda9e16b8c007f29b40a5aa63e662a305f44127
; lowering: gandr_pipeline::lower::lower_source_total
; items: 3

(c
  (bind
    (app
      (app
        (force
          (var "mul"))
        (i 2))
      (i 3))
    "%tmp0"
    (app
      (app
        (force
          (var "add"))
        (i 1))
      (var "%tmp0"))))

(v
  (pair
    (n
      (u32 1))
    (pair
      (n
        (u64 2))
      (pair
        (n
          (i32 3))
        (pair
          (n
            (i64 4))
          (pair
            (n
              (f32 1069547520))
            (n
              (f64 4612811918334230528))))))))

(c
  (ret
    (var "total")))
