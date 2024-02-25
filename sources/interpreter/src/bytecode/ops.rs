use super::{Instruction, Progression};
use crate::arg;
use crate::pop;
use crate::Context;

use anyhow::Context as AnyhowContext;
use parse::{classfile::Resolvable, pool::ConstantEntry};
use runtime::error::Throwable;
use runtime::error::VMError;
use runtime::internal;
use runtime::object::builtins::Array;
use runtime::object::builtins::ArrayPrimitive;
use runtime::object::builtins::Class;
use runtime::object::builtins::Object;

use runtime::object::layout::types;
use runtime::object::layout::types::Bool;
use runtime::object::layout::types::Byte;
use runtime::object::layout::types::Char;
use runtime::object::layout::types::Double;
use runtime::object::layout::types::Float;
use runtime::object::layout::types::Int;
use runtime::object::layout::types::Long;
use runtime::object::layout::types::Short;

use runtime::object::mem::RefTo;

use runtime::object::numeric::IntegralType;

use runtime::object::value::RuntimeValue;
use runtime::vm::VM;

pub use super::binary::*;
pub use super::invoke::*;
pub use super::load_store::*;
pub use super::unary::*;

#[macro_export]
macro_rules! pop {
    ($ctx: expr) => {
        $ctx.operands
            .pop()
            .context("no value to pop from the operand stack")?
    };
}

#[macro_export]
macro_rules! arg {
    ($ctx: expr, $side: expr => i32) => {{
        let val = pop!($ctx);

        let val = val
            .as_integral()
            .context(format!("{} was not an integral", $side))?;
        if val.ty != IntegralType::Int {
            return Err(runtime::internal!(format!(
                "{} was not an int, got {:#?}",
                $side, val
            )));
        }

        val.clone()
    }};
    ($ctx: expr, $side: expr => i64) => {{
        let val = pop!($ctx);

        let val = val
            .as_integral()
            .context(format!("{} was not an integral", $side))?;
        if val.ty != IntegralType::Long {
            return Err(runtime::internal!(format!(
                "{} was not a long, got {:#?}",
                $side, val
            )));
        }

        val.clone()
    }};
    ($ctx: expr, $side: expr => f32) => {{
        let val = pop!($ctx);

        let val = val
            .as_floating()
            .context(format!("{} was not a floating", $side))?;
        if val.ty != FloatingType::Float {
            return Err(runtime::internal!(format!("{} was not a float", $side)));
        }

        val.clone()
    }};
    ($ctx: expr, $side: expr => f64) => {{
        let val = pop!($ctx);

        let val = val
            .as_floating()
            .context(format!("{} was not a floating", $side))?;
        if val.ty != FloatingType::Double {
            return Err(runtime::internal!(format!("{} was not a double", $side)));
        }

        val.clone()
    }};
    ($ctx: expr, $side: expr => Object) => {{
        let val = pop!($ctx);

        let val = val
            .as_object()
            .context(format!("{} was not an object", $side))?;
        val.clone()
    }};
    ($ctx: expr, $side: expr => Array<$comp: ty>) => {{
        let val = pop!($ctx);

        let val = val
            .as_object()
            .context(format!("{} was not an object", $side))?;

        unsafe { val.cast::<Array<$comp>>().clone() }
    }};
}

#[derive(Debug)]
pub struct Nop;
impl Instruction for Nop {}

#[derive(Debug)]
pub struct VoidReturn;
impl Instruction for VoidReturn {
    fn handle(&self, _vm: &VM, _ctx: &mut Context) -> Result<Progression, Throwable> {
        Ok(Progression::Return(None))
    }
}

#[derive(Debug)]
pub struct ValueReturn;

impl Instruction for ValueReturn {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let return_value = ctx.operands.pop().context("no return value popped")?;

        Ok(Progression::Return(Some(return_value)))
    }
}

#[derive(Debug)]
pub struct Goto {
    pub(crate) jump_to: i16,
}

impl Instruction for Goto {
    fn handle(&self, _vm: &VM, _ctx: &mut Context) -> Result<Progression, Throwable> {
        Ok(Progression::JumpRel(self.jump_to as i32))
    }
}

#[derive(Debug)]
pub struct ArrayLength;

impl Instruction for ArrayLength {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        // The arrayref must be of type reference and must refer to an array. It is popped from the operand stack.
        let array = arg!(ctx, "array" => Array<()>);

        // The length of the array it references is determined.
        let len = array.unwrap_ref().len() as i32;

        // That length is pushed onto the operand stack as an int.
        ctx.operands.push(RuntimeValue::Integral(len.into()));
        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct ANewArray {
    pub(crate) type_index: u16,
}

impl Instruction for ANewArray {
    fn handle(&self, vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        // The count must be of type int. It is popped off the operand stack.
        let count = arg!(ctx, "count" => i32);

        // The run-time constant pool entry at the index must
        // be a symbolic reference to a class, array, or interface type. The
        // named class, array, or interface type is resolved (§5.4.3.1).
        let ty: ConstantEntry = ctx
            .class
            .unwrap_ref()
            .class_file()
            .constant_pool
            .address(self.type_index)
            .resolve();

        let array_ty = match ty {
            ConstantEntry::Class(data) => {
                let class_name = data.name.resolve().string();

                // Allow descriptors that already look like arrays to be parsed as normal
                let class_name = if class_name.starts_with('[') {
                    class_name
                } else {
                    // Otherwise, form them into array descriptors
                    format!("[L{};", class_name)
                };

                vm.class_loader().for_name(class_name.into())?
            }
            e => return Err(internal!("{:#?} cannot be used as an array type", e)),
        };

        // All components of the new array are initialized to null, the default value for reference types (§2.4).
        let mut values: Vec<RefTo<Object>> = Vec::with_capacity(count.value as usize);
        values.resize_with(count.value as usize, RefTo::null);

        // A new array with components of that type, of length count, is allocated
        // from the garbage-collected heap.
        let array = Array::<RefTo<Object>>::from_vec(array_ty, values);

        // and a ref to this new array object is pushed onto the operand stack.
        ctx.operands.push(RuntimeValue::Object(array.erase()));

        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct NewArray {
    pub(crate) type_tag: u8,
}

impl Instruction for NewArray {
    fn handle(&self, vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        // The count must be of type int. It is popped off the operand stack.
        let count = arg!(ctx, "count" => i32);

        // The atype is a code that indicates the type of array to create.
        let atype = ArrayPrimitive::from_tag(self.type_tag)?;

        // A new array whose components are of type atype and of length
        // count is allocated from the garbage-collected heap.
        let array = match &atype {
            ArrayPrimitive::Bool => {
                let values: Vec<Bool> = vec![0; count.value as usize];
                RuntimeValue::Object(
                    Array::<Bool>::from_vec(vm.class_loader().for_name("[Z".into())?, values)
                        .erase(),
                )
            }
            ArrayPrimitive::Char => {
                let values: Vec<Char> = vec![0; count.value as usize];
                RuntimeValue::Object(
                    Array::<Char>::from_vec(vm.class_loader().for_name("[C".into())?, values)
                        .erase(),
                )
            }
            ArrayPrimitive::Float => {
                let values: Vec<Float> = vec![0.0; count.value as usize];
                RuntimeValue::Object(
                    Array::<Float>::from_vec(vm.class_loader().for_name("[F".into())?, values)
                        .erase(),
                )
            }
            ArrayPrimitive::Double => {
                let values: Vec<Double> = vec![0.0; count.value as usize];
                RuntimeValue::Object(
                    Array::<Double>::from_vec(vm.class_loader().for_name("[D".into())?, values)
                        .erase(),
                )
            }
            ArrayPrimitive::Byte => {
                let values: Vec<Byte> = vec![0; count.value as usize];
                RuntimeValue::Object(
                    Array::<Byte>::from_vec(vm.class_loader().for_name("[B".into())?, values)
                        .erase(),
                )
            }
            ArrayPrimitive::Short => {
                let values: Vec<Short> = vec![0; count.value as usize];
                RuntimeValue::Object(
                    Array::<Short>::from_vec(vm.class_loader().for_name("[S".into())?, values)
                        .erase(),
                )
            }
            ArrayPrimitive::Int => {
                let values: Vec<Int> = vec![0; count.value as usize];
                RuntimeValue::Object(
                    Array::<Int>::from_vec(vm.class_loader().for_name("[I".into())?, values)
                        .erase(),
                )
            }
            ArrayPrimitive::Long => {
                let values: Vec<Long> = vec![0; count.value as usize];
                RuntimeValue::Object(
                    Array::<Long>::from_vec(vm.class_loader().for_name("[J".into())?, values)
                        .erase(),
                )
            }
        };

        // and an arrayref to this new array object is pushed onto the operand stack.
        ctx.operands.push(array);

        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct ArrayStore {
    pub(crate) ty: RefTo<Class>,
}

impl Instruction for ArrayStore {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let value = pop!(ctx);
        let index = arg!(ctx, "index" => i32);

        let ty = self.ty.unwrap_ref();
        if ty.is_primitive() {
            match ty.name() {
                n if { n == types::LONG.name } => {
                    let array = arg!(ctx, "array" => Array<Long>);
                    array.with_lock(|array| {
                        let array = array.slice_mut();
                        let value = value.as_integral().expect("array store exception").value;
                        array[index.value as usize] = value as Long;
                    });
                }
                n if { n == types::DOUBLE.name } => {
                    let array = arg!(ctx, "array" => Array<Double>);
                    array.with_lock(|array| {
                        let array = array.slice_mut();
                        let value = value.as_floating().expect("array store exception").value;
                        array[index.value as usize] = value as Double;
                    });
                }
                n if { n == types::FLOAT.name } => {
                    let array = arg!(ctx, "array" => Array<Float>);
                    array.with_lock(|array| {
                        let array = array.slice_mut();
                        let value = value.as_floating().expect("array store exception").value;
                        array[index.value as usize] = value as Float;
                    });
                }
                n if { n == types::BYTE.name } => {
                    let array = arg!(ctx, "array" => Array<Byte>);
                    array.with_lock(|array| {
                        let array = array.slice_mut();
                        let value = value.as_integral().expect("array store exception").value;
                        array[index.value as usize] = value as Byte;
                    });
                }
                n if { n == types::CHAR.name } => {
                    let array = arg!(ctx, "array" => Array<Char>);
                    array.with_lock(|array| {
                        let array = array.slice_mut();
                        let value = value.as_integral().expect("array store exception").value;
                        array[index.value as usize] = value as Char;
                    });
                }
                n if { n == types::INT.name } => {
                    let array = arg!(ctx, "array" => Array<Int>);
                    array.with_lock(|array| {
                        let array = array.slice_mut();
                        let value = value.as_integral().expect("array store exception").value;
                        array[index.value as usize] = value as Int;
                    });
                }
                ty => return Err(internal!("cannot encode {:#?}", ty)),
            }
        } else {
            let array = arg!(ctx, "array" => Array<RefTo<Object>>);
            array.with_lock(|array| {
                let array = array.slice_mut();
                let value = value.as_object().expect("array store exception").clone();
                array[index.value as usize] = value;
            });
        }

        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct ArrayLoad {
    pub(crate) ty: RefTo<Class>,
}

impl Instruction for ArrayLoad {
    fn handle(&self, vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let index = arg!(ctx, "index" => i32);
        let ty = self.ty.unwrap_ref();

        if index.value < 0 {
            return Ok(Progression::Throw(vm.try_make_error(
                VMError::ArrayIndexOutOfBounds { at: index.value },
            )?));
        }

        let value = if ty.is_primitive() {
            match ty.name() {
                n if { n == types::CHAR.name } => {
                    let array = arg!(ctx, "array" => Array<Char>);
                    let array = array.unwrap_ref().slice();

                    if index.value >= array.len() as i64 {
                        return Ok(Progression::Throw(vm.try_make_error(
                            VMError::ArrayIndexOutOfBounds { at: index.value },
                        )?));
                    }

                    let value = array[index.value as usize];

                    // TODO: Sign extension here?
                    RuntimeValue::Integral((value as i32).into())
                }
                n if { n == types::DOUBLE.name } => {
                    let array = arg!(ctx, "array" => Array<Double>);
                    let array = array.unwrap_ref().slice();
                    let value = array[index.value as usize];

                    RuntimeValue::Floating(value.into())
                }
                n if { n == types::FLOAT.name } => {
                    let array = arg!(ctx, "array" => Array<Float>);
                    let array = array.unwrap_ref().slice();
                    let value = array[index.value as usize];

                    RuntimeValue::Floating(value.into())
                }
                n if { n == types::BYTE.name } => {
                    let array = arg!(ctx, "array" => Array<Byte>);
                    let array = array.unwrap_ref().slice();
                    let value = array[index.value as usize];

                    // TODO: Sign extension here
                    RuntimeValue::Integral((value as i32).into())
                }
                n if { n == types::LONG.name } => {
                    let array = arg!(ctx, "array" => Array<Long>);
                    let array = array.unwrap_ref().slice();
                    let value = array[index.value as usize];

                    RuntimeValue::Integral(value.into())
                }
                n if { n == types::INT.name } => {
                    let array = arg!(ctx, "array" => Array<Int>);
                    let array = array.unwrap_ref().slice();

                    if index.value >= array.len() as i64 {
                        return Ok(Progression::Throw(vm.try_make_error(
                            VMError::ArrayIndexOutOfBounds { at: index.value },
                        )?));
                    }

                    let value = array[index.value as usize];

                    RuntimeValue::Integral(value.into())
                }
                ty => return Err(internal!("cannot encode {:#?}", ty)),
            }
        } else {
            let array = arg!(ctx, "array" => Array<RefTo<Object>>);
            let array = array.unwrap_ref().slice();
            let value = array[index.value as usize].clone();

            RuntimeValue::Object(value)
        };

        ctx.operands.push(value);

        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct MonitorEnter;
impl Instruction for MonitorEnter {
    // TODO: Support when we support MT
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        pop!(ctx);
        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct MonitorExit;
impl Instruction for MonitorExit {
    // TODO: Support when we support MT
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        pop!(ctx);
        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub enum WideFormat {
    // FIXME: Add back when we support this
    // Format1 { opcode: u8, index: u16 },
    Format2 { index: u16, const_val: i16 },
}

#[derive(Debug)]
pub struct Wide {
    pub(crate) format: WideFormat,
}

impl Instruction for Wide {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        match self.format {
            WideFormat::Format2 { index, const_val } => {
                let local = ctx
                    .locals
                    .get_mut(index as usize)
                    .context(format!("no local @ {}", index))?;

                let int = local.as_integral_mut().context("not an int")?;
                int.value += const_val as i64;
            }
        }

        Ok(Progression::Next)
    }
}
