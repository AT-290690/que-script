use crate::vm::Instruction::*;
#[allow(unused_macros)]
macro_rules! s {
    ($s:expr) => {
        $s.to_string()
    };
}
pub fn load_bytecode() -> Vec<crate::vm::Instruction> {
    vec![PushInt(1), PushInt(2), Add]
}
