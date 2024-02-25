use std::fmt;

use parse::{
    attributes::{CodeAttribute, ExceptionEntry},
    classfile::Resolvable,
};
use thiserror::Error;

use crate::{
    object::{builtins::Class, mem::RefTo, value::RuntimeValue},
    vm::VM,
};

pub enum VMError {
    ArrayIndexOutOfBounds { at: i64 },
    NullPointerException { ctx: String },
    StackOverflowError {},
    ClassCastException { from: String, to: String },
}

impl VMError {
    pub fn class_name(&self) -> &'static str {
        match self {
            VMError::ArrayIndexOutOfBounds { .. } => "java/lang/ArrayIndexOutOfBoundsException",
            VMError::NullPointerException { .. } => "java/lang/NullPointerException",
            VMError::StackOverflowError { .. } => "java/lang/StackOverflowError",
            VMError::ClassCastException { .. } => "java/lang/ClassCastException",
        }
    }

    pub fn message(&self) -> String {
        let ctx = match self {
            VMError::ArrayIndexOutOfBounds { at } => format!("OOB @ {}", at),
            VMError::NullPointerException { ctx } => format!("NPE ({})", ctx),
            VMError::StackOverflowError { .. } => {
                "thread main has overflowed its stack".to_string()
            }
            VMError::ClassCastException { from, to } => {
                format!("invalid cast from {} to {}", from, to)
            }
        };

        format!("{}: {}", self.class_name(), ctx)
    }
}

#[derive(Error, Debug)]
pub enum Throwable {
    #[error(transparent)]
    Runtime(RuntimeException),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

#[derive(Debug)]
pub struct ThrownState {
    pub pc: i32,
    pub locals: Vec<RuntimeValue>,
}

impl Throwable {
    pub fn caught_by<'a>(
        &'a self,
        vm: &VM,
        method: &'a CodeAttribute,
        state: &'a ThrownState,
    ) -> Result<Option<&'a ExceptionEntry>, Throwable> {
        if let Throwable::Runtime(rte) = self {
            let ty = &rte.ty;

            for entry in &method.exception_table {
                // The handler supports the type of the exception
                let has_type_match = if entry.catch_type.index() != 0 {
                    let entry_ty = {
                        let name = entry.catch_type.resolve().name.resolve().string();
                        vm.class_loader().for_name(format!("L{};", name).into())
                    }?;

                    Class::can_assign(entry_ty, ty.clone())
                } else {
                    // If the value of the catch_type item is zero, this exception handler is called for all exceptions.
                    true
                };

                // The handler covers the range of code we just called
                let has_range_match = (entry.start_pc..entry.end_pc).contains(&(state.pc as u16));

                if has_type_match && has_range_match {
                    return Ok(Some(entry));
                }
            }
        }

        Ok(None)
    }
}

#[macro_export]
macro_rules! internal {
    ($msg:literal $(,)?) => {
        $crate::error::Throwable::Internal(anyhow::anyhow!($msg))
    };
    ($err:expr $(,)?) => {
        $crate::error::Throwable::Internal(anyhow::anyhow!($err))
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::error::Throwable::Internal(anyhow::anyhow!($fmt, $($arg)*))
    };
}

#[macro_export]
macro_rules! internalise {
    () => {
        |f| $crate::internal!(f)
    };
}

#[derive(Error, Debug, Clone)]
#[error("at {class_name}.{method_name}")]
pub struct Frame {
    pub method_name: String,
    pub class_name: String,
}

#[derive(Error)]
#[error("{message}")]
pub struct RuntimeException {
    pub message: String,
    pub ty: RefTo<Class>,
    pub obj: RuntimeValue,
    pub sources: Vec<Frame>,
}

impl fmt::Debug for RuntimeException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeException")
            .field("message", &self.message)
            .field("ty", &&"<ty>")
            .field("sources", &self.sources)
            .finish()
    }
}
