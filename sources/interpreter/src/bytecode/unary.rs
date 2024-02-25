#![allow(clippy::redundant_closure_call)]

use super::{Instruction, Progression};
use crate::arg;
use crate::pop;
use crate::Context;

use anyhow::Context as AnyhowContext;
use parse::classfile::Resolvable;
use parse::pool::ConstantClass;
use runtime::error::Throwable;

use runtime::error::VMError;

use runtime::object::builtins::Class;

use runtime::object::numeric::Floating;
use runtime::object::numeric::FloatingType;
use runtime::object::numeric::Integral;
use runtime::object::numeric::IntegralType;

use runtime::object::value::RuntimeValue;
use runtime::vm::VM;
use support::descriptor::FieldType;

macro_rules! unop {
    // Generic value transformation
    ($ins: ident, $res_ty: ident, $res_trans: expr => $op: expr) => {
        #[derive(Debug)]
        pub struct $ins;

        impl Instruction for $ins {
            fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
                let val = arg!(ctx, "unary value" => $res_ty);

                let result = $op(val);
                ctx.operands.push($res_trans(result));

                Ok(Progression::Next)
            }
        }
    };
    // Generic duplicated value transformation
    (x2 $ins: ident, $res_ty: ident, $res_trans: expr => $op: expr) => {
        #[derive(Debug)]
        pub struct $ins;

        impl Instruction for $ins {
            fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
                let val = arg!(ctx, "unary value" => $res_ty);

                let result: $res_ty = $op(val);
                ctx.operands.push($res_trans(result));
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
                let val = arg!(ctx, "unary value" => $res_ty);

                let result: bool = $op(val);
                if (result) {
                    Ok(Progression::JumpRel(self.jump_to as i32))
                } else {
                    Ok(Progression::Next)
                }
            }
        }
    };
    ($ins: ident (int) => $op: expr) => {
      unop!($ins, i32, |result: i32| RuntimeValue::Integral(result.into()) => $op);
    };
    ($ins: ident (int cond) => $op: expr) => {
      unop!($ins, i32 => $op);
    };
    ($ins: ident (int => long) => $op: expr) => {
      unop!($ins, i32, |result: i64| RuntimeValue::Integral(result.into()) => $op);
    };
    ($ins: ident (int => float) => $op: expr) => {
      unop!($ins, i32, |result: f32| RuntimeValue::Floating(result.into()) => $op);
    };
    ($ins: ident (int => double) => $op: expr) => {
      unop!($ins, i32, |result: f64| RuntimeValue::Floating(result.into()) => $op);
    };
    ($ins: ident (long => int) => $op: expr) => {
      unop!($ins, i64, |result: i32| RuntimeValue::Integral(result.into()) => $op);
    };
    ($ins: ident (long => float) => $op: expr) => {
      unop!($ins, i64, |result: f32| RuntimeValue::Floating(result.into()) => $op);
    };
    ($ins: ident (long => double) => $op: expr) => {
      unop!($ins, i64, |result: f64| RuntimeValue::Floating(result.into()) => $op);
    };
    ($ins: ident (float => int) => $op: expr) => {
      unop!($ins, f32, |result: i32| RuntimeValue::Integral(result.into()) => $op);
    };
    ($ins: ident (float => double) => $op: expr) => {
      unop!($ins, f32, |result: f64| RuntimeValue::Floating(result.into()) => $op);
    };
    ($ins: ident (double => long) => $op: expr) => {
      unop!($ins, f64, |result: i64| RuntimeValue::Integral(result.into()) => $op);
    };
    ($ins: ident (double => int) => $op: expr) => {
      unop!($ins, f64, |result: i32| RuntimeValue::Integral(result.into()) => $op);
    };
    ($ins: ident (double => float) => $op: expr) => {
      unop!($ins, f64, |result: f32| RuntimeValue::Floating(result.into()) => $op);
    };
    ($ins: ident (long) => $op: expr) => {
      unop!(x2 $ins, i64, |result: i64| RuntimeValue::Integral(result.into()) => $op);
    };
    ($ins: ident (float) => $op: expr) => {
      unop!($ins, f32, |result: f32| RuntimeValue::Floating(result.into()) => $op);
    };
    ($ins: ident (double) => $op: expr) => {
      unop!($ins, f64, |result: f64| RuntimeValue::Floating(result.into()) => $op);
    };
}

// Negations
unop!(Ineg (int) => |val: Integral| {
    (val.value as i32).wrapping_neg()
});

unop!(Lneg (long) => |val: Integral| {
    val.value.wrapping_neg()
});

unop!(Fneg (float) => |val: Floating| {
    -(val.value as f32)
});

unop!(Dneg (double) => |val: Floating| {
    -val.value
});

// Conversions
unop!(I2l (int => long) => |val: Integral| {
    val.value as i32 as i64
});

unop!(I2d (int => double) => |val: Integral| {
    val.value as i32 as f64
});

unop!(I2c (int) => |val: Integral| {
    val.value as i16 as i32
});

unop!(I2b (int) => |val: Integral| {
    val.value as i8 as i32
});

unop!(I2f (int => float) => |val: Integral| {
    val.value as i32 as f32
});

unop!(F2i (float => int) => |val: Floating| {
    val.value as f32 as i32
});

unop!(F2d (float => double) => |val: Floating| {
    val.value as f32 as f64
});

unop!(D2l (double => long) => |val: Floating| {
    val.value as i64
});

unop!(D2i (double => int) => |val: Floating| {
    val.value as i32
});

unop!(D2f (double => float) => |val: Floating| {
    val.value as f32
});

unop!(L2i (long => int) => |val: Integral| {
    val.value as i32
});

unop!(L2f (long => float) => |val: Integral| {
    val.value as f32
});

unop!(L2d (long => double) => |val: Integral| {
    val.value as f64
});

// Zero comparisons
unop!(IfEq (int cond) => |val: Integral| {
    val.value == 0
});

unop!(IfNe (int cond) => |val: Integral| {
    val.value != 0
});

unop!(IfLt (int cond) => |val: Integral| {
    val.value < 0
});

unop!(IfLe (int cond) => |val: Integral| {
    val.value <= 0
});

unop!(IfGt (int cond) => |val: Integral| {
    val.value > 0
});

unop!(IfGe (int cond) => |val: Integral| {
    val.value >= 0
});

// If[Not]null
#[derive(Debug)]
pub struct IfNull {
    pub(crate) jump_to: i16,
}

impl Instruction for IfNull {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let val = pop!(ctx);
        let val = val.as_object().context("not an object")?;

        if val.is_null() {
            Ok(Progression::JumpRel(self.jump_to as i32))
        } else {
            Ok(Progression::Next)
        }
    }
}

#[derive(Debug)]
pub struct IfNotNull {
    pub(crate) jump_to: i16,
}

impl Instruction for IfNotNull {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let val = pop!(ctx);
        let val = val.as_object().context("not an object")?;

        if val.is_null() {
            Ok(Progression::Next)
        } else {
            Ok(Progression::JumpRel(self.jump_to as i32))
        }
    }
}

#[derive(Debug)]
pub struct InstanceOf {
    pub(crate) type_index: u16,
}

impl Instruction for InstanceOf {
    fn handle(&self, vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let val = pop!(ctx);
        let val = val.as_object().context("not an object")?;

        // If objectref is null, the instanceof instruction pushes an int result of 0 as an int onto the operand stack.
        if val.is_null() {
            ctx.operands.push(RuntimeValue::Integral((0_i32).into()));
            return Ok(Progression::Next);
        }

        // TODO: Support interface type checking etc
        //  The run-time constant pool entry at the index must be a symbolic reference to a class, array, or interface type
        let ty: ConstantClass = ctx
            .class
            .unwrap_ref()
            .class_file()
            .constant_pool
            .address(self.type_index)
            .resolve();

        let ty_class_name = ty.name.resolve().string();
        let ty_class_name = FieldType::parse(ty_class_name.clone())
            .or_else(|_| FieldType::parse(format!("L{};", ty_class_name)))?;
        let ty_class = vm.class_loader().for_name(ty_class_name)?;

        let class = val.unwrap_ref().class.clone();

        if Class::can_assign(class, ty_class) {
            ctx.operands.push(RuntimeValue::Integral(1_i32.into()))
        } else {
            ctx.operands.push(RuntimeValue::Integral(0_i32.into()))
        }

        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct CheckCast {
    pub(crate) type_index: u16,
}

impl Instruction for CheckCast {
    fn handle(&self, vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let _val = pop!(ctx);
        let val = _val.as_object().context("not an object")?;

        // If objectref is null, then the operand stack is unchanged
        if val.is_null() {
            // NOTE: we need to push the val back
            ctx.operands.push(_val);
            return Ok(Progression::Next);
        }

        //  The run-time constant pool entry at the index must be a symbolic reference to a class, array, or interface type
        let (other_class, other_class_name) = {
            let other: ConstantClass = ctx
                .class
                .unwrap_ref()
                .class_file()
                .constant_pool
                .address(self.type_index)
                .resolve();

            let other_class_name = other.name.resolve().string();
            let other_class_name = FieldType::parse(other_class_name.clone())
                .or_else(|_| FieldType::parse(format!("L{};", other_class_name)))?;

            (
                vm.class_loader().for_name(other_class_name.clone())?,
                other_class_name,
            )
        };

        let val_class = &val.unwrap_ref().class;
        {
            let _from = val_class.unwrap_ref().name().clone();
            let _to = other_class_name.to_string();
        }

        if Class::can_assign(val_class.clone(), other_class) {
            ctx.operands.push(RuntimeValue::Object(val.clone()));
        } else {
            let from = val_class.unwrap_ref().name().clone();
            let to = other_class_name.to_string();

            return Ok(Progression::Throw(
                vm.try_make_error(VMError::ClassCastException { from, to })?,
            ));
        }

        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct Pop {
    pub(crate) amount: u8,
}

impl Instruction for Pop {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        for _ in 0..self.amount {
            pop!(ctx);
        }

        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct Iinc {
    pub(crate) index: u8,
    pub(crate) constant: i8,
}

impl Instruction for Iinc {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let local = ctx
            .locals
            .get_mut(self.index as usize)
            .context(format!("no local @ {}", self.index))?;

        let int = local.as_integral_mut().context("not an int")?;
        int.value += self.constant as i64;

        Ok(Progression::Next)
    }
}
