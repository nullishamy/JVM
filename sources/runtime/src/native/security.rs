use std::collections::HashMap;

use support::types::MethodDescriptor;

use crate::{
    error::Throwable,
    module_base,
    object::{builtins::Class, mem::RefTo, value::RuntimeValue},
    static_method,
    vm::VM,
};

use super::{NativeFunction, NativeModule};
module_base!(SecurityAccessController);
impl NativeModule for SecurityAccessController {
    fn classname(&self) -> &'static str {
        "java/security/AccessController"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn get_stack_access_control_context(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            // Null defined in the docs as "privileged code".
            Ok(Some(RuntimeValue::null_ref()))
        }

        self.set_method(
            (
                "getStackAccessControlContext",
                "()Ljava/security/AccessControlContext;",
            ),
            static_method!(get_stack_access_control_context),
        );
    }
}
