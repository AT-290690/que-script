use crate::vm::Instruction::*;
pub fn load_bytecode() -> Vec<crate::vm::Instruction> {
vec![PushInt(1),PushInt(2),Add,]
}