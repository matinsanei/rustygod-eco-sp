;;; Reference flat-rate tax plugin for rustygod-saleor (Phase 3).
;;;
;;; Hand-written WAT — no toolchain needed, auditable to the byte.
;;; Uses the raw extism kernel ABI (the same imports `extism-pdk` declares):
;;; input via `input_length`/`input_load_u8`, output via `alloc` +
;;; `store_u8` + `output_set`. No PDK, no libc, no allocator.
;;;
;;; Extension point: `calculate_tax`.
;;; Input:  {"subtotal_cents":10000,"rate_bps":900}
;;; Output: {"tax_cents":900,"total_cents":10900}
;;; tax = (subtotal * rate_bps + 5000) / 10000  (HALF_UP, integer math —
;;; money never touches floats, same rule as the core money module).
(module
  (import "extism:host/env" "input_length" (func $input_length (result i64)))
  (import "extism:host/env" "input_load_u8" (func $input_load_u8 (param i64) (result i32)))
  (import "extism:host/env" "alloc" (func $alloc (param i64) (result i64)))
  (import "extism:host/env" "output_set" (func $output_set (param i64 i64)))
  (import "extism:host/env" "store_u8" (func $store_u8 (param i64 i32)))

  (memory (export "memory") 1)

  ;; Static strings.
  (data (i32.const 0) "\"subtotal_cents\":")   ;; 17 bytes @ 0
  (data (i32.const 32) "\"rate_bps\":")        ;; 11 bytes @ 32
  (data (i32.const 64) "{\"tax_cents\":")      ;; 14 bytes @ 64
  (data (i32.const 96) ",\"total_cents\":")    ;; 16 bytes @ 96
  (data (i32.const 128) "}")                   ;; 1 byte @ 128

  ;; Input staging buffer: 1024 .. 1024+2048.
  (global $in_base i32 (i32.const 1024))
  (global $in_cap i32 (i32.const 2048))

  ;; Copy host input into the staging buffer (truncated to capacity).
  ;; Returns the staged length.
  (func $stage_input (result i32)
    (local $len i32) (local $i i32)
    (local.set $len (i32.wrap_i64 (call $input_length)))
    (if (i32.gt_u (local.get $len) (global.get $in_cap))
      (then (local.set $len (global.get $in_cap))))
    (local.set $i (i32.const 0))
    (block $done
      (loop $l
        (br_if $done (i32.eq (local.get $i) (local.get $len)))
        (i32.store8 (i32.add (global.get $in_base) (local.get $i))
                    (call $input_load_u8 (i64.extend_i32_u (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $l)))
    (local.get $len))

  ;; Find needle[0..nlen) in staged input; return index or -1.
  (func $find (param $n i32) (param $nl i32) (param $hl i32) (result i32)
    (local $i i32) (local $j i32)
    (local.set $i (i32.const 0))
    (block $miss
      (loop $outer
        (br_if $miss
          (i32.gt_u (i32.add (local.get $i) (local.get $nl)) (local.get $hl)))
        (local.set $j (i32.const 0))
        (block $next
          (loop $inner
            (br_if $next (i32.eq (local.get $j) (local.get $nl)))
            (br_if $next
              (i32.ne
                (i32.load8_u (i32.add (i32.add (global.get $in_base) (local.get $i))
                                      (local.get $j)))
                (i32.load8_u (i32.add (local.get $n) (local.get $j)))))
            (local.set $j (i32.add (local.get $j) (i32.const 1)))
            (br $inner)))
        (if (i32.eq (local.get $j) (local.get $nl))
          (then (return (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $outer)))
    (i32.const -1))

  ;; Parse unsigned digits starting at staged offset p (skip non-digits
  ;; first). Returns (value, end_offset).
  (func $parse_u64 (param $p i32) (param $end i32) (result i64 i32)
    (local $v i64) (local $c i32)
    (block $found
      (loop $skip
        (br_if $found (i32.ge_u (local.get $p) (local.get $end)))
        (local.set $c
          (i32.load8_u (i32.add (global.get $in_base) (local.get $p))))
        (br_if $found
          (i32.and (i32.ge_u (local.get $c) (i32.const 48))
                   (i32.le_u (local.get $c) (i32.const 57))))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (br $skip)))
    (local.set $v (i64.const 0))
    (block $done
      (loop $digits
        (br_if $done (i32.ge_u (local.get $p) (local.get $end)))
        (local.set $c
          (i32.load8_u (i32.add (global.get $in_base) (local.get $p))))
        (br_if $done
          (i32.or (i32.lt_u (local.get $c) (i32.const 48))
                  (i32.gt_u (local.get $c) (i32.const 57))))
        (local.set $v
          (i64.add (i64.mul (local.get $v) (i64.const 10))
                   (i64.extend_i32_u (i32.sub (local.get $c) (i32.const 48)))))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (br $digits)))
    (local.get $v) (local.get $p))

  ;; Emit one byte to the output allocation; return next offset.
  (func $emit8 (param $o i64) (param $c i32) (result i64)
    (call $store_u8 (local.get $o) (local.get $c))
    (i64.add (local.get $o) (i64.const 1)))

  ;; Emit a static slice; return next offset.
  (func $emit_str (param $o i64) (param $s i32) (param $n i32) (result i64)
    (local $i i32)
    (local.set $i (i32.const 0))
    (block $done
      (loop $l
        (br_if $done (i32.eq (local.get $i) (local.get $n)))
        (local.set $o
          (call $emit8 (local.get $o)
                       (i32.load8_u (i32.add (local.get $s) (local.get $i)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $l)))
    (local.get $o))

  ;; Emit u64 decimal; return next offset. Reversal scratch at 512.
  (func $emit_u64 (param $o i64) (param $v i64) (result i64)
    (local $n i32) (local $i i32)
    (local.set $n (i32.const 0))
    (if (i64.eqz (local.get $v))
      (then (return (call $emit8 (local.get $o) (i32.const 48)))))
    (block $gen
      (loop $l
        (br_if $gen (i64.eqz (local.get $v)))
        (i32.store8 (i32.add (i32.const 512) (local.get $n))
          (i32.add (i32.const 48)
                   (i32.wrap_i64 (i64.rem_u (local.get $v) (i64.const 10)))))
        (local.set $n (i32.add (local.get $n) (i32.const 1)))
        (local.set $v (i64.div_u (local.get $v) (i64.const 10)))
        (br $l)))
    (local.set $i (i32.const 0))
    (block $rev
      (loop $r
        (br_if $rev (i32.eq (local.get $i) (local.get $n)))
        (local.set $o
          (call $emit8 (local.get $o)
            (i32.load8_u (i32.add (i32.const 512)
              (i32.sub (i32.sub (local.get $n) (i32.const 1)) (local.get $i))))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $r)))
    (local.get $o))

  (func (export "calculate_tax") (result i32)
    (local $hlen i32) (local $at i32)
    (local $sub i64) (local $rate i64) (local $tax i64) (local $total i64)
    (local $o i64) (local $start i64)
    (local.set $hlen (call $stage_input))

    ;; subtotal_cents
    (i32.const 0) (i32.const 17) (local.get $hlen) (call $find)
    (local.set $at)
    (if (i32.eq (local.get $at) (i32.const -1)) (then (return (i32.const 1))))
    (i32.add (local.get $at) (i32.const 17)) (local.get $hlen)
    (call $parse_u64)
    (local.set $at) (local.set $sub)

    ;; rate_bps
    (i32.const 32) (i32.const 11) (local.get $hlen) (call $find)
    (local.set $at)
    (if (i32.eq (local.get $at) (i32.const -1)) (then (return (i32.const 1))))
    (i32.add (local.get $at) (i32.const 11)) (local.get $hlen)
    (call $parse_u64)
    (local.set $at) (local.set $rate)

    ;; tax = (sub * rate + 5000) / 10000 ; total = sub + tax
    (local.set $tax
      (i64.div_u (i64.add (i64.mul (local.get $sub) (local.get $rate))
                          (i64.const 5000))
                 (i64.const 10000)))
    (local.set $total (i64.add (local.get $sub) (local.get $tax)))

    ;; {"tax_cents":T,"total_cents":N}
    (i64.const 64) (call $alloc) (local.set $start)
    (local.set $o (local.get $start))
    (local.set $o (call $emit_str (local.get $o) (i32.const 64) (i32.const 13)))
    (local.set $o (call $emit_u64 (local.get $o) (local.get $tax)))
    (local.set $o (call $emit_str (local.get $o) (i32.const 96) (i32.const 15)))
    (local.set $o (call $emit_u64 (local.get $o) (local.get $total)))
    (local.set $o (call $emit_str (local.get $o) (i32.const 128) (i32.const 1)))
    (call $output_set (local.get $start)
                      (i64.sub (local.get $o) (local.get $start)))
    (i32.const 0))
)
