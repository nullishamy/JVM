use super::{Instruction, Progression};
use crate::arg;
use crate::pop;
use crate::Context;

use anyhow::Context as AnyhowContext;
use parse::pool::ConstantClass;
use parse::{
    classfile::Resolvable,
    pool::{ConstantEntry, ConstantField},
};
use runtime::error::Throwable;
use runtime::internal;
use runtime::object::builtins::Array;

use runtime::object::builtins::Object;
use runtime::object::interner::intern_string;
use runtime::object::layout::types::Bool;
use runtime::object::layout::types::Byte;
use runtime::object::layout::types::Char;
use runtime::object::layout::types::Double;
use runtime::object::layout::types::Float;
use runtime::object::layout::types::Int;
use runtime::object::layout::types::Long;
use runtime::object::layout::types::Short;
use runtime::object::mem::FieldRef;

use runtime::object::mem::RefTo;

use runtime::object::numeric::IntegralType;
use runtime::object::value::ComputationalType;
use runtime::object::value::RuntimeValue;
use runtime::vm::VM;
use support::descriptor::ArrayType;
use support::descriptor::{BaseType, FieldType};
use support::types::FieldDescriptor;

#[derive(Debug)]
pub struct PushConst {
    pub(crate) value: RuntimeValue,
}

impl Instruction for PushConst {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        ctx.operands.push(self.value.clone());
        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct Ldc2W {
    pub(crate) index: u16,
}

impl Instruction for Ldc2W {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let value = ctx
            .class
            .unwrap_ref()
            .class_file()
            .constant_pool
            .address(self.index)
            .try_resolve()
            .context(format!("no value @ index {}", self.index))?;

        match value {
            ConstantEntry::Long(data) => {
                ctx.operands
                    .push(RuntimeValue::Integral((data.bytes as i64).into()));
            }
            ConstantEntry::Double(data) => {
                ctx.operands.push(RuntimeValue::Floating(data.bytes.into()));
            }
            v => return Err(internal!("cannot load {:#?} with ldc2w", v)),
        };

        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct Ldc {
    pub(crate) index: u16,
}

impl Instruction for Ldc {
    fn handle(&self, vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let value = ctx
            .class
            .unwrap_ref()
            .class_file()
            .constant_pool
            .address(self.index)
            .try_resolve()
            .context(format!("no value @ index {}", self.index))?;

        match value {
            ConstantEntry::Integer(data) => {
                ctx.operands
                    .push(RuntimeValue::Integral((data.bytes as i32).into()));
            }
            ConstantEntry::Float(data) => {
                ctx.operands.push(RuntimeValue::Floating(data.bytes.into()));
            }
            ConstantEntry::String(data) => {
                let str = data.string();
                let obj = intern_string(str)?;

                ctx.operands.push(RuntimeValue::Object(obj.erase()))
            }
            ConstantEntry::Class(data) => {
                let class_name = data.name.resolve().string();
                let class_name = FieldType::parse(class_name.clone())
                    .or_else(|_| FieldType::parse(format!("L{};", class_name)))?;

                let class = vm.class_loader().for_name(class_name)?;

                ctx.operands.push(RuntimeValue::Object(class.erase()));
            }
            v => return Err(internal!("cannot load {:#?} with ldc", v)),
        };

        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct LoadLocal {
    pub(crate) index: usize,
}

impl Instruction for LoadLocal {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let local = ctx
            .locals
            .get(self.index)
            .context(format!("no local @ {}", self.index))?;

        ctx.operands.push(local.clone());
        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct StoreLocal {
    pub(crate) index: usize,
    pub(crate) store_next: bool,
}

impl Instruction for StoreLocal {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let locals = &mut ctx.locals;
        let target_index = if self.store_next {
            self.index + 1
        } else {
            self.index
        };
        let value = ctx.operands.pop().context("no operand to pop")?.clone();

        // Fill enough slots to be able to store at an arbitrary index
        while locals.len() <= target_index {
            locals.push(RuntimeValue::null_ref());
        }

        locals[self.index] = value.clone();
        if self.store_next {
            locals[self.index + 1] = value;
        }
        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct GetField {
    pub(crate) index: u16,
}

impl Instruction for GetField {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        // The run-time constant pool entry at the index must be a symbolic
        // reference to a field (§5.1), which gives the name and descriptor of
        // the field as well as a symbolic reference to the class in which the
        // field is to be found.
        let field: ConstantField = ctx
            .class
            .unwrap_ref()
            .class_file()
            .constant_pool
            .address(self.index)
            .try_resolve()?;

        // The referenced field is resolved (§5.4.3.2).
        // TODO: Field resolution (through super classes etc)

        let name_and_type = field.name_and_type.resolve();
        let field_ty: FieldDescriptor = (
            name_and_type.name.resolve().string(),
            name_and_type.descriptor.resolve().string(),
        )
            .try_into()
            .unwrap();

        // The objectref, which must be of type reference but not an array
        // type, is popped from the operand stack.
        let objectref = arg!(ctx, "objectref" => Object);

        // The value of the reference field in objectref is fetched and pushed onto the operand stack.
        let value = match field_ty.descriptor() {
            FieldType::Base(ty) => match ty {
                BaseType::Boolean => {
                    let field: FieldRef<Bool> = objectref.unwrap_ref().field(&field_ty).unwrap();
                    RuntimeValue::Integral(field.copy_out().into())
                }
                BaseType::Char => {
                    let field: FieldRef<Char> = objectref.unwrap_ref().field(&field_ty).unwrap();

                    // TODO: Not entirely sure what the behaviour is supposed to be here wrt extension
                    let val = field.copy_out();
                    RuntimeValue::Integral((val as Int).into())
                }
                BaseType::Float => {
                    let field: FieldRef<Float> = objectref.unwrap_ref().field(&field_ty).unwrap();
                    RuntimeValue::Floating(field.copy_out().into())
                }
                BaseType::Double => {
                    let field: FieldRef<Double> = objectref.unwrap_ref().field(&field_ty).unwrap();
                    RuntimeValue::Floating(field.copy_out().into())
                }
                BaseType::Byte => {
                    let field: FieldRef<Byte> = objectref.unwrap_ref().field(&field_ty).unwrap();

                    // TODO: Not entirely sure what the behaviour is supposed to be here wrt extension
                    let val = field.copy_out();
                    RuntimeValue::Integral((val as Int).into())
                }
                BaseType::Short => {
                    let field: FieldRef<Short> = objectref.unwrap_ref().field(&field_ty).unwrap();

                    // TODO: Not entirely sure what the behaviour is supposed to be here wrt extension
                    let val = field.copy_out();
                    RuntimeValue::Integral((val as Int).into())
                }
                BaseType::Int => {
                    let field: FieldRef<Int> = objectref.unwrap_ref().field(&field_ty).unwrap();

                    RuntimeValue::Integral(field.copy_out().into())
                }
                BaseType::Long => {
                    let field: FieldRef<Long> = objectref.unwrap_ref().field(&field_ty).unwrap();
                    RuntimeValue::Integral(field.copy_out().into())
                }
                BaseType::Void => return Err(internal!("cannot read void field")),
            },
            FieldType::Object(_) => {
                let field: FieldRef<RefTo<Object>> =
                    objectref.unwrap_ref().field(&field_ty).unwrap();
                RuntimeValue::Object(field.unwrap_ref().clone())
            }
            FieldType::Array(_) => {
                let field: FieldRef<RefTo<Object>> =
                    objectref.unwrap_ref().field(&field_ty).unwrap();
                let value = field.unwrap_ref();
                RuntimeValue::Object(value.clone())
            }
        };

        ctx.operands.push(value);

        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct PutField {
    pub(crate) index: u16,
}

impl Instruction for PutField {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        // The run-time constant pool entry at the index must be a symbolic
        // reference to a field (§5.1), which gives the name and descriptor of
        // the field as well as a symbolic reference to the class in which the
        // field is to be found.
        let field: ConstantField = ctx
            .class
            .unwrap_ref()
            .class_file()
            .constant_pool
            .address(self.index)
            .try_resolve()?;

        // The referenced field is resolved (§5.4.3.2).
        // TODO: Field resolution (through super classes etc)

        let name_and_type = field.name_and_type.resolve();
        let field_ty: FieldDescriptor = (
            name_and_type.name.resolve().string(),
            name_and_type.descriptor.resolve().string(),
        )
            .try_into()
            .unwrap();

        // TODO: Type check & convert as needed

        //The value and objectref are popped from the operand stack.
        let value = pop!(ctx);
        let objectref = arg!(ctx, "objectref" => Object);

        let o = objectref.unwrap_ref();
        match field_ty.descriptor() {
            FieldType::Base(ref base) => match base {
                BaseType::Boolean => {
                    let field = o.field::<Bool>(&field_ty).unwrap();
                    field.write(value.as_integral().unwrap().value as Bool)
                }
                BaseType::Char => {
                    let field = o.field::<Char>(&field_ty).unwrap();
                    field.write(value.as_integral().unwrap().value as Char)
                }
                BaseType::Float => {
                    let field = o.field::<Float>(&field_ty).unwrap();
                    field.write(value.as_floating().unwrap().value as Float)
                }
                BaseType::Double => {
                    let field = o.field::<Double>(&field_ty).unwrap();
                    field.write(value.as_floating().unwrap().value as Double)
                }
                BaseType::Byte => {
                    let field = o.field::<Byte>(&field_ty).unwrap();
                    field.write(value.as_integral().unwrap().value as Byte)
                }
                BaseType::Short => {
                    let field = o.field::<Short>(&field_ty).unwrap();
                    field.write(value.as_integral().unwrap().value as Short)
                }
                BaseType::Int => {
                    let field = o.field::<Int>(&field_ty).unwrap();
                    field.write(value.as_integral().unwrap().value as Int)
                }
                BaseType::Long => {
                    let field = o.field::<Long>(&field_ty).unwrap();
                    field.write(value.as_integral().unwrap().value as Long)
                }
                BaseType::Void => todo!(),
            },
            FieldType::Object(_) => {
                let field = o.field::<RefTo<Object>>(&field_ty).unwrap();
                field.write(value.as_object().unwrap().clone());
            }
            FieldType::Array(_) => {
                let field = o.field::<RefTo<Object>>(&field_ty).unwrap();
                field.write(value.as_object().unwrap().clone());
            }
        };

        // Otherwise, the referenced field in objectref is set to value
        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct GetStatic {
    pub(crate) index: u16,
}

impl Instruction for GetStatic {
    fn handle(&self, vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        // The run-time constant pool entry at the index must be a symbolic
        // reference to a field (§5.1), which gives the name and descriptor of
        // the field as well as a symbolic reference to the class in which the
        // field is to be found.
        let field: ConstantField = ctx
            .class
            .unwrap_ref()
            .class_file()
            .constant_pool
            .address(self.index)
            .try_resolve()?;

        // The referenced field is resolved (§5.4.3.2).
        // TODO: Field resolution (through super classes etc)

        // On successful resolution of the field, the class or interface that
        // declared the resolved field is initialized if that class or interface
        // has not already been initialized (§5.5).
        let class_name = field.class.resolve().name.resolve().string();
        let class = vm
            .class_loader()
            .for_name(format!("L{};", class_name).into())?;

        vm.initialise_class(class.clone())?;

        let name_and_type = field.name_and_type.resolve();
        let name = name_and_type.name.resolve().string();

        let _class_name = class.unwrap_ref().name();

        // The value of the class or interface field is fetched and pushed onto the operand stack.

        // HACK: Hacking in inherited statics
        // This should be handled by the field resolution algorithm, as suggested above.
        // This algorithm is similar to the method resolution algorithms already implemented for the invoke* instructions
        let mut cls = class.clone();
        loop {
            let statics = cls.unwrap_ref().statics();
            let statics = statics.read();
            let field = statics.get(&name);

            if let Some(f) = field {
                let value = f.value.clone().unwrap();
                ctx.operands.push(value);
                break;
            } else {
                let sup = cls.unwrap_ref().super_class();
                // We searched every class and could not find the static, this is a fault with the classfile
                // or our parser / layout mechanisms
                if sup.is_null() {
                    return Err(internal!(
                        "could not locate static field in class or super class(es)"
                    ));
                }

                vm.initialise_class(sup.clone())?;

                drop(statics);

                cls = sup;
            }
        }

        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct PutStatic {
    pub(crate) index: u16,
}

impl Instruction for PutStatic {
    fn handle(&self, vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        // The run-time constant pool entry at the index must be a symbolic
        // reference to a field (§5.1), which gives the name and descriptor of
        // the field as well as a symbolic reference to the class in which the
        // field is to be found.
        let field: ConstantField = ctx
            .class
            .unwrap_ref()
            .class_file()
            .constant_pool
            .address(self.index)
            .try_resolve()?;

        // The referenced field is resolved (§5.4.3.2).
        // TODO: Field resolution (through super classes etc)

        // On successful resolution of the field, the class or interface that
        // declared the resolved field is initialized if that class or interface
        // has not already been initialized (§5.5).
        let class_name = field.class.resolve().name.resolve().string();
        let class = vm
            .class_loader()
            .for_name(format!("L{};", class_name).into())?;
        vm.initialise_class(class.clone())?;

        let name_and_type = field.name_and_type.resolve();
        let name = name_and_type.name.resolve().string();

        // TODO: Type check & convert as needed
        let value = pop!(ctx);
        let statics = class.unwrap_ref().statics();
        let mut statics = statics.write();

        statics.get_mut(&name).unwrap().value = Some(value);

        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct Dup;

impl Instruction for Dup {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let value = pop!(ctx);
        ctx.operands.push(value.clone());
        ctx.operands.push(value);
        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct DupX1;

impl Instruction for DupX1 {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let value1 = pop!(ctx);
        let value2 = pop!(ctx);

        ctx.operands.push(value1.clone());
        ctx.operands.push(value2);
        ctx.operands.push(value1.clone());
        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct DupX2;

impl Instruction for DupX2 {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let value = pop!(ctx);

        match value.computational_type() {
            ComputationalType::Category1 => {
                // Form 1:
                // ..., value3, value2, value1 →
                // ..., value1, value3, value2, value1
                // where value1, value2, and value3 are all values of a category 1
                // computational type (§2.11.1).
                let value1 = value;
                let value2 = pop!(ctx);
                let value3 = pop!(ctx);

                ctx.operands.push(value1.clone());
                ctx.operands.push(value3.clone());
                ctx.operands.push(value2.clone());
                ctx.operands.push(value1);
            }
            ComputationalType::Category2 => {
                // Form 2:
                // ..., value2, value1 →
                // ..., value1, value2, value1
                // where value1 is a value of a category 1 computational type and
                // value2 is a value of a category 2 computational type (§2.11.1).
                let value1 = value;
                let value2 = pop!(ctx);

                ctx.operands.push(value1.clone());
                ctx.operands.push(value2);
                ctx.operands.push(value1);
            }
        }

        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct Dup2;

impl Instruction for Dup2 {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let value = pop!(ctx);

        match value.computational_type() {
            ComputationalType::Category1 => {
                // Form 1:
                // ..., value2, value1 →
                // ..., value2, value1, value2, value1
                // where both value1 and value2 are values of a category 1
                // computational type (§2.11.1).
                let value1 = value;
                let value2 = pop!(ctx);

                ctx.operands.push(value2.clone());
                ctx.operands.push(value1.clone());
                ctx.operands.push(value2);
                ctx.operands.push(value1);
            }
            ComputationalType::Category2 => {
                // Form 2:
                // ..., value →
                // ..., value, value
                // where value is a value of a category 2 computational type
                // (§2.11.1).
                ctx.operands.push(value.clone());
                ctx.operands.push(value);
            }
        }

        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct MultiANewArray {
    pub(crate) type_index: u16,
    pub(crate) dimensions: u8,
}

impl Instruction for MultiANewArray {
    fn handle(&self, vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let mut counts = vec![];

        for _ in 0..self.dimensions {
            counts.push(arg!(ctx, "" => i32).value)
        }

        let class: ConstantClass = ctx
            .class
            .unwrap_ref()
            .class_file()
            .constant_pool
            .address(self.type_index)
            .try_resolve()?;

        let class_name = class.name.resolve().string();
        let array_type = FieldType::parse(class_name.clone())
            .or_else(|_| FieldType::parse(format!("L{};", class_name)))?;

        let array_type = ArrayType {
            field_type: Box::new(array_type),
            dimensions: self.dimensions as usize,
        };

        fn make_layers(
            ty: ArrayType,
            vm: &VM,
            mut counts: &mut [i64],
        ) -> RefTo<Object> {
            let len = counts.len();
            let count = counts[len-1];
            counts = &mut counts[..len-1];
            
            let array = {
                let array_class = vm.class_loader().for_name(*ty.field_type.clone()).unwrap();

                // Need to null-init first. RefTo is really just a ptr, and a null ptr is 000000000, which is also the bit pattern
                // for the numeric types. Thus, we can treat it as such(?)
                let mut values: Vec<RefTo<Object>> = Vec::with_capacity(count as usize);
                values.resize_with(count as usize, RefTo::null);

                // Now we construct the array
                Array::from_vec(array_class, values)
            };

            // If we can construct further layers, do so.
            if !counts.is_empty() {
                let next_layer_type = {
                    // Go one level deeper
                    let mut ty = ty.clone();
                    ty.dimensions -= 1;
                    ty
                };

                // Make all the subarrays
                for i in 0..count {
                    let next_layer = make_layers(next_layer_type.clone(), vm, counts);
                    let slice = array.unwrap_mut().slice_mut();
                    slice[i as usize] = next_layer;
                }
            }

            // Otherwise, we are done. Return what will the first layer once the stack unwinds.
            array.erase()
        }

        let array = make_layers(array_type, vm, &mut counts);
        ctx.operands.push(RuntimeValue::Object(array));

        Ok(Progression::Next)
    }
}
