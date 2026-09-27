  (global $__deser_data (mut i32) (i32.const 0))
  (global $__deser_len (mut i32) (i32.const 0))
  (global $__deser_pos (mut i32) (i32.const 0))

  (func $__deser_begin (param $source i32)
    local.get $source i32.load global.set $__deser_len
    local.get $source i32.const 16 i32.add i32.load global.set $__deser_data
    i32.const 0 global.set $__deser_pos)

  (func $__deser_peek (result i32)
    global.get $__deser_pos global.get $__deser_len i32.ge_u
    if i32.const -1 return end
    global.get $__deser_data global.get $__deser_pos i32.const 4 i32.mul i32.add i32.load)

  (func $__deser_get (result i32)
    (local $c i32)
    call $__deser_peek local.set $c
    local.get $c i32.const -1 i32.ne
    if global.get $__deser_pos i32.const 1 i32.add global.set $__deser_pos end
    local.get $c)

  (func $__deser_ws
    block $done loop $scan
      call $__deser_peek
      i32.const 32 i32.eq
      call $__deser_peek i32.const 9 i32.eq i32.or
      call $__deser_peek i32.const 10 i32.eq i32.or
      call $__deser_peek i32.const 13 i32.eq i32.or
      i32.eqz br_if $done
      call $__deser_get drop br $scan
    end end)

  (func $__deser_expect (param $c i32)
    call $__deser_ws
    call $__deser_get local.get $c i32.ne if unreachable end)

  (func $__deser_int (result i32)
    (local $negative i32) (local $value i64) (local $c i32)
    call $__deser_ws
    call $__deser_peek i32.const 45 i32.eq
    if i32.const 1 local.set $negative call $__deser_get drop end
    block $done loop $digits
      call $__deser_peek local.tee $c i32.const 48 i32.lt_s br_if $done
      local.get $c i32.const 57 i32.gt_s br_if $done
      call $__deser_get drop
      local.get $value i64.const 10 i64.mul
      local.get $c i32.const 48 i32.sub i64.extend_i32_u i64.add local.set $value
      br $digits
    end end
    local.get $negative
    if i64.const 0 local.get $value i64.sub local.set $value end
    local.get $value i64.const -2147483648 i64.lt_s
    local.get $value i64.const 2147483647 i64.gt_s i32.or if unreachable end
    local.get $value i32.wrap_i64)

  (func $__deser_dec (result i32)
    (local $negative i32) (local $whole i64) (local $frac i64)
    (local $places i32) (local $c i32) (local $scaled i64)
    call $__deser_ws
    call $__deser_peek i32.const 45 i32.eq
    if i32.const 1 local.set $negative call $__deser_get drop end
    block $whole_done loop $whole_digits
      call $__deser_peek local.tee $c i32.const 48 i32.lt_s br_if $whole_done
      local.get $c i32.const 57 i32.gt_s br_if $whole_done
      call $__deser_get drop
      local.get $whole i64.const 10 i64.mul
      local.get $c i32.const 48 i32.sub i64.extend_i32_u i64.add local.set $whole
      br $whole_digits
    end end
    call $__deser_peek i32.const 46 i32.eq
    if
      call $__deser_get drop
      block $frac_done loop $frac_digits
        call $__deser_peek local.tee $c i32.const 48 i32.lt_s br_if $frac_done
        local.get $c i32.const 57 i32.gt_s br_if $frac_done
        call $__deser_get drop
        local.get $places i32.const __DEC_DIGITS__ i32.lt_u
        if
          local.get $frac i64.const 10 i64.mul
          local.get $c i32.const 48 i32.sub i64.extend_i32_u i64.add local.set $frac
          local.get $places i32.const 1 i32.add local.set $places
        end
        br $frac_digits
      end end
    end
    block $pad_done loop $pad
      local.get $places i32.const __DEC_DIGITS__ i32.ge_u br_if $pad_done
      local.get $frac i64.const 10 i64.mul local.set $frac
      local.get $places i32.const 1 i32.add local.set $places br $pad
    end end
    local.get $whole i64.const __DEC_SCALE__ i64.mul local.get $frac i64.add local.set $scaled
    local.get $negative if i64.const 0 local.get $scaled i64.sub local.set $scaled end
    local.get $scaled i64.const -2147483648 i64.lt_s
    local.get $scaled i64.const 2147483647 i64.gt_s i32.or if unreachable end
    local.get $scaled i32.wrap_i64)

  (func $__deser_word (param $ptr i32) (param $len i32)
    (local $i i32)
    call $__deser_ws
    block $done loop $chars
      local.get $i local.get $len i32.ge_u br_if $done
      call $__deser_get
      local.get $ptr local.get $i i32.add i32.load8_u
      i32.ne if unreachable end
      local.get $i i32.const 1 i32.add local.set $i br $chars
    end end)

  (func $__deser_string (result i32)
    (local $out i32) (local $c i32)
    i32.const 34 call $__deser_expect
    i32.const 0 i32.const 0 call $vec_new_i32 local.set $out
    block $done loop $chars
      call $__deser_get local.set $c
      local.get $c i32.const 34 i32.eq br_if $done
      local.get $c i32.const -1 i32.eq if unreachable end
      local.get $c i32.const 92 i32.eq
      if
        call $__deser_get local.set $c
        local.get $c i32.const 110 i32.eq if i32.const 10 local.set $c end
        local.get $c i32.const 114 i32.eq if i32.const 13 local.set $c end
        local.get $c i32.const 116 i32.eq if i32.const 9 local.set $c end
        local.get $c i32.const 48 i32.eq if i32.const 0 local.set $c end
      end
      local.get $out local.get $c call $vec_push_i32 drop br $chars
    end end
    local.get $out)
