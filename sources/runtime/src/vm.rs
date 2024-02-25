use std::cell::{OnceCell, RefCell};


use parse::attributes::CodeAttribute;
use support::types::MethodDescriptor;
use tracing::debug;

use crate::{
    error::{self, Frame, Throwable, ThrownState, VMError},
    object::{
        builtins::{Class, Object, BuiltinThread, BuiltinThreadGroup},
        loader::ClassLoader,
        mem::RefTo,
        value::RuntimeValue, interner::intern_string,
    },
};

pub struct Context {
    pub code: CodeAttribute,
    pub class: RefTo<Class>,

    pub pc: i32,
    pub is_reentry: bool,
    pub operands: Vec<RuntimeValue>,
    pub locals: Vec<RuntimeValue>,
}

impl Context {
    pub fn for_method(descriptor: &MethodDescriptor, class: RefTo<Class>) -> Self {
        let class_file = class.unwrap_ref().class_file();
        let method = class_file.methods.locate(descriptor).unwrap();

        let code = method
            .attributes
            .known_attribute::<CodeAttribute>(&class_file.constant_pool)
            .unwrap();

        Self {
            class,
            code,
            pc: 0,
            is_reentry: false,
            operands: vec![],
            locals: vec![],
        }
    }

    pub fn set_locals(&mut self, args: Vec<RuntimeValue>) {
        self.locals = args;
    }
}

pub trait Executor {
    fn run(&self, vm: &VM, ctx: Context) -> Result<Option<RuntimeValue>, (Throwable, ThrownState)>;
}

pub struct VM {
    class_loader: ClassLoader,
    frames: RefCell<Vec<Frame>>,

    main_thread: RefTo<Object>,
    executor: OnceCell<Box<dyn Executor>>,
}

impl VM {
    pub fn new(class_loader: ClassLoader) -> Self {
        Self {
            class_loader,
            frames: RefCell::new(vec![]),
            main_thread: RefTo::null(),
            executor: OnceCell::new(),
        }
    }

    pub fn set_executor(&mut self, new_executor: Box<dyn Executor>) {
        if self.executor.get().is_some() {
            panic!("Executor already set.")
        }

        if self.executor.set(new_executor).is_err() {
            panic!("Failed to set executor")
        }
    }

    pub fn set_main_thread(&mut self, new_thread: RefTo<Object>) {
        if self.main_thread.is_not_null() {
            panic!("Cannot set main thread more than once");
        }

        self.main_thread = new_thread;
    }

    pub fn class_loader(&self) -> &ClassLoader {
        &self.class_loader
    }

    pub fn frames(&self) -> Vec<Frame> {
        self.frames.borrow().clone()
    }

    pub fn push_frame(&self, frame: Frame) {
        self.frames.borrow_mut().push(frame);
    }

    pub fn pop_frame(&self) {
        self.frames.borrow_mut().pop();
    }

    /// Try and make the error. This may fail if a class fails to resolve, or object creation fails
    pub fn try_make_error(&self, ty: VMError) -> Result<Throwable, Throwable> {
        let cls = self
            .class_loader
            .for_name(format!("L{};", ty.class_name()).into())?;

        Ok(Throwable::Runtime(error::RuntimeException {
            message: ty.message(),
            ty: cls,
            obj: RuntimeValue::null_ref(),
            sources: self.frames.borrow().clone(),
        }))
    }

    pub fn main_thread(&self) -> RefTo<Object> {
        self.main_thread.clone()
    }

    pub fn run(&self, ctx: Context) -> Result<Option<RuntimeValue>, (Throwable, ThrownState)> {
        let executor = self.executor.get().unwrap();
        executor.run(self, ctx)
    }

    pub fn initialise_class(&self, class: RefTo<Class>) -> Result<(), Throwable> {
        let class_name = class.unwrap_ref().name().clone();

        if class.unwrap_ref().is_initialised() {
            debug!(
                "Not initialising {}, class is already initialised",
                class_name
            );

            return Ok(());
        }

        let clinit = class
            .unwrap_ref()
            .class_file()
            .methods
            .locate(&("<clinit>", "()V").try_into().unwrap())
            .cloned();

        if let Some(clinit) = clinit {
            debug!("Initialising {}", class_name);

            // Need to drop our lock on the class object before running the class initialiser
            // as it could call instructions which access class data
            class.with_lock(|class| {
                class.set_initialised(true);
            });

            let code = clinit
                .attributes
                .known_attribute(&class.unwrap_ref().class_file().constant_pool)?;

            let ctx = Context {
                code,
                class,
                pc: 0,
                is_reentry: false,
                operands: vec![],
                locals: vec![],
            };

            self.push_frame(Frame {
                method_name: "<clinit>".to_string(),
                class_name: class_name.clone(),
            });

            let res = self.run(ctx).map_err(|e| e.0);
            res?;

            debug!("Finished initialising {}", class_name);
            self.pop_frame();
        } else {
            debug!("No clinit in {}", class_name);
            // Might as well mark this as initialised to avoid future
            // needless method lookups
            class.with_lock(|class| {
                class.set_initialised(true);
            });
        }

        Ok(())
    }

    pub fn bootstrap(&mut self) -> Result<(), Throwable> {
        // Init String so that we can set the static after it's done. The clinit sets it to a default.
        let jlstr = self.class_loader().for_name("Ljava/lang/String;".into())?;
        self.initialise_class(jlstr.clone())?;

        // Load up System so that we can set up the statics
        let jlsys = self.class_loader().for_name("Ljava/lang/System;".into())?;

        {
            let statics = jlstr.unwrap_ref().statics();
            let mut statics = statics.write();
            let field = statics.get_mut("COMPACT_STRINGS").unwrap();

            field.value = Some(RuntimeValue::Integral(0_i32.into()));
        }

        {
            let statics = jlsys.unwrap_ref().statics();
            let mut statics = statics.write();
            // indicates if a security manager is possible
            // private static final int NEVER = 1;
            let field = statics
                .get_mut(&"allowSecurityManager".to_string())
                .unwrap();

            field.value = Some(RuntimeValue::Integral(1_i32.into()));
        }

        // Init thread
        let thread_class = self.class_loader().for_name("Ljava/lang/Thread;".into())?;
        self.initialise_class(thread_class.clone())?;

        let thread_group_class = self
            .class_loader()
            .for_name("Ljava/lang/ThreadGroup;".into())?;
        self.initialise_class(thread_class.clone())?;

        let thread = BuiltinThread {
            object: Object::new(
                thread_class.clone(),
                thread_class.unwrap_ref().super_class(),
            ),
            name: intern_string("main".to_string())?,
            priority: 1,
            daemon: 0,
            interrupted: 0,
            stillborn: 0,
            eetop: 0,
            target: RefTo::null(),
            thread_group: RefTo::new(BuiltinThreadGroup {
                object: Object::new(
                    thread_group_class.clone(),
                    thread_group_class.unwrap_ref().super_class(),
                ),
                parent: RefTo::null(),
                name: intern_string("main".to_string())?,
                max_priority: 0,
                destroyed: 0,
                daemon: 0,
                n_unstarted_threads: 0,
                n_threads: 0,
                threads: RefTo::null(),
                n_groups: 0,
                groups: RefTo::null(),
            }),
            context_class_loader: RefTo::null(),
            inherited_access_control_context: RefTo::null(),
            thread_locals: RefTo::null(),
            inheritable_thread_locals: RefTo::null(),
            stack_size: 0,
            tid: 1,
            status: 1,
            park_blocker: RefTo::null(),
            uncaught_exception_handler: RefTo::null(),
            thread_local_random_seed: 0,
            thread_local_random_probe: 0,
            thread_local_random_secondary_seed: 0,
        };

        let thread_ref = RefTo::new(thread);
        self.set_main_thread(thread_ref.erase());

        Ok(())
    }
}
