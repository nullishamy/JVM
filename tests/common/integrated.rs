use std::{
    fs,
    cell::RefCell,
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};

use interpreter::Interpreter;
use runtime::{
    error::{Frame, Throwable},
    native::{NativeFunction, NativeModule},
    object::{
        builtins::{BuiltinString, Class},
        interner::{set_interner, StringInterner},
        loader::ClassLoader,
        mem::RefTo,
        value::RuntimeValue,
    },
    static_method,
    vm::{VM, Context, Executor},
};
use support::{
    descriptor::{FieldType, ObjectType},
    types::MethodDescriptor,
    jar::JarFile
};
use tracing::Level;
use tracing_subscriber::fmt;

use super::CompiledClass;

const SOURCE_DIR: &str = env!("CARGO_MANIFEST_DIR");

pub fn make_vm() -> VM {
    let mut class_loader = ClassLoader::new();
    let stdjar = {
        let path = format!("{SOURCE_DIR}/../std.jar");
        let bytes = fs::read(path).unwrap();

        JarFile::from_bytes(bytes).unwrap()
    };

    class_loader.add_jar(stdjar);

    let bootstrapped_classes = class_loader
        .bootstrap()
        .expect("classloader bootstrap to succeed");

    let interner = StringInterner::new(
        bootstrapped_classes.java_lang_string.clone(),
        bootstrapped_classes.java_lang_object.clone(),
        bootstrapped_classes.byte_array_ty.clone(),
    );

    set_interner(interner);
    let mut vm = VM::new(
        class_loader
    );

    let interpreter = Interpreter::new(
        interpreter::BootOptions { max_stack: 50 },
    );

    interpreter.bootstrap(&vm).expect("natives to load (interpreter bootstrap)");
    vm.set_executor(Box::new(interpreter));
    vm.bootstrap().expect("vm to bootstrap");

    vm
}

pub fn load_test<const N: usize>(vm: &VM, class: impl Into<CompiledClass<N>>) -> RefTo<Class> {
    let class = class.into();
    vm.class_loader()
        .for_bytes(
            FieldType::Object(ObjectType {
                class_name: class.name().to_string(),
            }),
            class.bytes(),
        )
        .expect("class to load")
}

pub struct TestCaptures {
    methods: HashMap<MethodDescriptor, NativeFunction>,
    id: usize,
}

#[derive(Clone, Debug)]
pub struct CapturedOutput {
    cursor: usize,
    values: Vec<RuntimeValue>,
}

impl CapturedOutput {
    pub fn get(&self, index: usize) -> RuntimeValue {
        self.values
            .get(index)
            .cloned()
            .expect("index to be in range")
    }

    pub fn next(&mut self) -> RuntimeValue {
        let cr = self.cursor;
        self.cursor += 1;
        self.values.get(cr).unwrap().clone()
    }
}

static CAPTURE_COUNTER: AtomicUsize = AtomicUsize::new(0);
lazy_static::lazy_static! {
    static ref CAPTURE_STATE: Mutex<HashMap<usize, CapturedOutput>> = {
        Mutex::new(HashMap::new())
    };
}

pub fn get_captures(id: usize) -> CapturedOutput {
    let states = CAPTURE_STATE
        .lock()
        .expect("capture lock to be not poisoned");
    let state = states.get(&id);
    state.expect("state to exist after test execution").clone()
}

impl NativeModule for TestCaptures {
    fn classname(&self) -> &'static str {
        unimplemented!()
    }

    fn init(&mut self) {
        let id = self.id;

        let capture = move |_: RefTo<Class>,
                            args: Vec<RuntimeValue>,
                            _: &VM|
              -> Result<Option<RuntimeValue>, Throwable> {
            let mut states = CAPTURE_STATE
                .lock()
                .expect("capture lock to be not poisoned");
            let state = states
                .get_mut(&id)
                .expect("state to exist during test execution");
            state
                .values
                .push(args.get(0).expect("capture arg to be passed").clone());

            Ok(None)
        };

        self.set_method(("capture", "(D)V"), static_method!(capture));
        self.set_method(("capture", "(I)V"), static_method!(capture));
        self.set_method(("capture", "(Ljava/lang/String;)V"), static_method!(capture));
        self.set_method(("capture", "(Ljava/lang/Object;)V"), static_method!(capture));
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }
}

pub fn attach_utils(class: RefTo<Class>) -> usize {
    let id = CAPTURE_COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut module = TestCaptures {
        id,
        methods: HashMap::new(),
    };

    module.init();

    let mut states = CAPTURE_STATE
        .lock()
        .expect("capture lock to not be poisoned");
    states.insert(id, CapturedOutput { cursor: 0, values: vec![] });
    class
        .unwrap_mut()
        .set_native_module(Box::new(RefCell::new(module)));

    id
}

pub fn execute_test(vm: &VM, cls: RefTo<Class>, capture_id: usize) -> CapturedOutput {
    let main_ty: MethodDescriptor = ("runTest", "()V").try_into().unwrap();
    let ctx = Context::for_method(&main_ty, cls.clone());

    vm.push_frame(Frame {
        class_name: cls.unwrap_ref().name().to_string(),
        method_name: "runTest".to_string(),
    });


    let format = fmt::format()
        .with_ansi(true)
        .without_time()
        .with_level(true)
        .with_target(false)
        .with_thread_names(false)
        .with_source_location(true)
        .compact();

    let _ = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .event_format(format)
        .with_writer(std::io::stderr)
        .try_init();
    
    let res = vm.run(ctx);
    let captures = get_captures(capture_id);
    if let Err(e) = res {
        eprintln!("Execution encountered error:");
        eprintln!("{:#?}", e);
        eprintln!("Captures:");
        for capture in captures.values.iter() {
            match capture {
                RuntimeValue::Object(o) => {
                    let cls = o.unwrap_ref().class();
                    let name = cls.unwrap_ref().name();
                    match name.as_str() {
                        "java/lang/String" => {
                            let str = unsafe { o.cast::<BuiltinString>() };
                            eprintln!("\"{}\"", str.unwrap_ref().string().unwrap());
                        }
                        _ => eprintln!("{:#?}", o),
                    }
                },
                v => eprintln!("{:?}", v)
            }
        }
        panic!();
    }

    captures
}

#[track_caller]
pub fn iassert_eq(lhs: i64, rhs: RuntimeValue) {
    let val = rhs.as_integral().expect("was not an integral").value;
    assert_eq!(lhs, val);
}

#[track_caller]
pub fn dassert_eq(lhs: f64, rhs: RuntimeValue) {
    let val = rhs.as_floating().expect("was not a floating value").value;
    assert_eq!(lhs, val);
}

#[track_caller]
pub fn sassert_eq(lhs: impl Into<String>, rhs: RuntimeValue) {
    let val = rhs.as_object().expect("was not an object").clone();
    let str = unsafe { val.cast::<BuiltinString>() };
    let str_val = str.unwrap_ref().string().expect("could not decode string");

    assert_eq!(lhs.into(), str_val);
}


#[track_caller]
pub fn assert_null(val: RuntimeValue) {
    let obj = val.as_object().expect("not an object");
    assert!(obj.is_null());
}

#[track_caller]
pub fn assert_not_null(val: RuntimeValue) {
    let obj = val.as_object().expect("not an object");
    assert!(obj.is_not_null());
}
