///! This module contains functions for producing a higher level view disassembler of Brillig.
use super::BrilligBinaryOp;
use crate::brillig::brillig_ir::{ReservedRegisters, BRILLIG_MEMORY_ADDRESSING_BIT_SIZE};
use acvm::acir::brillig_vm::{
    BinaryFieldOp, BinaryIntOp, BlackBoxOp, HeapArray, HeapVector, RegisterIndex, RegisterOrMemory,
    Value,
};

/// Controls whether debug traces are enabled
const ENABLE_DEBUG_TRACE: bool = true;

/// Trait for converting values into debug-friendly strings.
trait DebugToString {
    fn debug_to_string(&self) -> String;
}

macro_rules! default_to_string_impl {
    ($($t:ty)*) => ($(
        impl DebugToString for $t {
            fn debug_to_string(&self) -> String {
                self.to_string()
            }
        }
    )*)
}

default_to_string_impl! { str usize u32 }

impl DebugToString for RegisterIndex {
    fn debug_to_string(&self) -> String {
        if *self == ReservedRegisters::stack_pointer() {
            "Stack".into()
        } else {
            format!("R{}", self.to_usize())
        }
    }
}

impl DebugToString for HeapArray {
    fn debug_to_string(&self) -> String {
        format!("{}[0..{}]", self.pointer.debug_to_string(), self.size)
    }
}

impl DebugToString for HeapVector {
    fn debug_to_string(&self) -> String {
        format!("{}[0..{}]", self.pointer.debug_to_string(), self.size.debug_to_string())
    }
}

impl DebugToString for BinaryFieldOp {
    fn debug_to_string(&self) -> String {
        match self {
            BinaryFieldOp::Add => "f+".into(),
            BinaryFieldOp::Sub => "f-".into(),
            BinaryFieldOp::Mul => "f*".into(),
            BinaryFieldOp::Div => "f/".into(),
            BinaryFieldOp::Equals => "f==".into(),
        }
    }
}

impl DebugToString for BinaryIntOp {
    fn debug_to_string(&self) -> String {
        match self {
            BinaryIntOp::Add => "+".into(),
            BinaryIntOp::Sub => "-".into(),
            BinaryIntOp::Mul => "*".into(),
            BinaryIntOp::Equals => "==".into(),
            BinaryIntOp::SignedDiv => "/".into(),
            BinaryIntOp::UnsignedDiv => "//".into(),
            BinaryIntOp::LessThan => "<".into(),
            BinaryIntOp::LessThanEquals => "<=".into(),
            BinaryIntOp::And => "&&".into(),
            BinaryIntOp::Or => "||".into(),
            BinaryIntOp::Xor => "^".into(),
            BinaryIntOp::Shl => "<<".into(),
            BinaryIntOp::Shr => ">>".into(),
        }
    }
}

impl DebugToString for BrilligBinaryOp {
    fn debug_to_string(&self) -> String {
        match self {
            BrilligBinaryOp::Field { op } => op.debug_to_string(),
            BrilligBinaryOp::Integer { op, bit_size } => {
                // rationale: if there's >= 64 bits, we should not bother with this detail
                if *bit_size >= BRILLIG_MEMORY_ADDRESSING_BIT_SIZE {
                    op.debug_to_string()
                } else {
                    format!("i{}::{}", bit_size, op.debug_to_string())
                }
            }
            BrilligBinaryOp::Modulo { is_signed_integer, bit_size } => {
                let op = if *is_signed_integer { "%" } else { "%%" };
                // rationale: if there's >= 64 bits, we should not bother with this detail
                if *bit_size >= BRILLIG_MEMORY_ADDRESSING_BIT_SIZE {
                    op.into()
                } else {
                    format!("{}:{}", op, bit_size)
                }
            }
        }
    }
}

impl DebugToString for Value {
    fn debug_to_string(&self) -> String {
        self.to_usize().to_string()
    }
}

impl DebugToString for RegisterOrMemory {
    fn debug_to_string(&self) -> String {
        match self {
            RegisterOrMemory::RegisterIndex(index) => index.debug_to_string(),
            RegisterOrMemory::HeapArray(heap_array) => heap_array.debug_to_string(),
            RegisterOrMemory::HeapVector(vector) => vector.debug_to_string(),
        }
    }
}

impl<T: DebugToString> DebugToString for [T] {
    fn debug_to_string(&self) -> String {
        self.iter().map(|x| x.debug_to_string()).collect::<Vec<String>>().join(", ")
    }
}

macro_rules! debug_println {
    ( $literal:expr ) => {
        if ENABLE_DEBUG_TRACE {
            println!("{}", $literal);
        }
    };
    ( $format_message:expr, $( $x:expr ),* ) => {
        if ENABLE_DEBUG_TRACE {
            println!($format_message, $( $x.debug_to_string(), )*)
        }
    };
}

/// Emits brillig bytecode to jump to a trap condition if `condition`
/// is false.
pub(crate) fn constrain_instruction(condition: RegisterIndex) {
    debug_println!("  ASSERT {} != 0", condition);
}

/// Processes a return instruction.
pub(crate) fn return_instruction(return_registers: &[RegisterIndex]) {
    let registers_string = return_registers
        .iter()
        .map(RegisterIndex::debug_to_string)
        .collect::<Vec<String>>()
        .join(", ");

    debug_println!("  // return {};", registers_string);
}

/// Emits a `mov` instruction.
pub(crate) fn mov_instruction(destination: RegisterIndex, source: RegisterIndex) {
    debug_println!("  MOV {}, {}", destination, source);
}

/// Processes a binary instruction according `operation`.
pub(crate) fn binary_instruction(
    lhs: RegisterIndex,
    rhs: RegisterIndex,
    result: RegisterIndex,
    operation: BrilligBinaryOp,
) {
    debug_println!("  {} = {} {} {}", result, lhs, operation, rhs);
}

/// Stores the value of `constant` in the `result` register
pub(crate) fn const_instruction(result: RegisterIndex, constant: Value) {
    debug_println!("  CONST {} = {}", result, constant);
}

/// Processes a not instruction. Append with "_" as this is a high-level instruction.
pub(crate) fn not_instruction(condition: RegisterIndex, bit_size: u32, result: RegisterIndex) {
    debug_println!("  i{}_NOT {} = !{}", bit_size, result, condition);
}

/// Processes a foreign call instruction.
pub(crate) fn foreign_call_instruction(
    func_name: String,
    inputs: &[RegisterOrMemory],
    outputs: &[RegisterOrMemory],
) {
    debug_println!("  FOREIGN_CALL {} ({}) => {}", func_name, inputs, outputs);
}

/// Emits a load instruction
pub(crate) fn load_instruction(destination: RegisterIndex, source_pointer: RegisterIndex) {
    debug_println!("  LOAD {} = *{}", destination, source_pointer);
}

/// Emits a store instruction
pub(crate) fn store_instruction(destination_pointer: RegisterIndex, source: RegisterIndex) {
    debug_println!("  STORE *{} = {}", destination_pointer, source);
}

/// Emits a stop instruction
pub(crate) fn stop_instruction() {
    debug_println!("  STOP");
}

/// Debug function for allocate_array_instruction
pub(crate) fn allocate_array_instruction(
    pointer_register: RegisterIndex,
    size_register: RegisterIndex,
) {
    debug_println!("  ALLOCATE_ARRAY {} SIZE {}", pointer_register, size_register);
}

/// Debug function for array_get
pub(crate) fn array_get(array_ptr: RegisterIndex, index: RegisterIndex, result: RegisterIndex) {
    debug_println!("  ARRAY_GET {}[{}] -> {}", array_ptr, index, result);
}

/// Debug function for array_set
pub(crate) fn array_set(array_ptr: RegisterIndex, index: RegisterIndex, value: RegisterIndex) {
    debug_println!("  ARRAY_SET {}[{}] = {}", array_ptr, index, value);
}

/// Debug function for copy_array_instruction
pub(crate) fn copy_array_instruction(
    source: RegisterIndex,
    destination: RegisterIndex,
    num_elements_register: RegisterIndex,
) {
    debug_println!(
        "  COPY_ARRAY {} -> {} ({} ELEMENTS)",
        source,
        destination,
        num_elements_register
    );
}

/// Debug function for enter_context
pub(crate) fn enter_context(label: String) {
    if !label.ends_with("-b0") {
        // Hacky readability fix: don't print labels e.g. f1 then f1-b0 one after another, they mean the same thing
        debug_println!("{}:", label);
    }
}

/// Debug function for jump_instruction
pub(crate) fn jump_instruction(target_label: String) {
    debug_println!("  JUMP_TO {}", target_label);
}

/// Debug function for jump_if_instruction
pub(crate) fn jump_if_instruction<T: ToString>(condition: RegisterIndex, target_label: T) {
    debug_println!("  JUMP_IF {} TO {}", condition, target_label.to_string());
}

/// Debug function for cast_instruction
pub(crate) fn cast_instruction(
    destination: RegisterIndex,
    source: RegisterIndex,
    target_bit_size: u32,
) {
    debug_println!("  CAST {} FROM {} TO {} BITS", destination, source, target_bit_size);
}

/// Debug function for black_box_op
pub(crate) fn black_box_op_instruction(op: BlackBoxOp) {
    match op {
        BlackBoxOp::Sha256 { message, output } => {
            debug_println!("  SHA256 {} -> {}", message, output);
        }
        BlackBoxOp::Keccak256 { message, output } => {
            debug_println!("  KECCAK256 {} -> {}", message, output);
        }
        BlackBoxOp::Blake2s { message, output } => {
            debug_println!("  BLAKE2S {} -> {}", message, output);
        }
        BlackBoxOp::HashToField128Security { message, output } => {
            debug_println!("  HASH_TO_FIELD_128_SECURITY {} -> {}", message, output);
        }
        BlackBoxOp::EcdsaSecp256k1 {
            hashed_msg,
            public_key_x,
            public_key_y,
            signature,
            result,
        } => {
            debug_println!(
                "  ECDSA_SECP256K1 {} {} {} {} -> {}",
                hashed_msg,
                public_key_x,
                public_key_y,
                signature,
                result
            );
        }
    }
}

/// Debug function for cast_instruction
pub(crate) fn add_external_call_instruction(func_label: String) {
    debug_println!("  CALL {}", func_label);
}
