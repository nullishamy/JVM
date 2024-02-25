#![allow(clippy::redundant_closure_call)]

use std::cmp::Ordering;

use super::{Instruction, Progression};
use crate::arg;
use crate::pop;
use crate::Context;
use anyhow::Context as AnyhowContext;
use runtime::error::Throwable;
use runtime::vm::VM;

use runtime::object::numeric::Floating;
use runtime::object::numeric::FloatingType;
use runtime::object::numeric::Integral;
use runtime::object::numeric::IntegralType;
use runtime::object::value::RuntimeValue;

macro_rules! binop {
    // Generic value transformation
    ($ins: ident, $lhs: ident, $rhs: ident, $res: ty, $res_trans: expr => $op: expr) => {
        #[derive(Debug)]
        pub struct $ins;

        impl Instruction for $ins {
            fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
                let rhs = arg!(ctx, "rhs" => $rhs);
                let lhs = arg!(ctx, "lhs" => $lhs);

                let result: $res = $op(lhs, rhs);
                ctx.operands.push($res_trans(result));

                Ok(Progression::Next)
            }
        }
    };
    // Generic conditional transformation
    ($ins: ident, $res_ty: ident => $op: expr) => {
        #[derive(Debug)]
        pub struct $ins {
            pub(crate) jump_to: i16
        }

        impl Instruction for $ins {
            fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
                let rhs = arg!(ctx, "rhs" => $res_ty);
                let lhs = arg!(ctx, "lhs" => $res_ty);

                let result: bool = $op(lhs, rhs);
                if (result) {
                    Ok(Progression::JumpRel(self.jump_to as i32))
                } else {
                    Ok(Progression::Next)
                }
            }
        }
    };
    ($ins: ident (int) => $op: expr) => {
      binop!($ins, i32, i32, i32, |result: i32| RuntimeValue::Integral(result.into()) => $op);
    };
    ($ins: ident (long) => $op: expr) => {
      binop!($ins, i64, i64, i64, |result: i64| RuntimeValue::Integral(result.into()) => $op);
    };
    ($ins: ident (long => int) => $op: expr) => {
      binop!($ins, i64, i64, i32, |result: i32| RuntimeValue::Integral(result.into()) => $op);
    };
    ($ins: ident (float => int) => $op: expr) => {
      binop!($ins, f32, f32, i32, |result: i32| RuntimeValue::Integral(result.into()) => $op);
    };
    ($ins: ident (double => int) => $op: expr) => {
      binop!($ins, f64, f64, i32, |result: i32| RuntimeValue::Integral(result.into()) => $op);
    };
    ($ins: ident (long bitwise) => $op: expr) => {
      binop!($ins, i64, i32, i64, |result: i64| RuntimeValue::Integral(result.into()) => $op);
    };
    ($ins: ident (float) => $op: expr) => {
      binop!($ins, f32, f32, f32, |result: f32| RuntimeValue::Floating(result.into()) => $op);
    };
    ($ins: ident (double) => $op: expr) => {
      binop!($ins, f64, f64, f64, |result: f64| RuntimeValue::Floating(result.into()) => $op);
    };
    ($ins: ident (int cond) => $op: expr) => {
      binop!($ins, i32 => $op);
    };
    ($ins: ident (Object cond) => $op: expr) => {
      binop!($ins, Object => $op);
    };
}

// Binary (int)
binop!(Isub (int) => |lhs: Integral, rhs: Integral| {
    (lhs.value as i32).wrapping_sub(rhs.value as i32)
});

binop!(Iadd (int) => |lhs: Integral, rhs: Integral| {
    (lhs.value as i32).wrapping_add(rhs.value as i32)
});

binop!(Imul (int) => |lhs: Integral, rhs: Integral| {
    (lhs.value as i32).wrapping_mul(rhs.value as i32)
});

binop!(Idiv (int) => |lhs: Integral, rhs: Integral| {
    (lhs.value as i32).wrapping_div(rhs.value as i32)
});

binop!(Irem (int) => |lhs: Integral, rhs: Integral| {
    (lhs.value as i32) % (rhs.value as i32)
});

// Binary (long)
binop!(Lsub (long) => |lhs: Integral, rhs: Integral| {
    lhs.value.wrapping_sub(rhs.value)
});

binop!(Ladd (long) => |lhs: Integral, rhs: Integral| {
    lhs.value.wrapping_add(rhs.value)
});

binop!(Lmul (long) => |lhs: Integral, rhs: Integral| {
    lhs.value.wrapping_mul(rhs.value)
});

binop!(Ldiv (long) => |lhs: Integral, rhs: Integral| {
    lhs.value.wrapping_div(rhs.value)
});

binop!(Lrem (long) => |lhs: Integral, rhs: Integral| {
    lhs.value % rhs.value
});

// Binary (float)
binop!(Fadd (float) => |lhs: Floating, rhs: Floating| {
    (lhs.value as f32) + (rhs.value as f32)
});

binop!(Fsub (float) => |lhs: Floating, rhs: Floating| {
    (lhs.value as f32) - (rhs.value as f32)
});

binop!(Fmul (float) => |lhs: Floating, rhs: Floating| {
    (lhs.value as f32) * (rhs.value as f32)
});

binop!(Fdiv (float) => |lhs: Floating, rhs: Floating| {
    (lhs.value as f32) / (rhs.value as f32)
});

binop!(Frem (float) => |lhs: Floating, rhs: Floating| {
    (lhs.value as f32) % (rhs.value as f32)
});

// Binary (double)
binop!(Dadd (double) => |lhs: Floating, rhs: Floating| {
    lhs.value + rhs.value
});

binop!(Dsub (double) => |lhs: Floating, rhs: Floating| {
    lhs.value - rhs.value
});

binop!(Dmul (double) => |lhs: Floating, rhs: Floating| {
    lhs.value * rhs.value
});

binop!(Ddiv (double) => |lhs: Floating, rhs: Floating| {
    lhs.value / rhs.value
});

binop!(Drem (double) => |lhs: Floating, rhs: Floating| {
    lhs.value % rhs.value
});

// Conditional (int)
binop!(Ieq (int cond) => |lhs: Integral, rhs: Integral| {
    lhs.value == rhs.value
});

binop!(Ine (int cond) => |lhs: Integral, rhs: Integral| {
    lhs.value != rhs.value
});

binop!(Ile (int cond) => |lhs: Integral, rhs: Integral| {
    (lhs.value as i32) <= (rhs.value as i32)
});

binop!(Ige (int cond) => |lhs: Integral, rhs: Integral| {
    (lhs.value as i32) >= (rhs.value as i32)
});

binop!(Igt (int cond) => |lhs: Integral, rhs: Integral| {
    (lhs.value as i32) > (rhs.value as i32)
});

binop!(Ilt (int cond) => |lhs: Integral, rhs: Integral| {
    (lhs.value as i32) < (rhs.value as i32)
});

// Bitwise
binop!(Lshl (long bitwise) => |lhs: Integral, rhs: Integral| {
    lhs.value.wrapping_shl(rhs.value as u32)
});

binop!(Lshr (long bitwise) => |lhs: Integral, rhs: Integral| {
    lhs.value.wrapping_shr(rhs.value as u32)
});

binop!(Land (long) => |lhs: Integral, rhs: Integral| {
    lhs.value & rhs.value
});

binop!(Lushr (long bitwise) => |lhs: Integral, rhs: Integral| {
    ((lhs.value as u64) >> (rhs.value as u64)) as i64
});

binop!(Ishl (int) => |lhs: Integral, rhs: Integral| {
    (lhs.value as i32).wrapping_shl(rhs.value as u32)
});

binop!(Ishr (int) => |lhs: Integral, rhs: Integral| {
    (lhs.value as i32).wrapping_shr(rhs.value as u32)
});

binop!(Iushr (int) => |lhs: Integral, rhs: Integral| {
    ((lhs.value as u32) >> (rhs.value as u32)) as i32
});

binop!(Iand (int) => |lhs: Integral, rhs: Integral| {
    (lhs.value as i32) & (rhs.value as i32)
});

binop!(Ior (int) => |lhs: Integral, rhs: Integral| {
    (lhs.value as i32) | (rhs.value as i32)
});

binop!(Lor (long) => |lhs: Integral, rhs: Integral| {
    lhs.value | rhs.value
});

binop!(Ixor (int) => |lhs: Integral, rhs: Integral| {
    (lhs.value as i32) ^ (rhs.value as i32)
});

binop!(Lxor (long) => |lhs: Integral, rhs: Integral| {
    lhs.value ^ rhs.value
});

// Comparisons
binop!(Lcmp (long => int) => |lhs: Integral, rhs: Integral| {
    let lhs = lhs.value;
    let rhs = rhs.value;

    match lhs.cmp(&rhs) {
        Ordering::Less => {
            // If value1 is less than value2, the int value -1 is pushed onto the operand stack.
            -1
        },
        Ordering::Equal => {
            // If value1 is equal to value2, the int value 0 is pushed onto the operand stack.
            0
        },
        Ordering::Greater => {
            // If value1 is greater than value2, the int value 1 is pushed onto the operand stack.
            1
        },
    }
});

binop!(Fcmpg (float => int) => |lhs: Floating, rhs: Floating| {
    let lhs = lhs.value as f32;
    let rhs = rhs.value as f32;

    // Otherwise, at least one of value1 or value2 is NaN. The fcmpg instruction pushes the int value 1 onto the operand stack
    if lhs.is_nan() || rhs.is_nan() {
        return 1;
    }

    // Use partial_cmp because it seems to fit the spec
    match lhs.partial_cmp(&rhs).expect("don't know how to handle cases where ordering is not available") {
        Ordering::Less => {
            // If value1 is less than value2, the int value -1 is pushed onto the operand stack.
            -1
        },
        Ordering::Equal => {
            // If value1 is equal to value2, the int value 0 is pushed onto the operand stack.
            0
        },
        Ordering::Greater => {
            // If value1 is greater than value2, the int value 1 is pushed onto the operand stack.
            1
        },
    }
});

binop!(Fcmpl (float => int) => |lhs: Floating, rhs: Floating| {
    let lhs = lhs.value as f32;
    let rhs = rhs.value as f32;

    // Otherwise, at least one of value1 or value2 is NaN. ... the fcmpl instruction pushes the int value -1 onto the operand stack.
    if lhs.is_nan() || rhs.is_nan() {
        return -1;
    }

    // Use partial_cmp because it seems to fit the spec
    match lhs.partial_cmp(&rhs).expect("don't know how to handle cases where ordering is not available") {
        Ordering::Less => {
            // If value1 is less than value2, the int value -1 is pushed onto the operand stack.
            -1
        },
        Ordering::Equal => {
            // If value1 is equal to value2, the int value 0 is pushed onto the operand stack.
            0
        },
        Ordering::Greater => {
            // If value1 is greater than value2, the int value 1 is pushed onto the operand stack.
            1
        },
    }
});

binop!(Dcmpg (double => int) => |lhs: Floating, rhs: Floating| {
    let lhs = lhs.value;
    let rhs = rhs.value;

    // Otherwise, at least one of value1 or value2 is NaN. The fcmpg instruction pushes the int value 1 onto the operand stack
    if lhs.is_nan() || rhs.is_nan() {
        return 1;
    }

    // Use partial_cmp because it seems to fit the spec
    match lhs.partial_cmp(&rhs).expect("don't know how to handle cases where ordering is not available") {
        Ordering::Less => {
            // If value1 is less than value2, the int value -1 is pushed onto the operand stack.
            -1
        },
        Ordering::Equal => {
            // If value1 is equal to value2, the int value 0 is pushed onto the operand stack.
            0
        },
        Ordering::Greater => {
            // If value1 is greater than value2, the int value 1 is pushed onto the operand stack.
            1
        },
    }
});

binop!(Dcmpl (double => int) => |lhs: Floating, rhs: Floating| {
    let lhs = lhs.value;
    let rhs = rhs.value;

    // Otherwise, at least one of value1 or value2 is NaN. ... the fcmpl instruction pushes the int value -1 onto the operand stack.
    if lhs.is_nan() || rhs.is_nan() {
        return -1;
    }

    // Use partial_cmp because it seems to fit the spec
    match lhs.partial_cmp(&rhs).expect("don't know how to handle cases where ordering is not available") {
        Ordering::Less => {
            // If value1 is less than value2, the int value -1 is pushed onto the operand stack.
            -1
        },
        Ordering::Equal => {
            // If value1 is equal to value2, the int value 0 is pushed onto the operand stack.
            0
        },
        Ordering::Greater => {
            // If value1 is greater than value2, the int value 1 is pushed onto the operand stack.
            1
        },
    }
});

// Referential equality
#[derive(Debug)]
pub struct IfRefEq {
    pub(crate) jump_to: i16,
}

impl Instruction for IfRefEq {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let rhs = pop!(ctx);
        let lhs = pop!(ctx);

        let equal = lhs.hash_code() == rhs.hash_code();
        if equal {
            Ok(Progression::JumpRel(self.jump_to as i32))
        } else {
            Ok(Progression::Next)
        }
    }
}

#[derive(Debug)]
pub struct IfRefNe {
    pub(crate) jump_to: i16,
}

impl Instruction for IfRefNe {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let rhs = pop!(ctx);
        let lhs = pop!(ctx);

        let not_equal = lhs.hash_code() != rhs.hash_code();
        if not_equal {
            Ok(Progression::JumpRel(self.jump_to as i32))
        } else {
            Ok(Progression::Next)
        }
    }
}
