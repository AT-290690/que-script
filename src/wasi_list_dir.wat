  (func $__wasi_decode_name (param $ptr i32) (param $len i32) (result i32)
    (local $out i32) (local $i i32) (local $b i32) (local $c i32) (local $need i32)
    i32.const 0 i32.const 0 call $vec_new_i32 local.set $out
    block $done loop $scan
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $ptr local.get $i i32.add i32.load8_u local.set $b
      local.get $i i32.const 1 i32.add local.set $i
      local.get $need i32.eqz
      if
        local.get $b i32.const 128 i32.lt_u
        if local.get $b local.set $c
        else
          local.get $b i32.const 224 i32.and i32.const 192 i32.eq
          if local.get $b i32.const 31 i32.and local.set $c i32.const 1 local.set $need
          else
            local.get $b i32.const 240 i32.and i32.const 224 i32.eq
            if local.get $b i32.const 15 i32.and local.set $c i32.const 2 local.set $need
            else local.get $b i32.const 7 i32.and local.set $c i32.const 3 local.set $need
            end
          end
        end
      else
        local.get $c i32.const 6 i32.shl
        local.get $b i32.const 63 i32.and i32.or local.set $c
        local.get $need i32.const 1 i32.sub local.set $need
      end
      local.get $need i32.eqz
      if local.get $out local.get $c call $vec_push_i32 drop end
      br $scan
    end end
    local.get $need if unreachable end
    local.get $out)

  (func $__wasi_string_gt (param $a i32) (param $b i32) (result i32)
    (local $al i32) (local $bl i32) (local $n i32) (local $i i32)
    (local $ad i32) (local $bd i32) (local $ac i32) (local $bc i32)
    local.get $a i32.load local.set $al
    local.get $b i32.load local.set $bl
    local.get $al local.get $bl i32.lt_u
    if (result i32) local.get $al else local.get $bl end local.set $n
    local.get $a i32.const 16 i32.add i32.load local.set $ad
    local.get $b i32.const 16 i32.add i32.load local.set $bd
    block $same loop $chars
      local.get $i local.get $n i32.ge_u br_if $same
      local.get $ad local.get $i i32.const 4 i32.mul i32.add i32.load local.set $ac
      local.get $bd local.get $i i32.const 4 i32.mul i32.add i32.load local.set $bc
      local.get $ac local.get $bc i32.gt_u if i32.const 1 return end
      local.get $ac local.get $bc i32.lt_u if i32.const 0 return end
      local.get $i i32.const 1 i32.add local.set $i br $chars
    end end
    local.get $al local.get $bl i32.gt_u)

  (func $v_list_dash_dir_bang_ (param $path i32) (result i32)
    (local $plen i32) (local $fd i32) (local $used i32) (local $pos i32)
    (local $nlen i32) (local $nameptr i32) (local $name i32) (local $cookie i64)
    (local $out i32) (local $n i32) (local $i i32) (local $j i32)
    (local $data i32) (local $a i32) (local $b i32)
    local.get $path call $__wasi_encode_path local.set $plen
    i32.const 3 i32.const 0 i32.const 1024 local.get $plen i32.const 2
    i64.const 16384 i64.const 0 i32.const 0 i32.const 16 call $__wasi_path_open
    if unreachable end
    i32.const 16 i32.load local.set $fd
    i32.const 0 i32.const 1 call $vec_new_i32 local.set $out
    block $read_done loop $read
      local.get $fd i32.const 32768 i32.const 30000 local.get $cookie i32.const 24
      call $__wasi_fd_readdir if unreachable end
      i32.const 24 i32.load local.tee $used i32.eqz br_if $read_done
      i32.const 0 local.set $pos
      block $page_done loop $entries
        local.get $pos i32.const 24 i32.add local.get $used i32.gt_u br_if $page_done
        i32.const 32768 local.get $pos i32.add i64.load local.set $cookie
        i32.const 32784 local.get $pos i32.add i32.load local.set $nlen
        i32.const 32792 local.get $pos i32.add local.set $nameptr
        local.get $nlen i32.const 1 i32.eq
        local.get $nameptr i32.load8_u i32.const 46 i32.eq i32.and
        local.get $nlen i32.const 2 i32.eq
        local.get $nameptr i32.load16_u i32.const 11822 i32.eq i32.and i32.or
        if
        else
          local.get $nameptr local.get $nlen call $__wasi_decode_name local.set $name
          local.get $out local.get $name call $vec_push_i32 drop
          local.get $name call $rc_release_vec drop
        end
        local.get $pos i32.const 24 i32.add local.get $nlen i32.add local.set $pos
        br $entries
      end end
      br $read
    end end
    local.get $fd call $__wasi_fd_close drop
    local.get $out i32.load local.set $n
    i32.const 0 local.set $i
    block $sorted loop $passes
      local.get $i local.get $n i32.ge_u br_if $sorted
      i32.const 0 local.set $j
      block $pass_done loop $pairs
        local.get $j i32.const 1 i32.add local.get $n local.get $i i32.sub i32.ge_u br_if $pass_done
        local.get $out i32.const 16 i32.add i32.load local.set $data
        local.get $data local.get $j i32.const 4 i32.mul i32.add i32.load local.set $a
        local.get $data local.get $j i32.const 1 i32.add i32.const 4 i32.mul i32.add i32.load local.set $b
        local.get $a local.get $b call $__wasi_string_gt
        if
          local.get $data local.get $j i32.const 4 i32.mul i32.add local.get $b i32.store
          local.get $data local.get $j i32.const 1 i32.add i32.const 4 i32.mul i32.add local.get $a i32.store
        end
        local.get $j i32.const 1 i32.add local.set $j br $pairs
      end end
      local.get $i i32.const 1 i32.add local.set $i br $passes
    end end
    local.get $out)
