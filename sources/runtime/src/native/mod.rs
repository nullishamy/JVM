use std::collections::HashMap;

use support::types::MethodDescriptor;

use crate::{
    error::Throwable,
    object::{
        builtins::{Class, Object},
        mem::RefTo,
        value::RuntimeValue,
    },
    vm::VM,
};

pub mod io;
pub mod jdk;
pub mod lang;
pub mod security;

pub type NativeStaticFunction = Box<
    dyn FnMut(
        // this-class
        RefTo<Class>,
        // args
        Vec<RuntimeValue>,
        // VM
        &VM,
    ) -> Result<Option<RuntimeValue>, Throwable>,
>;

pub type NativeInstanceFunction = Box<
    dyn FnMut(
        // this
        RefTo<Object>,
        // args
        Vec<RuntimeValue>,
        // VM
        &VM,
    ) -> Result<Option<RuntimeValue>, Throwable>,
>;

pub enum NativeFunction {
    Static(NativeStaticFunction),
    Instance(NativeInstanceFunction),
}

pub trait NativeModule {
    fn classname(&self) -> &'static str;
    fn init(&mut self);

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction>;
    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction>;

    fn get_method(&mut self, method: MethodDescriptor) -> Option<&mut NativeFunction> {
        self.methods_mut().get_mut(&method)
    }

    // Weird signature for `method` because it cant use `impl TryInto<MethodDescriptor>` as this trait must be object safe.
    fn set_method(&mut self, method: (&'static str, &'static str), func: NativeFunction) {
        self.methods_mut().insert(method.try_into().unwrap(), func);
    }
}

pub struct DefaultNativeModule {
    methods: HashMap<MethodDescriptor, NativeFunction>,
    class_name: &'static str,
}

impl NativeModule for DefaultNativeModule {
    fn classname(&self) -> &'static str {
        self.class_name
    }

    fn init(&mut self) {}

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }
}

impl DefaultNativeModule {
    pub fn new(class_name: &'static str) -> Self {
        Self {
            methods: HashMap::new(),
            class_name,
        }
    }
}

#[macro_export]
macro_rules! static_method {
    ($method: expr) => {
        NativeFunction::Static(Box::new($method))
    };
}

#[macro_export]
macro_rules! instance_method {
    ($method: expr) => {
        NativeFunction::Instance(Box::new($method))
    };
}

#[macro_export]
macro_rules! module_base {
    ($ty: ident) => {
        pub struct $ty {
            methods: HashMap<MethodDescriptor, NativeFunction>,
        }

        impl $ty {
            pub fn new() -> Self {
                Self {
                    methods: HashMap::new(),
                }
            }
        }
    };
}
