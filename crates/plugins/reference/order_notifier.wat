;;; Reference order notifier plugin (Phase 3, #3).
;;;
;;; Hand-written WAT. Extension point: `order.paid`. Needs the `events`
;;; capability: it emits a Slack-style message event to the host, which
;;; delivers it through the webhook system (see docs below).
;;; Input:  {"order_id":"3fa85f64-5717-4562-b3fc-2c963f66afa6"}
;;; Output: {"notified":true}
;;; Event:  {"channel":"#orders","text":"Order <id> paid"}
(module
  (import "extism:host/env" "input_length" (func $input_length (result i64)))
  (import "extism:host/env" "input_load_u8" (func $input_load_u8 (param i64) (result i32)))
  (import "extism:host/env" "alloc" (func $alloc (param i64) (result i64)))
  (import "extism:host/env" "output_set" (func $output_set (param i64 i64)))
  (import "extism:host/env" "store_u8" (func $store_u8 (param i64 i32)))
  (import "extism:host/user" "emit_event" (func $emit (param i64 i64) (result i64)))

  (memory (export "memory") 1)

  (data (i32.const 0) "\"order_id\":\"")                        ;; 12 @ 0
  (data (i32.const 32) "{\"notified\":true}")                   ;; 17 @ 32
  (data (i32.const 64) "{\"channel\":\"#orders\",\"text\":\"Order ") ;; 35 @ 64
  (data (i32.const 128) " paid\"}")                             ;; 7 @ 128

  (global $in_base i32 (i32.const 1024))
  (global $in_cap i32 (i32.const 2048))

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

  (func $emit8 (param $o i64) (param $c i32) (result i64)
    (call $store_u8 (local.get $o) (local.get $c))
    (i64.add (local.get $o) (i64.const 1)))

  ;; Copy n bytes guest->host; return next offset.
  (func $emit_guest (param $o i64) (param $s i32) (param $n i32) (result i64)
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

  ;; Copy staged input bytes [p, end) stopping at `"` (max 64); return (o, count).
  (func $emit_id (param $o i64) (param $p i32) (param $end i32) (result i64 i32)
    (local $n i32) (local $c i32)
    (local.set $n (i32.const 0))
    (block $done
      (loop $l
        (br_if $done (i32.ge_u (local.get $p) (local.get $end)))
        (br_if $done (i32.ge_u (local.get $n) (i32.const 64)))
        (local.set $c
          (i32.load8_u (i32.add (global.get $in_base) (local.get $p))))
        (br_if $done (i32.eq (local.get $c) (i32.const 34))) ;; closing quote
        (local.set $o (call $emit8 (local.get $o) (local.get $c)))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (local.set $n (i32.add (local.get $n) (i32.const 1)))
        (br $l)))
    (local.get $o) (local.get $n))

  (func (export "order.paid") (result i32)
    (local $hlen i32) (local $at i32) (local $idlen i32)
    (local $o i64) (local $start i64)
    (local.set $hlen (call $stage_input))

    ;; order_id value starts after the 12-byte needle.
    (i32.const 0) (i32.const 12) (local.get $hlen) (call $find)
    (local.set $at)
    (if (i32.eq (local.get $at) (i32.const -1)) (then (return (i32.const 1))))
    (local.set $at (i32.add (local.get $at) (i32.const 12)))

    ;; Build the Slack event: prefix + id + suffix (35 + id + 7).
    (i64.const 128) (call $alloc) (local.set $start)
    (local.set $o (local.get $start))
    (local.set $o (call $emit_guest (local.get $o) (i32.const 64) (i32.const 35)))
    (local.get $o) (local.get $at) (local.get $hlen) (call $emit_id)
    (local.set $idlen) (local.set $o)
    (if (i32.eqz (local.get $idlen)) (then (return (i32.const 2))))
    (local.set $o (call $emit_guest (local.get $o) (i32.const 128) (i32.const 7)))
    (call $emit (local.get $start) (i64.sub (local.get $o) (local.get $start)))
    (drop)

    ;; Answer {"notified":true} (static, 17 bytes).
    (i64.const 32) (call $alloc) (local.set $start)
    (local.set $o (local.get $start))
    (local.set $o (call $emit_guest (local.get $o) (i32.const 32) (i32.const 17)))
    (call $output_set (local.get $start)
                      (i64.sub (local.get $o) (local.get $start)))
    (i32.const 0))
)
