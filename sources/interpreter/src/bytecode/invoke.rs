use std::sync::atomic::AtomicU64;

use super::{Instruction, Progression};
use crate::arg;
use crate::pop;
use crate::Context;

use anyhow::Context as AnyhowContext;
use runtime::error::Frame;
use runtime::error::RuntimeException;
use runtime::error::Throwable;
use runtime::error::VMError;
use runtime::internal;
use runtime::native::NativeFunction;
use runtime::object::builtins::Array;
use runtime::object::builtins::BuiltinString;
use runtime::object::builtins::Class;
use runtime::object::builtins::Object;
use runtime::object::layout::types::Bool;
use runtime::object::layout::types::Byte;
use runtime::object::layout::types::Char;
use runtime::object::layout::types::Double;
use runtime::object::layout::types::Float;
use runtime::object::layout::types::Int;
use runtime::object::layout::types::Long;
use runtime::object::layout::types::Short;
use runtime::object::mem::FieldRef;
use runtime::object::mem::JavaObject;
use runtime::object::mem::RefTo;
use runtime::object::numeric::FloatingType;
use runtime::object::numeric::IntegralType;
use runtime::object::value::RuntimeValue;

use parking_lot::RwLock;
use parse::{
    attributes::CodeAttribute,
    classfile::{Method, Resolvable},
    flags::{FieldAccessFlag, MethodAccessFlag},
    pool::ConstantEntry,
};
use runtime::vm::VM;
use support::types::MethodDescriptor;
use support::{
    descriptor::{BaseType, FieldType, MethodType},
    encoding::{decode_string, CompactEncoding},
};
use tracing::{debug, info};

#[derive(Debug)]
pub struct InvokeDynamic {
    pub(crate) _index: u16,
    pub(crate) _zeroes: u16,
}

impl Instruction for InvokeDynamic {
    fn handle(&self, vm: &VM, _ctx: &mut Context) -> Result<Progression, Throwable> {
        Err(internal!("cannot use invokedynamic!: {:#?}", vm.frames()))
    }
}

#[derive(Debug)]
pub struct InvokeVirtual {
    pub(crate) index: u16,
}

impl Instruction for InvokeVirtual {
    fn handle(&self, vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let cls = ctx.class.clone();
        let cls = cls
            .to_ref()
            .ok_or(vm.try_make_error(VMError::NullPointerException {
                ctx: "invokevirtual".to_string(),
            })?)?;

        let pool = &cls.class_file().constant_pool;

        // The unsigned indExbyte1 and indexbyte2 are used to construct an
        // index into the run-time constant pool of the current class (§2.6),
        let pool_entry = pool
            .address::<ConstantEntry>(self.index)
            .try_resolve()
            .context(format!("no method at index {}", self.index))?;

        // The run-time constant pool entry at the index must be a symbolic
        // reference to a method or an interface method (§5.1), which gives
        // the name and descriptor (§4.3.3) of the method or interface method
        // as well as a symbolic reference to the class or interface in which
        // the method or interface method is to be found.
        let (method_ty, class_name, _) = to_method_info(pool_entry)?;

        // The named method is resolved (§5.4.3.3, §5.4.3.4).
        let class_name_desc = if !class_name.starts_with('[') {
            FieldType::parse(format!("L{};", class_name))?
        } else {
            FieldType::parse(class_name.clone())?
        };

        let loaded_class = vm.class_loader().for_name(class_name_desc)?;

        let loaded_method = resolve_class_method(vm, loaded_class.clone(), &method_ty)?;

        // If the resolved method is not signature polymorphic (§2.9.3), then
        // the invokevirtual instruction proceeds as follows.
        // TODO: Support signature polymorphic methods

        vm.push_frame(Frame {
            method_name: method_ty.name().clone(),
            class_name,
        });

        debug!("Invoking: {:#?}", vm.frames().last());

        // NOTE: We must get the args before relative resolution.
        // This is because the `objectref` lives at the "bottom" of the stack,
        // below all of the args.
        let mut args_for_call = clone_args_from_operands(method_ty.descriptor(), ctx)?;
        let objectref = arg!(ctx, "objectref" => Object);

        // Let C be the class of objectref. A method is selected with respect
        // to C and the resolved method (§5.4.6). This is the method to be invoked.
        let objectclass = objectref
            .to_ref()
            .ok_or(vm.try_make_error(VMError::NullPointerException {
                ctx: format!("cannot invoke method '{}' on null", &method_ty.name()),
            })?)?
            .header()
            .class
            .clone();

        let (selected_class, selected_method) =
            select_method(vm, objectclass, loaded_class, loaded_method)?
                .ok_or(internal!("could not resolve method {}", method_ty))?;

        args_for_call.push(RuntimeValue::Object(objectref));
        args_for_call.reverse();

        let exec_result = do_call(vm, selected_method, selected_class, args_for_call)?;

        debug!("Returned from: {:#?}", vm.frames().last());
        vm.pop_frame();

        // Callee gave us a value, push it to our stack (Xreturn does this)
        if let Some(return_value) = exec_result {
            ctx.operands.push(return_value);
        }

        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct InvokeSpecial {
    pub(crate) index: u16,
}

impl Instruction for InvokeSpecial {
    fn handle(&self, vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let cls =
            ctx.class
                .to_ref()
                .ok_or(vm.try_make_error(VMError::NullPointerException {
                    ctx: "invokespecial".to_string(),
                })?)?;

        let pool = &cls.class_file().constant_pool;

        // The unsigned indexbyte1 and indexbyte2 are used to construct an
        // index into the run-time constant pool of the current class (§2.6),
        let pool_entry = pool
            .address::<ConstantEntry>(self.index)
            .try_resolve()
            .context(format!("no method at index {}", self.index))?;

        // The run-time constant pool entry at the index must be a symbolic
        // reference to a method or an interface method (§5.1), which gives
        // the name and descriptor (§4.3.3) of the method or interface method
        // as well as a symbolic reference to the class or interface in which
        // the method or interface method is to be found.
        let (method_ty, class_name, _) = to_method_info(pool_entry)?;

        // The named method is resolved (§5.4.3.3, §5.4.3.4).
        let loaded_class = vm
            .class_loader()
            .for_name(format!("L{};", class_name).into())?;

        let loaded_method = resolve_class_method(vm, loaded_class.clone(), &method_ty)?;

        vm.push_frame(Frame {
            method_name: method_ty.name().clone(),
            class_name,
        });

        debug!("Invoking: {:#?}", vm.frames().last());

        // NOTE: We must get the args before resolution.
        // This is because the `objectref` lives at the "bottom" of the stack,
        // below all of the args.
        let mut args_for_call = clone_args_from_operands(method_ty.descriptor(), ctx)?;

        let objectref = arg!(ctx, "objectref" => Object);

        let (selected_class, selected_method) =
            select_special_method(vm, loaded_class.clone(), loaded_class, loaded_method)?
                .ok_or(internal!("could not resolve special method {}", method_ty))?;

        args_for_call.push(RuntimeValue::Object(objectref));
        args_for_call.reverse();

        let exec_result = do_call(vm, selected_method, selected_class, args_for_call)?;

        debug!("Returned from: {:#?}", vm.frames().last());
        vm.pop_frame();

        // Callee gave us a value, push it to our stack (Xreturn does this)
        if let Some(return_value) = exec_result {
            ctx.operands.push(return_value);
        }

        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct InvokeStatic {
    pub(crate) index: u16,
}

impl Instruction for InvokeStatic {
    fn handle(&self, vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let cls =
            ctx.class
                .to_ref()
                .ok_or(vm.try_make_error(VMError::NullPointerException {
                    ctx: "invokestatic".to_string(),
                })?)?;
        let pool = &cls.class_file().constant_pool;

        // The unsigned indexbyte1 and indexbyte2 are used to construct an
        // index into the run-time constant pool of the current class (§2.6),
        let pool_entry = pool
            .address::<ConstantEntry>(self.index)
            .try_resolve()
            .context(format!("no method at index {}", self.index))?;

        // The run-time constant pool entry at the index must be a symbolic
        // reference to a method or an interface method (§5.1), which gives
        // the name and descriptor (§4.3.3) of the method or interface method
        // as well as a symbolic reference to the class or interface in which
        // the method or interface method is to be found.
        let (method_ty, class_name, location) = to_method_info(pool_entry)?;

        // The named method is resolved (§5.4.3.3, §5.4.3.4).
        let loaded_class = vm
            .class_loader()
            .for_name(format!("L{};", class_name).into())?;

        let loaded_method = match location {
            MethodLocation::Interface => {
                resolve_interface_method(vm, loaded_class.clone(), &method_ty)
            }
            MethodLocation::Class => resolve_class_method(vm, loaded_class.clone(), &method_ty),
        }?;

        // The resolved method must not be an instance initialization method,
        // or the class or interface initialization method (§2.9.1, §2.9.2).
        if method_ty.name() == "<clinit>" || method_ty.name() == "<init>" {
            return Err(internal!(
                "cannot call static method {}, it is a class initialisation method",
                method_ty
            ));
        }

        // On successful resolution of the method, the class or interface that
        // declared the resolved method is initialized if that class or interface
        // has not already been initialized (§5.5).
        vm.initialise_class(loaded_class.clone())?;

        // If the method is not native, the nargs argument values are popped
        // from the operand stack. A new frame is created on the Java Virtual
        // Machine stack for the method being invoked. The nargs argument
        // values are consecutively made the values of local variables of the
        // new frame, with arg1 in local variable 0 (or, if arg1 is of type
        // long or double, in local variables 0 and 1) and so on.

        vm.push_frame(Frame {
            method_name: method_ty.name().clone(),
            class_name,
        });

        debug!("Invoking: {:#?}", vm.frames().last());

        let mut args_for_call = clone_args_from_operands(method_ty.descriptor(), ctx)?;
        args_for_call.reverse();

        let exec_result = do_call(vm, loaded_method, loaded_class, args_for_call)?;

        debug!("Returned from: {:#?}", vm.frames().last());
        vm.pop_frame();

        // Callee gave us a value, push it to our stack (Xreturn does this)
        if let Some(return_value) = exec_result {
            ctx.operands.push(return_value);
        }

        Ok(Progression::Next)
    }
}

#[derive(Debug)]
pub struct InvokeInterface {
    pub(crate) index: u16,
    pub(crate) count: u8,
    pub(crate) zero: u8,
}

impl Instruction for InvokeInterface {
    fn handle(&self, vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        // The value of the fourth operand byte must always be zero.
        if self.zero != 0 {
            return Err(internal!("zero was not zero"));
        }

        // The count operand is an unsigned byte that must not be zero
        if self.count == 0 {
            return Err(internal!("count was 0"));
        }

        let cls = ctx.class.unwrap_ref();
        let pool = &cls.class_file().constant_pool;

        // The unsigned indexbyte1 and indexbyte2 are used to construct an
        // index into the run-time constant pool of the current class (§2.6),
        let pool_entry = pool
            .address::<ConstantEntry>(self.index)
            .try_resolve()
            .context(format!("no method at index {}", self.index))?;

        // The run-time constant pool entry at the index must be a symbolic
        // reference to a method or an interface method (§5.1), which gives
        // the name and descriptor (§4.3.3) of the method or interface method
        // as well as a symbolic reference to the class or interface in which
        // the method or interface method is to be found.
        let (method_ty, class_name, _) = to_method_info(pool_entry)?;

        // The named method is resolved (§5.4.3.3, §5.4.3.4).
        let loaded_class = vm
            .class_loader()
            .for_name(format!("L{};", class_name).into())?;
        let loaded_method = resolve_interface_method(vm, loaded_class.clone(), &method_ty)?;

        // The resolved method must not be an instance initialization method,
        // or the class or interface initialization method (§2.9.1, §2.9.2).
        if method_ty.name() == "<clinit>" || method_ty.name() == "<init>" {
            return Err(internal!(
                "cannot call interface method {}, it is a class initialisation method",
                method_ty
            ));
        }

        // NOTE: We must get the args before relative resolution.
        // This is because the `objectref` lives at the "bottom" of the stack,
        // below all of the args.
        let mut args_for_call = clone_args_from_operands(method_ty.descriptor(), ctx)?;
        let objectref = arg!(ctx, "objectref" => Object);

        // Let C be the class of objectref. A method is selected with respect
        // to C and the resolved method (§5.4.6). This is the method to be invoked.
        let objectclass = objectref.unwrap_ref().header().class.clone();

        let (selected_class, selected_method) =
            select_method(vm, objectclass, loaded_class, loaded_method)?
                .ok_or(internal!("could not resolve method {}", method_ty))?;

        vm.push_frame(Frame {
            method_name: method_ty.name().clone(),
            class_name,
        });

        debug!("Invoking: {:#?}", vm.frames().last());
        args_for_call.push(RuntimeValue::Object(objectref));
        args_for_call.reverse();

        let exec_result = do_call(vm, selected_method, selected_class, args_for_call)?;

        debug!("Returned from: {:#?}", vm.frames().last());
        vm.pop_frame();

        // Callee gave us a value, push it to our stack (Xreturn does this)
        if let Some(return_value) = exec_result {
            ctx.operands.push(return_value);
        }

        Ok(Progression::Next)
    }
}

fn resolve_class_method(
    vm: &VM,
    class: RefTo<Class>,
    method_ty: &MethodDescriptor,
) -> Result<Method, Throwable> {
    // To resolve an unresolved symbolic reference from D to a method in a class C, the
    // symbolic reference to C given by the method reference is first resolved (§5.4.3.1).

    // When resolving a method reference:
    // 1. If C is an interface, method resolution throws an IncompatibleClassChangeError.
    if class.unwrap_ref().is_interface() {
        return Err(internal!("cannot resolve method on interface"));
    }

    // 2. Otherwise, method resolution attempts to locate the referenced method in C and its superclasses:
    // • If C declares exactly one method with the name specified by the method
    // reference, and the declaration is a signature polymorphic method (§2.9.3),
    // then method lookup succeeds. All the class names mentioned in the
    // descriptor are resolved (§5.4.3.1).
    // The resolved method is the signature polymorphic method declaration. It is
    // not necessary for C to declare a method with the descriptor specified by the
    // method reference.
    // • Otherwise, if C declares a method with the name and descriptor specified by
    // the method reference, method lookup succeeds.

    let class_method = class
        .unwrap_ref()
        .class_file()
        .methods
        .locate(method_ty)
        .cloned();

    if let Some(class_method) = class_method {
        return Ok(class_method.clone());
    }

    // • Otherwise, if C has a superclass, step 2 of method resolution is recursively
    // invoked on the direct superclass of C.
    if let Some(super_class) = class.unwrap_ref().super_class().into_option() {
        let class_name = super_class.name();
        let super_class = vm
            .class_loader()
            .for_name(format!("L{};", class_name).into())?;

        return resolve_class_method(vm, super_class, method_ty);
    }

    // Otherwise, method resolution attempts to locate the referenced method in the
    // superinterfaces of the specified class C:
    // • If the maximally-specific superinterface methods of C for the name and
    // descriptor specified by the method reference include exactly one method that
    // does not have its ACC_ABSTRACT flag set, then this method is chosen and
    // method lookup succeeds.
    // • Otherwise, if any superinterface of C declares a method with the name and
    // descriptor specified by the method reference that has neither its ACC_PRIVATE
    // flag nor its ACC_STATIC flag set, one of these is arbitrarily chosen and method
    // lookup succeeds.

    // TODO: Interface resolution

    // • Otherwise, method lookup fails.

    Err(internal!(
        "could not resolve method {} in {}",
        method_ty,
        class.unwrap_ref().name()
    ))
}

fn resolve_interface_method(
    vm: &VM,
    class: RefTo<Class>,
    method_ty: &MethodDescriptor,
) -> Result<Method, Throwable> {
    // To resolve an unresolved symbolic reference from D to an interface method in an
    // interface C, the symbolic reference to C given by the interface method reference is first resolved (§5.4.3.1)

    // When resolving an interface method reference:
    // 1. If C is not an interface, interface method resolution throws an IncompatibleClassChangeError
    if !class.unwrap_ref().is_interface() {
        return Err(internal!("cannot resolve interface method on class"));
    }

    // 2. Otherwise, if C declares a method with the name and descriptor specified by
    // the interface method reference, method lookup succeeds.
    let own_method = class
        .unwrap_ref()
        .class_file()
        .methods
        .locate(method_ty)
        .cloned();

    if let Some(own_method) = own_method {
        return Ok(own_method.clone());
    }

    // 3. Otherwise, if the class Object declares a method with the name and descriptor
    // specified by the interface method reference, which has its ACC_PUBLIC flag set
    // and does not have its ACC_STATIC flag set, method lookup succeeds.
    // TODO: Respect the flags
    if let Some(super_class) = class.unwrap_ref().super_class().into_option() {
        let class_name = super_class.name();
        let super_class = vm
            .class_loader()
            .for_name(format!("L{};", class_name).into())?;

        return resolve_interface_method(vm, super_class, method_ty);
    }

    // 4. Otherwise, if the maximally-specific superinterface methods (§5.4.3.3) of C
    // for the name and descriptor specified by the method reference include exactly
    // one method that does not have its ACC_ABSTRACT flag set, then this method is
    // chosen and method lookup succeeds

    // TODO: Interface resolution

    // 5. Otherwise, if any superinterface of C declares a method with the name and
    // descriptor specified by the method reference that has neither its ACC_PRIVATE
    // flag nor its ACC_STATIC flag set, one of these is arbitrarily chosen and method
    // lookup succeeds

    // • Otherwise, method lookup fails.

    Err(internal!(
        "could not resolve method {} in {}",
        method_ty,
        class.unwrap_ref().name()
    ))
}

fn select_special_method(
    vm: &VM,
    class: RefTo<Class>,
    declared_class: RefTo<Class>,
    method: Method,
) -> Result<Option<(RefTo<Class>, Method)>, Throwable> {
    let method_ty: MethodDescriptor = (
        method.name.resolve().try_string()?,
        method.descriptor.resolve().try_string()?,
    )
        .try_into()
        .unwrap();

    // Let C be the class or interface named by the symbolic reference.

    // 1. If C contains a declaration for an instance method with the
    // same name and descriptor as the resolved method, then it is
    // the method to be invoked.
    let instance_method = class
        .unwrap_ref()
        .class_file()
        .methods
        .locate(&method_ty)
        .cloned();

    if let Some(instance_method) = instance_method {
        return Ok(Some((class, instance_method.clone())));
    }

    // 2. Otherwise, if C is a class and has a superclass, a search for
    // a declaration of an instance method with the same name
    // and descriptor as the resolved method is performed, starting
    // with the direct superclass of C and continuing with the direct
    // superclass of that class, and so forth, until a match is found or
    // no further superclasses exist. If a match is found, then it is the
    // method to be invoked.
    if let Some(super_class) = class.unwrap_ref().super_class().into_option() {
        let class_name = super_class.name();
        let super_class = vm
            .class_loader()
            .for_name(format!("L{};", class_name).into())?;

        return select_method(vm, super_class, declared_class, method);
    }

    // 3. Otherwise, if C is an interface and the class Object contains a
    // declaration of a public instance method with the same name
    // and descriptor as the resolved method, then it is the method
    // to be invoked.
    // TODO: Implement

    // 4. Otherwise, if there is exactly one maximally-specific method
    // (§5.4.3.3) in the superinterfaces of C that matches the resolved
    // method's name and descriptor and is not abstract, then it is
    // the method to be invoked.
    // TODO: Implement

    Ok(None)
}

fn select_method(
    vm: &VM,
    class: RefTo<Class>,
    declared_class: RefTo<Class>,
    method: Method,
) -> Result<Option<(RefTo<Class>, Method)>, Throwable> {
    // During execution of an invokeinterface or invokevirtual instruction, a method is
    // selected with respect to (i) the run-time type of the object on the stack, and (ii) a
    // method that was previously resolved by the instruction. The rules to select a method
    // with respect to a class or interface C and a method mR are as follows:

    let method_ty: MethodDescriptor = (
        method.name.resolve().try_string()?,
        method.descriptor.resolve().try_string()?,
    )
        .try_into()
        .unwrap();

    // 1. If mR is marked ACC_PRIVATE, then it is the selected method.
    if method.flags.has(MethodAccessFlag::PRIVATE) {
        let method = declared_class
            .unwrap_ref()
            .class_file()
            .methods
            .locate(&method_ty)
            .cloned()
            .ok_or(internal!("could not resolve method"))?;

        return Ok(Some((declared_class, method.clone())));
    }

    // 2. Otherwise, the selected method is determined by the following lookup procedure:
    // If C contains a declaration of an instance method m that can override mR (§5.4.5), then m is the selected method.
    let instance_method = class
        .unwrap_ref()
        .class_file()
        .methods
        .locate(&method_ty)
        .cloned();

    if let Some(instance_method) = instance_method {
        if method_can_override(&method, &instance_method) {
            return Ok(Some((class, instance_method.clone())));
        }
    }

    // Otherwise, if C has a superclass, a search for a declaration of an instance
    // method that can override mR is performed, starting with the direct superclass
    // of C and continuing with the direct superclass of that class, and so forth, until
    // a method is found or no further superclasses exist. If a method is found, it
    // is the selected method.
    if let Some(super_class) = class.unwrap_ref().super_class().into_option() {
        let class_name = super_class.name();
        let super_class = vm
            .class_loader()
            .for_name(format!("L{};", class_name).into())?;

        return select_method(vm, super_class, declared_class, method);
    }

    // Otherwise, the maximally-specific superinterface methods of C are
    // determined (§5.4.3.3). If exactly one matches mR's name and descriptor and
    // is not abstract, then it is the selected method.
    // TODO: This, once we figure out a better way to handle super classes

    Ok(None)
}

fn method_can_override(base: &Method, derived: &Method) -> bool {
    // An instance method mC can override another instance method mA iff all of the following are true:
    let (base_name, base_descriptor) = (
        base.name.resolve().string(),
        base.descriptor.resolve().string(),
    );

    let (derived_name, derived_descriptor) = (
        derived.name.resolve().string(),
        derived.descriptor.resolve().string(),
    );

    // mC has the same name and descriptor as mA.
    if base_name != derived_name && base_descriptor != derived_descriptor {
        return false;
    }

    // mC is not marked ACC_PRIVATE.
    let flags = &base.flags;
    if flags.has(MethodAccessFlag::PRIVATE) {
        return false;
    }

    // One of the following is true:
    // – mA is marked ACC_PUBLIC.
    // – mA is marked ACC_PROTECTED.
    // – mA is marked neither ACC_PUBLIC nor ACC_PROTECTED nor ACC_PRIVATE, and
    // either (a) the declaration of mA appears in the same run-time package as the
    // declaration of mC, or (b) if mA is declared in a class A and mC is declared in a class
    // C, then there exists a method mB declared in a class B such that C is a subclass
    // of B and B is a subclass of A and mC can override mB and mB can override mA.

    if flags.has(MethodAccessFlag::PUBLIC) || flags.has(MethodAccessFlag::PROTECTED) {
        return true;
    }

    // TODO: Try resolving package information before returning true
    true
}

#[derive(Debug)]
pub struct New {
    pub(crate) index: u16,
}

impl Instruction for New {
    fn handle(&self, vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        // The run-time constant pool entry at the index must be a symbolic
        // reference to a class or interface type. The named class or interface
        // type is resolved (§5.4.3.1) and should result in a class type.
        let entry: ConstantEntry = ctx
            .class
            .unwrap_ref()
            .class_file()
            .constant_pool
            .address(self.index)
            .resolve();

        let object_ty = match entry {
            ConstantEntry::Class(data) => {
                let class_name = data.name.resolve().string();
                vm.class_loader()
                    .for_name(format!("L{};", class_name).into())?
            }
            e => return Err(internal!("{:#?} cannot be used to create a new object", e)),
        };

        // Memory for a new instance of that class is allocated from the
        // garbage-collected heap, and the instance variables of the new
        // object are initialized to their default initial values (§2.3, §2.4).

        let layout = object_ty.unwrap_ref().instance_layout();
        let ptr = layout.alloc();

        // NOTE: layout.fields() contains the all inherited fields too.
        // FIXME: Do we need to do this if the memory is zero from alloc time?
        for field in layout.fields().values() {
            if field.data.flags.has(FieldAccessFlag::STATIC) {
                continue;
            }

            let field_start = unsafe { ptr.byte_add(field.location.offset) };
            let descriptor = field.data.descriptor.resolve().string();
            let descriptor = FieldType::parse(descriptor)?;
            match descriptor {
                FieldType::Base(ty) => match ty {
                    BaseType::Boolean => unsafe {
                        field_start.cast::<Bool>().write(0);
                    },
                    BaseType::Char => unsafe {
                        field_start.cast::<Char>().write(0);
                    },
                    BaseType::Float => unsafe {
                        field_start.cast::<Float>().write(0.0);
                    },
                    BaseType::Double => unsafe {
                        field_start.cast::<Double>().write(0.0);
                    },
                    BaseType::Byte => unsafe {
                        field_start.cast::<Byte>().write(0);
                    },
                    BaseType::Short => unsafe {
                        field_start.cast::<Short>().write(0);
                    },
                    BaseType::Int => unsafe {
                        field_start.cast::<Int>().write(0);
                    },
                    BaseType::Long => unsafe {
                        field_start.cast::<Long>().write(0);
                    },
                    BaseType::Void => return Err(internal!("cannot read void field")),
                },
                FieldType::Object(_) => {
                    unsafe { field_start.cast::<RefTo<Object>>().write(RefTo::null()) };
                }
                FieldType::Array(_) => {
                    unsafe { field_start.cast::<RefTo<Array<()>>>().write(RefTo::null()) };
                }
            }
        }

        let super_class = object_ty.unwrap_ref().super_class();

        unsafe {
            (*ptr).class = object_ty.clone();
            (*ptr).ref_count = AtomicU64::new(0);
            (*ptr).lock = RwLock::new(());
            (*ptr).super_class = super_class;
        }

        // The objectref, a reference to the instance, is pushed onto the operand stack.

        // Safety: We know ptr is not null here
        let objectref = unsafe { RefTo::from_ptr(ptr) };
        ctx.operands.push(RuntimeValue::Object(objectref));

        Ok(Progression::Next)
    }
}

#[derive(Debug)]
enum MethodLocation {
    Interface,
    Class,
}

fn to_method_info(
    pool_entry: ConstantEntry,
) -> Result<(MethodDescriptor, String, MethodLocation), Throwable> {
    match pool_entry {
        ConstantEntry::Method(data) => {
            let name_and_type = data.name_and_type.resolve();
            let method_name = name_and_type.name.resolve().try_string()?;

            let method_descriptor = name_and_type.descriptor.resolve().try_string()?;
            let method_descriptor = MethodType::parse(method_descriptor)?;

            let class = data.class.resolve();
            let class = class.name.resolve().try_string()?;

            Ok((
                MethodDescriptor::new(method_name, method_descriptor),
                class,
                MethodLocation::Class,
            ))
        }
        ConstantEntry::InterfaceMethod(data) => {
            let name_and_type = data.name_and_type.resolve();
            let method_name = name_and_type.name.resolve().try_string()?;

            let method_descriptor = name_and_type.descriptor.resolve().try_string()?;
            let method_descriptor = MethodType::parse(method_descriptor)?;

            let class = data.class.resolve();
            let class = class.name.resolve().try_string()?;

            Ok((
                MethodDescriptor::new(method_name, method_descriptor),
                class,
                MethodLocation::Interface,
            ))
        }
        e => Err(internal!(
            "expected interface method / method, got {:#?}",
            e
        )),
    }
}

fn clone_args_from_operands(
    descriptor: &MethodType,
    ctx: &mut Context,
) -> Result<Vec<RuntimeValue>, Throwable> {
    let mut reversed_descriptor = descriptor.clone();
    reversed_descriptor.parameters.reverse();
    let mut args = Vec::new();

    for _arg in reversed_descriptor.parameters.iter() {
        let popped_arg = ctx.operands.pop().ok_or(internal!("not enough args"))?;

        if let Some(int) = popped_arg.as_integral() {
            if int.ty == IntegralType::Long {
                args.push(popped_arg.clone());
            }
        }

        if let Some(float) = popped_arg.as_floating() {
            if float.ty == FloatingType::Double {
                args.push(popped_arg.clone());
            }
        }

        // TODO: Type check args properly

        args.push(popped_arg.clone());
    }

    Ok(args)
}

fn do_java_call(vm: &VM, ctx: Context) -> Result<Option<RuntimeValue>, Throwable> {
    // The new frame is then made current, and the Java Virtual Machine pc is set
    // to the opcode of the first instruction of the method to be invoked.
    // Execution continues with the first instruction of the method.
    let code = ctx.code.clone();
    let class = ctx.class.clone();
    let res = vm.run(ctx);

    // See if this code threw a (runtime // java) exception
    // If so, see if we (the caller) can handle it
    if let Err((err, state)) = &res {
        // Only try to catch runtime errors. Internal errors should never be caught by user code.
        if let Throwable::Runtime(rte) = err {
            if let Some(entry) = err.caught_by(vm, &code, state)? {
                let re_enter_context = Context {
                    code: code.clone(),
                    class,
                    is_reentry: true,
                    pc: entry.handler_pc as i32,
                    // Push the exception object as the first operand
                    operands: vec![rte.obj.clone()],
                    locals: state.locals.clone(),
                };

                info!("Re-entering at {}", re_enter_context.pc);

                return do_java_call(vm, re_enter_context);
            }
        }

        // We couldn't handle the exception

        // Pop our frame, we are no longer part of the callstack.
        // This is only to correctly handle the re-entry case, in which a caller catches the exception
        // and adds new frames. The callstack is cloned when the exception is thrown, which will preserve
        // the state at that point
        vm.pop_frame();
    }

    // Pass the result to our caller
    res.map_err(|e| e.0)
}

fn do_call(
    vm: &VM,
    method: Method,
    class: RefTo<Class>,
    args: Vec<RuntimeValue>,
) -> Result<Option<RuntimeValue>, Throwable> {
    if !method.flags.has(MethodAccessFlag::NATIVE) {
        // Must load the context if and only if the method is not native.
        // Native methods do not have a code attribute.
        let code = method
            .attributes
            .known_attribute::<CodeAttribute>(&class.unwrap_ref().class_file().constant_pool)?;

        let ctx = Context {
            code: code.clone(),
            class: class.clone(),
            pc: 0,
            is_reentry: false,
            operands: vec![],
            locals: args.clone(),
        };

        do_java_call(vm, ctx)
    } else {
        let method_name = method.name.resolve().string();
        let method_descriptor = method.descriptor.resolve().string();

        let module = class
            .unwrap_ref()
            .native_module()
            .as_ref()
            .ok_or(internal!(
                "no native module on {} (when calling {:?} {} / {})",
                class.unwrap_ref().name(),
                method.flags.flags,
                method_name,
                method_descriptor
            ))?;

        let mut module = module.borrow_mut();
        let lookup = module
            .get_method(
                (method_name.clone(), method_descriptor.clone())
                    .try_into()
                    .unwrap(),
            )
            .ok_or(internal!(
                "no native method {} {:?} {} / {}",
                class.unwrap_ref().name(),
                method.flags.flags,
                method_name,
                method_descriptor
            ))?;

        match lookup {
            NativeFunction::Static(func) => func(class.clone(), args, vm),
            NativeFunction::Instance(func) => {
                let this_ref = args.get(0).unwrap().as_object().unwrap().clone();
                func(this_ref, args, vm)
            }
        }
    }
}

#[derive(Debug)]
pub struct Athrow;

impl Instruction for Athrow {
    fn handle(&self, vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        let _throwable = pop!(ctx);
        let throwable = _throwable
            .as_object()
            .context("throwable was not an object")?
            .clone();

        let throwable =
            throwable
                .to_ref()
                .ok_or(vm.try_make_error(VMError::NullPointerException {
                    ctx: "when throwing".to_string(),
                })?)?;

        let class = throwable.header().class.clone();
        let class_name = class.unwrap_ref().name();

        let message: FieldRef<RefTo<Object>> = throwable
            .field(&("detailMessage", "Ljava/lang/String;").try_into().unwrap())
            .unwrap();

        let message = message.unwrap_ref();
        // assert_eq!(message.borrow().ty, ObjectType::String);
        // Safety: Pray
        let message = unsafe { message.cast::<BuiltinString>() };

        let message = message
            .into_option()
            .map(|m| {
                let bytes = m.value.unwrap_ref();
                let bytes = bytes.slice().to_vec();

                decode_string((CompactEncoding::from_coder(m.coder), bytes))
            })
            .unwrap_or(Ok("".to_string()))?;

        Ok(Progression::Throw(Throwable::Runtime(RuntimeException {
            message: format!("{}: {}", class_name, message),
            ty: class,
            obj: _throwable,
            sources: vm.frames(),
        })))
    }
}

#[derive(Debug)]
pub struct TableSwitch {
    pub default: i32,
    pub low: i32,
    pub high: i32,
    pub table: Vec<i32>,
}

impl Instruction for TableSwitch {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        // The index must be of type int and is popped from the operand stack.
        let index = arg!(ctx, "index" => i32);
        let index = index.value as i32;

        let target_address = if index > self.high || index < self.low {
            // If index is less than low or index is greater than high, then
            // a target address is calculated by adding default to the address of
            // the opcode of this tableswitch instruction.
            ctx.pc + self.default
        } else {
            // Otherwise, the offset at position index - low of the jump table is extracted.
            let table_index = index - self.low;

            // The target address is calculated by adding that offset to the address of the
            // opcode of this tableswitch instruction.
            ctx.pc + self.table[table_index as usize]
        };

        // Execution then continues at the target address.
        // NOTE: We could use JumpRel and not do the manual ctx.pc addition above,
        //       but it feels easier to handle this complex jumping logic here and
        //       not get the caller involved in address calculation.
        Ok(Progression::JumpAbs(target_address))
    }
}

#[derive(Debug)]
pub struct LookupSwitch {
    pub default: i32,
    pub pairs: Vec<(i32, i32)>,
}

impl Instruction for LookupSwitch {
    fn handle(&self, _vm: &VM, ctx: &mut Context) -> Result<Progression, Throwable> {
        // The key must be of type int and is popped from the operand stack.
        let key = arg!(ctx, "key" => i32);
        let key = key.value as i32;

        // The key is compared against the match values.
        let found = self.pairs.iter().find(|p| p.0 == key).copied();

        let target_address: i32;

        if let Some(found) = found {
            // If it is equal to one of them, then a target address is calculated by adding
            // the corresponding offset to the address of the opcode of this lookupswitch instruction
            target_address = ctx.pc + found.1;
        } else {
            // If the key does not match any of the
            // match values, the target address is calculated by adding default
            // to the address of the opcode of this lookupswitch instruction.
            target_address = ctx.pc + self.default;
        }

        // Execution then continues at the target address.
        // NOTE: We could use JumpRel and not do the manual ctx.pc addition above,
        //       but it feels easier to handle this complex jumping logic here and
        //       not get the caller involved in address calculation.
        Ok(Progression::JumpAbs(target_address))
    }
}
