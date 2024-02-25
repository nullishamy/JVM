#![feature(pointer_byte_offsets)]
#![feature(offset_of)]
#![allow(clippy::new_without_default)]

use std::cell::RefCell;

use bytecode::decode_instruction;
use bytes::BytesMut;

use runtime::{
    error::{Throwable, ThrownState, VMError},
    object::{
        loader::ClassLoader,
        value::RuntimeValue,
    },
    vm::{Context, Executor, VM},
};
use tracing::{debug, info, trace};

pub mod bytecode;

pub struct BootOptions {
    pub max_stack: u64,
}

pub struct Interpreter {
    options: BootOptions,
}

impl Interpreter {
    pub fn new(options: BootOptions) -> Self {
        Self { options }
    }
}

impl Executor for Interpreter {
    fn run(
        &self,
        vm: &VM,
        mut ctx: Context,
    ) -> Result<Option<RuntimeValue>, (Throwable, ThrownState)> {
        let is_overflowing_in_different_method = {
            let is_overflowing = vm.frames().len() > self.options.max_stack as usize;
            is_overflowing && !ctx.is_reentry
        };

        if is_overflowing_in_different_method {
            return Err(vm
                .try_make_error(VMError::StackOverflowError {})
                .map_err(|e| {
                    (
                        e,
                        ThrownState {
                            pc: ctx.pc,
                            locals: ctx.locals.clone(),
                        },
                    )
                })
                .map(|e| {
                    (
                        e,
                        ThrownState {
                            pc: ctx.pc,
                            locals: ctx.locals.clone(),
                        },
                    )
                })?);
        }

        while ctx.pc < ctx.code.code.len() as i32 {
            let slice = &ctx.code.code[ctx.pc as usize..];
            let consumed_bytes_prev = slice.len();

            let mut code_bytes = BytesMut::new();
            code_bytes.extend_from_slice(slice);

            let mut instruction_bytes = BytesMut::new();
            instruction_bytes.extend_from_slice(slice);

            let instruction =
                decode_instruction(vm, &mut instruction_bytes, &ctx).map_err(|e| {
                    (
                        e,
                        ThrownState {
                            pc: ctx.pc,
                            locals: ctx.locals.clone(),
                        },
                    )
                })?;

            let consumed_bytes_post = instruction_bytes.len();
            let bytes_consumed_by_opcode = (consumed_bytes_prev - consumed_bytes_post) as i32;
            trace!(
                "opcode: {:?} consumed {} bytes",
                instruction,
                bytes_consumed_by_opcode
            );

            let progression = instruction.handle(vm, &mut ctx).map_err(|e| {
                (
                    e,
                    ThrownState {
                        pc: ctx.pc,
                        locals: ctx.locals.clone(),
                    },
                )
            })?;

            match progression {
                bytecode::Progression::JumpAbs(new_pc) => {
                    debug!("Jumping from {} to {}", ctx.pc, new_pc);
                    ctx.pc = new_pc;
                }
                bytecode::Progression::JumpRel(offset) => {
                    debug!(
                        "Jumping from {} by {} (new: {})",
                        ctx.pc,
                        offset,
                        ctx.pc + offset
                    );
                    ctx.pc += offset;
                }
                bytecode::Progression::Next => {
                    debug!(
                        "Moving to next (jump by {} bytes)",
                        bytes_consumed_by_opcode
                    );
                    ctx.pc += bytes_consumed_by_opcode;
                }
                bytecode::Progression::Return(return_value) => {
                    debug!("Returning");
                    return Ok(return_value);
                }
                bytecode::Progression::Throw(err) => {
                    info!("Throwing {}", err);
                    return Err((
                        err,
                        ThrownState {
                            pc: ctx.pc,
                            locals: ctx.locals.clone(),
                        },
                    ));
                }
            };
        }

        Ok(None)
    }
}

impl Interpreter {
    pub fn bootstrap(&self, vm: &VM) -> Result<(), Throwable> {
        // Load native modules
        use runtime::native::*;

        fn load_module(classloader: &ClassLoader, mut m: impl NativeModule + 'static) {
            // Setup all the methods
            m.init();

            // Load the class specified by this module
            let cls = classloader
                .for_name(format!("L{};", m.classname()).into())
                .unwrap();

            // Just to stop us making errors with registration.
            if cls.unwrap_ref().native_module().is_some() {
                panic!("attempted to re-register module {}", m.classname());
            }

            // Apply the module to the class
            cls.with_lock(|cls| {
                cls.set_native_module(Box::new(RefCell::new(m)));
            });
        }

        load_module(vm.class_loader(), lang::LangClass::new());
        load_module(vm.class_loader(), lang::LangSystem::new());
        load_module(vm.class_loader(), lang::LangObject::new());
        load_module(vm.class_loader(), lang::LangShutdown::new());
        load_module(vm.class_loader(), lang::LangStringUtf16::new());
        load_module(vm.class_loader(), lang::LangRuntime::new());
        load_module(vm.class_loader(), lang::LangDouble::new());
        load_module(vm.class_loader(), lang::LangFloat::new());
        load_module(vm.class_loader(), lang::LangString::new());
        load_module(vm.class_loader(), lang::LangThrowable::new());
        load_module(vm.class_loader(), lang::LangStackTraceElement::new());
        load_module(vm.class_loader(), lang::LangClassLoader::new());
        load_module(vm.class_loader(), lang::LangThread::new());

        load_module(vm.class_loader(), jdk::JdkVM::new());
        load_module(vm.class_loader(), jdk::JdkReflection::new());
        load_module(vm.class_loader(), jdk::JdkCDS::new());
        load_module(vm.class_loader(), jdk::JdkSystemPropsRaw::new());
        load_module(vm.class_loader(), jdk::JdkUnsafe::new());
        load_module(vm.class_loader(), jdk::JdkSignal::new());
        load_module(vm.class_loader(), jdk::JdkScopedMemoryAccess::new());
        load_module(vm.class_loader(), jdk::JdkBootLoader::new());

        load_module(vm.class_loader(), io::IOFileDescriptor::new());
        load_module(vm.class_loader(), io::IOFileOutputStream::new());
        load_module(vm.class_loader(), io::IOUnixFileSystem::new());
        load_module(vm.class_loader(), io::IOFileInputStream::new());

        load_module(vm.class_loader(), security::SecurityAccessController::new());

        Ok(())
    }
}

#[cfg(test)]
mod tests {}
