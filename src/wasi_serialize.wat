  (func $__serde_append (param $out i32) (param $text i32)
    (local $len i32) (local $data i32) (local $i i32)
    local.get $text i32.load local.set $len
    local.get $text i32.const 16 i32.add i32.load local.set $data
    block $done loop $copy
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $out local.get $data local.get $i i32.const 4 i32.mul i32.add i32.load
      call $vec_push_i32 drop
      local.get $i i32.const 1 i32.add local.set $i br $copy
    end end)

  (func $__serde_int (param $value i32) (result i32)
    (local $out i32) (local $v i64) (local $n i32) (local $i i32) (local $digit i32)
    i32.const 0 i32.const 0 call $vec_new_i32 local.set $out
    local.get $value i64.extend_i32_s local.set $v
    local.get $v i64.const 0 i64.lt_s
    if
      local.get $out i32.const 45 call $vec_push_i32 drop
      i64.const 0 local.get $v i64.sub local.set $v
    end
    local.get $v i64.eqz
    if local.get $out i32.const 48 call $vec_push_i32 drop local.get $out return end
    block $digits_done loop $digits
      local.get $v i64.eqz br_if $digits_done
      i32.const 512 local.get $n i32.add
      local.get $v i64.const 10 i64.rem_u i32.wrap_i64 i32.const 48 i32.add i32.store8
      local.get $v i64.const 10 i64.div_u local.set $v
      local.get $n i32.const 1 i32.add local.set $n br $digits
    end end
    local.get $n local.set $i
    block $copy_done loop $copy
      local.get $i i32.eqz br_if $copy_done
      local.get $i i32.const 1 i32.sub local.tee $i
      i32.const 512 i32.add i32.load8_u local.set $digit
      local.get $out local.get $digit call $vec_push_i32 drop
      br $copy
    end end
    local.get $out)

  (func $__serde_dec (param $value i32) (result i32)
    (local $out i32) (local $whole i32) (local $frac i32) (local $digits i32)
    (local $tmp i32) (local $i i32) (local $abs i64)
    i32.const 0 i32.const 0 call $vec_new_i32 local.set $out
    local.get $value i32.const 0 i32.lt_s
    if local.get $out i32.const 45 call $vec_push_i32 drop end
    local.get $value i64.extend_i32_s local.set $abs
    local.get $abs i64.const 0 i64.lt_s
    if i64.const 0 local.get $abs i64.sub local.set $abs end
    local.get $abs i64.const __DEC_SCALE__ i64.div_u i32.wrap_i64 local.set $whole
    local.get $whole call $__serde_int local.set $tmp
    local.get $out local.get $tmp call $__serde_append
    local.get $tmp call $rc_release_vec drop
    local.get $abs i64.const __DEC_SCALE__ i64.rem_u i32.wrap_i64 local.set $frac
    local.get $frac i32.eqz if local.get $out return end
    local.get $out i32.const 46 call $vec_push_i32 drop
    i32.const __DEC_DIGITS__ local.set $digits
    block $trim_done loop $trim
      local.get $frac i32.const 10 i32.rem_u i32.eqz
      local.get $digits i32.const 1 i32.gt_u i32.and i32.eqz br_if $trim_done
      local.get $frac i32.const 10 i32.div_u local.set $frac
      local.get $digits i32.const 1 i32.sub local.set $digits br $trim
    end end
    local.get $digits local.set $i
    block $frac_done loop $frac_digits
      local.get $i i32.eqz br_if $frac_done
      local.get $i i32.const 1 i32.sub local.tee $i
      i32.const 512 i32.add
      local.get $frac i32.const 10 i32.rem_u i32.const 48 i32.add i32.store8
      local.get $frac i32.const 10 i32.div_u local.set $frac br $frac_digits
    end end
    i32.const 0 local.set $i
    block $emit_done loop $emit
      local.get $i local.get $digits i32.ge_u br_if $emit_done
      local.get $out i32.const 512 local.get $i i32.add i32.load8_u call $vec_push_i32 drop
      local.get $i i32.const 1 i32.add local.set $i br $emit
    end end
    local.get $out)

  (func $__serde_string (param $value i32) (result i32)
    (local $out i32) (local $len i32) (local $data i32) (local $i i32) (local $c i32)
    i32.const 0 i32.const 0 call $vec_new_i32 local.set $out
    local.get $out i32.const 34 call $vec_push_i32 drop
    local.get $value i32.load local.set $len
    local.get $value i32.const 16 i32.add i32.load local.set $data
    block $done loop $chars
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $data local.get $i i32.const 4 i32.mul i32.add i32.load local.set $c
      local.get $c i32.const 92 i32.eq local.get $c i32.const 34 i32.eq i32.or
      if local.get $out i32.const 92 call $vec_push_i32 drop
      else
        local.get $c i32.const 10 i32.eq if local.get $out i32.const 92 call $vec_push_i32 drop i32.const 110 local.set $c end
        local.get $c i32.const 13 i32.eq if local.get $out i32.const 92 call $vec_push_i32 drop i32.const 114 local.set $c end
        local.get $c i32.const 9 i32.eq if local.get $out i32.const 92 call $vec_push_i32 drop i32.const 116 local.set $c end
        local.get $c i32.eqz if local.get $out i32.const 92 call $vec_push_i32 drop i32.const 48 local.set $c end
      end
      local.get $out local.get $c call $vec_push_i32 drop
      local.get $i i32.const 1 i32.add local.set $i br $chars
    end end
    local.get $out i32.const 34 call $vec_push_i32 drop
    local.get $out)
