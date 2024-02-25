use std::{collections::HashMap, process::exit, time::SystemTime};

use support::{
    encoding::{decode_string, CompactEncoding},
    types::MethodDescriptor,
};

use crate::{
    error::Throwable,
    instance_method, internal, module_base,
    object::{
        builtins::{Array, BuiltinString, Class, ClassType, Object},
        interner::intern_string,
        layout::types::{self},
        mem::{JavaObject, RefTo},
        numeric::{FALSE, TRUE},
        value::RuntimeValue,
    },
    static_method,
    vm::VM,
};

use super::{NativeFunction, NativeModule};

module_base!(LangClass);
impl NativeModule for LangClass {
    fn classname(&self) -> &'static str {
        "java/lang/Class"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn get_primitive_class(
            _: RefTo<Object>,
            args: Vec<RuntimeValue>,
            vm: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let prim_name = args.get(0).cloned().expect("no prim name");
            let prim_name = prim_name
                .as_object()
                .cloned()
                .expect("arg0 was not an object");

            let prim_name = unsafe { prim_name.cast::<BuiltinString>() };

            let bytes = prim_name.unwrap_ref().value.unwrap_ref().slice().to_vec();
            let prim_str = decode_string((
                CompactEncoding::from_coder(prim_name.unwrap_ref().coder),
                bytes,
            ))?;

            let prim_ty = match prim_str.as_str() {
                "byte" => types::BYTE,
                "float" => types::FLOAT,
                "double" => types::DOUBLE,
                "int" => types::INT,
                "char" => types::CHAR,
                "long" => types::LONG,
                "boolean" => types::BOOL,
                p => return Err(internal!("unknown primitive {}", p)),
            };

            let cls = vm.class_loader().for_name(prim_ty.name.into())?;

            Ok(Some(RuntimeValue::Object(cls.erase())))
        }

        self.set_method(
            ("getPrimitiveClass", "(Ljava/lang/String;)Ljava/lang/Class;"),
            instance_method!(get_primitive_class),
        );

        fn register_natives(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(None)
        }

        self.set_method(("registerNatives", "()V"), static_method!(register_natives));

        fn desired_assertion_status0(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(Some(RuntimeValue::Integral(TRUE)))
        }

        self.set_method(
            ("desiredAssertionStatus0", "(Ljava/lang/Class;)Z"),
            static_method!(desired_assertion_status0),
        );

        fn is_array(
            this: RefTo<Object>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let class = unsafe { this.cast::<Class>() };
            let result = if class.unwrap_ref().is_array() {
                1_i32
            } else {
                0_i32
            };

            Ok(Some(RuntimeValue::Integral(result.into())))
        }

        self.set_method(("isArray", "()Z"), instance_method!(is_array));

        fn is_interface(
            this: RefTo<Object>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let class = unsafe { this.cast::<Class>() };
            let result = if class.unwrap_ref().is_interface() {
                1_i32
            } else {
                0_i32
            };

            Ok(Some(RuntimeValue::Integral(result.into())))
        }

        self.set_method(("isInterface", "()Z"), instance_method!(is_interface));

        fn is_primitive(
            this: RefTo<Object>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let class = unsafe { this.cast::<Class>() };
            let result = if class.unwrap_ref().is_primitive() {
                1_i32
            } else {
                0_i32
            };

            Ok(Some(RuntimeValue::Integral(result.into())))
        }

        self.set_method(("isPrimitive", "()Z"), instance_method!(is_primitive));

        fn init_class_name(
            this: RefTo<Object>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let class = unsafe { this.cast::<Class>() };

            let name = class.unwrap_ref().name();

            Ok(Some(RuntimeValue::Object(
                intern_string(name.clone())?.erase(),
            )))
        }

        self.set_method(
            ("initClassName", "()Ljava/lang/String;"),
            instance_method!(init_class_name),
        );
    }
}

module_base!(LangSystem);
impl NativeModule for LangSystem {
    fn classname(&self) -> &'static str {
        "java/lang/System"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn register_natives(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(None)
        }

        self.set_method(("registerNatives", "()V"), static_method!(register_natives));

        fn nano_time(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let duration_since_epoch = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap();
            let timestamp_nanos = duration_since_epoch.as_nanos() as u64 as i64;

            Ok(Some(RuntimeValue::Integral(timestamp_nanos.into())))
        }

        self.set_method(("nanoTime", "()J"), static_method!(nano_time));

        fn identity_hash_code(
            _: RefTo<Class>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let obj = args.get(0).unwrap();
            let hash = obj.hash_code();

            Ok(Some(RuntimeValue::Integral(hash.into())))
        }

        self.set_method(
            ("identityHashCode", "(Ljava/lang/Object;)I"),
            static_method!(identity_hash_code),
        );

        fn set_in0(
            cls: RefTo<Class>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let stream = args.get(0).unwrap();
            let stream = stream.as_object().unwrap();

            let statics = cls.unwrap_ref().statics();
            let mut statics = statics.write();
            let field = statics.get_mut("in").unwrap();

            field.value = Some(RuntimeValue::Object(stream.clone()));

            Ok(None)
        }

        self.set_method(
            ("setIn0", "(Ljava/io/InputStream;)V"),
            static_method!(set_in0),
        );

        fn set_out0(
            cls: RefTo<Class>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let stream = args.get(0).unwrap();
            let stream = stream.as_object().unwrap();

            let statics = cls.unwrap_ref().statics();
            let mut statics = statics.write();
            let field = statics.get_mut("out").unwrap();

            field.value = Some(RuntimeValue::Object(stream.clone()));

            Ok(None)
        }

        self.set_method(
            ("setOut0", "(Ljava/io/PrintStream;)V"),
            static_method!(set_out0),
        );

        fn set_err0(
            cls: RefTo<Class>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let stream = args.get(0).unwrap();
            let stream = stream.as_object().unwrap();

            let statics = cls.unwrap_ref().statics();
            let mut statics = statics.write();
            let field = statics.get_mut("err").unwrap();

            field.value = Some(RuntimeValue::Object(stream.clone()));

            Ok(None)
        }

        self.set_method(
            ("setErr0", "(Ljava/io/PrintStream;)V"),
            static_method!(set_err0),
        );

        fn arraycopy(
            _: RefTo<Class>,
            args: Vec<RuntimeValue>,
            vm: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            use crate::object::layout::types::*;

            let (src, src_pos, dest, dest_pos, len) = (
                args.get(0)
                    .expect("no arg 0 (src)")
                    .as_object()
                    .expect("not an object"),
                args.get(1)
                    .expect("no arg 1 (src_pos)")
                    .as_integral()
                    .expect("not an integral"),
                args.get(2)
                    .expect("no arg 2 (dest)")
                    .as_object()
                    .expect("not an object"),
                args.get(3)
                    .expect("no arg 3 (dest_pos)")
                    .as_integral()
                    .expect("not an integral"),
                args.get(4)
                    .expect("no arg 4 (len)")
                    .as_integral()
                    .expect("not an integral"),
            );

            let src_pos = src_pos.value;
            let dest_pos = dest_pos.value;
            let len = len.value;

            let src_class = src.unwrap_ref().class();
            let src_ty = src_class.unwrap_ref();

            let dest_class = dest.unwrap_ref().class();
            let dest_ty = dest_class.unwrap_ref();

            assert_eq!(src_ty.ty(), ClassType::Array);
            assert_eq!(dest_ty.ty(), ClassType::Array);

            let _src_component = src_ty.component_type();
            let src_component = _src_component.unwrap_ref();

            let _dest_component = dest_ty.component_type();
            let dest_component = _dest_component.unwrap_ref();

            if src_pos < 0 {
                return Err(
                    vm.try_make_error(crate::error::VMError::ArrayIndexOutOfBounds { at: -1 })?
                );
            }

            if dest_pos < 0 {
                return Err(
                    vm.try_make_error(crate::error::VMError::ArrayIndexOutOfBounds { at: -1 })?
                );
            }

            if len < 0 {
                return Err(
                    vm.try_make_error(crate::error::VMError::ArrayIndexOutOfBounds { at: -1 })?
                );
            }

            let src_pos = src_pos as usize;
            let dest_pos = dest_pos as usize;
            let len = len as usize;

            fn write_prim<T: Copy>(
                src: &RefTo<Object>,
                dest: &RefTo<Object>,
                src_pos: usize,
                dest_pos: usize,
                len: usize,
            ) {
                let src = unsafe { src.cast::<Array<T>>() };
                let dest = unsafe { dest.cast::<Array<T>>() };

                // Same array special casing
                // If the src and dest arguments refer to the same array object, then the copying is performed as if the components at positions srcPos through srcPos+length-1 were
                // first copied to a temporary array with length components and then the contents of the temporary array were copied into positions destPos through destPos+length-1 of
                // the destination array.
                if src == dest {
                    let temp_array = src.with_lock(|src| {
                        let copy = src.slice().to_vec();
                        let ty = src.header().class();

                        Array::from_vec(ty, copy)
                    });

                    temp_array.with_lock(|tmp| {
                        dest.with_lock(|dest| {
                            let src_slice = tmp.slice();
                            let dest_slice = dest.slice_mut();
                            dest_slice[dest_pos..dest_pos + len]
                                .copy_from_slice(&src_slice[src_pos..src_pos + len]);
                        });
                    });

                    return;
                }

                src.with_lock(|src| {
                    dest.with_lock(|dest| {
                        let src_slice = src.slice();
                        let dest_slice = dest.slice_mut();

                        dest_slice[dest_pos..dest_pos + len]
                            .copy_from_slice(&src_slice[src_pos..src_pos + len]);
                    });
                });
            }

            if src_component.is_primitive() {
                assert!(dest_component.is_primitive());

                match src_component.name() {
                    n if { n == types::BOOL.name } => {
                        write_prim::<Bool>(src, dest, src_pos, dest_pos, len);
                    }
                    n if { n == types::BYTE.name } => {
                        write_prim::<Byte>(src, dest, src_pos, dest_pos, len);
                    }
                    n => todo!("implement {n}"),
                }
            } else {
                if !Class::can_assign(_src_component, _dest_component) {
                    panic!("array store exception")
                }

                let src = unsafe { src.cast::<Array<RefTo<Object>>>() };
                let dest = unsafe { dest.cast::<Array<RefTo<Object>>>() };

                // Same array special casing
                // If the src and dest arguments refer to the same array object, then the copying is performed as if the components at positions srcPos through srcPos+length-1 were
                // first copied to a temporary array with length components and then the contents of the temporary array were copied into positions destPos through destPos+length-1 of
                // the destination array.
                if src == dest {
                    let temp_array = src.with_lock(|src| {
                        let copy = src.slice().to_vec();
                        let ty = src.header().class();

                        Array::from_vec(ty, copy)
                    });

                    temp_array.with_lock(|tmp| {
                        dest.with_lock(|dest| {
                            let src_slice = tmp.slice();
                            let dest_slice = dest.slice_mut();
                            dest_slice[dest_pos..dest_pos + len]
                                .clone_from_slice(&src_slice[src_pos..src_pos + len]);
                        });
                    });

                    return Ok(None);
                }

                src.with_lock(|src| {
                    dest.with_lock(|dest| {
                        let src_slice = src.slice_mut();
                        let dest_slice = dest.slice_mut();
                        dest_slice[dest_pos..dest_pos + len]
                            .clone_from_slice(&src_slice[src_pos..src_pos + len]);
                    });
                });
            }

            Ok(None)
        }

        self.set_method(
            ("arraycopy", "(Ljava/lang/Object;ILjava/lang/Object;II)V"),
            static_method!(arraycopy),
        );
    }
}

module_base!(LangShutdown);
impl NativeModule for LangShutdown {
    fn classname(&self) -> &'static str {
        "java/lang/Shutdown"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn before_halt(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(None)
        }

        self.set_method(("beforeHalt", "()V"), static_method!(before_halt));

        fn halt0(
            _: RefTo<Class>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let exit_code = args
                .get(0)
                .expect("no arg 0")
                .as_integral()
                .expect("not an integral");
            exit(exit_code.value as i32);
        }

        self.set_method(("halt0", "(I)V"), static_method!(halt0));
    }
}

module_base!(LangObject);
impl NativeModule for LangObject {
    fn classname(&self) -> &'static str {
        "java/lang/Object"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn notify_all(
            _: RefTo<Object>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(None)
        }

        self.set_method(("notifyAll", "()V"), instance_method!(notify_all));

        fn clone(
            this: RefTo<Object>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            // TODO: Probably not the right semantics lmao
            Ok(Some(RuntimeValue::Object(this.clone())))
        }

        self.set_method(("clone", "()Ljava/lang/Object;"), instance_method!(clone));

        fn hash_code(
            this: RefTo<Object>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let rtv = RuntimeValue::Object(this);
            let hash: i32 = rtv.hash_code();

            Ok(Some(RuntimeValue::Integral(hash.into())))
        }

        self.set_method(("hashCode", "()I"), instance_method!(hash_code));

        fn get_class(
            this: RefTo<Object>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(Some(RuntimeValue::Object(
                this.unwrap_ref().class().erase(),
            )))
        }

        self.set_method(
            ("getClass", "()Ljava/lang/Class;"),
            instance_method!(get_class),
        );
    }
}

module_base!(LangStringUtf16);
impl NativeModule for LangStringUtf16 {
    fn classname(&self) -> &'static str {
        "java/lang/StringUTF16"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn is_big_endian(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            // We always encode in BE, for portability
            Ok(Some(RuntimeValue::Integral(TRUE)))
        }

        self.set_method(("isBigEndian", "()Z"), static_method!(is_big_endian));
    }
}

module_base!(LangString);
impl NativeModule for LangString {
    fn classname(&self) -> &'static str {
        "java/lang/String"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn intern(
            this: RefTo<Object>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let str = unsafe { this.cast::<BuiltinString>() };
            let str = str.unwrap_ref().string()?;
            let interned = intern_string(str)?;

            Ok(Some(RuntimeValue::Object(interned.erase())))
        }

        self.set_method(("intern", "()Ljava/lang/String;"), instance_method!(intern));
    }
}

module_base!(LangRuntime);
impl NativeModule for LangRuntime {
    fn classname(&self) -> &'static str {
        "java/lang/Runtime"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn available_processors(
            _: RefTo<Object>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            // TODO: Support MT and report this accurately
            Ok(Some(RuntimeValue::Integral(1_i32.into())))
        }

        self.set_method(
            ("availableProcessors", "()I"),
            instance_method!(available_processors),
        );

        fn max_memory(
            _: RefTo<Object>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            // TODO: Read this properly
            Ok(Some(RuntimeValue::Integral(1024_i64.into())))
        }

        self.set_method(("maxMemory", "()J"), instance_method!(max_memory));

        fn gc(
            _: RefTo<Object>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            // Our impl does not gc!
            Ok(None)
        }

        self.set_method(("gc", "()V"), instance_method!(gc));
    }
}

module_base!(LangStackTraceElement);
impl NativeModule for LangStackTraceElement {
    fn classname(&self) -> &'static str {
        "java/lang/StackTraceElement"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn init_stack_trace_elements(
            cls: RefTo<Class>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let (elements, throwable) = (
                unsafe {
                    args.get(0)
                        .unwrap()
                        .as_object()
                        .unwrap()
                        .cast::<Array<RefTo<StackTraceElement>>>()
                },
                args.get(1).unwrap().as_object().unwrap(),
            );

            let backtrace_field = throwable
                .unwrap_ref()
                .field::<RefTo<Object>>(&("backtrace", "Ljava/lang/Object;").try_into().unwrap())
                .unwrap();

            let back_trace = backtrace_field.unwrap_ref();
            let back_trace = unsafe { back_trace.cast::<Array<RefTo<StackTraceElement>>>() };

            // Move all of the backtrace data into the stackTrace (passed as elements) field.
            // This works because we set our internal backtrace field to just an array of StackTraceElement.
            // Note: The above assumption must hold for this to be safe.
            for (element, bt) in elements
                .unwrap_mut()
                .slice_mut()
                .iter_mut()
                .zip(back_trace.unwrap_ref().slice())
            {
                *element = bt.clone();

                // Garbage value, just so that it wont throw
                // FIXME: Find out what this is supposed to be set to.
                element.unwrap_mut().declaring_class_object = cls.clone();
            }

            Ok(None)
        }

        self.set_method(
            (
                "initStackTraceElements",
                "([Ljava/lang/StackTraceElement;Ljava/lang/Throwable;)V",
            ),
            static_method!(init_stack_trace_elements),
        );
    }
}

#[repr(C)]
#[derive(Debug)]
struct StackTraceElement {
    object: Object,

    declaring_class_object: RefTo<Class>,
    class_loader_name: RefTo<BuiltinString>,
    module_name: RefTo<BuiltinString>,
    module_version: RefTo<BuiltinString>,
    declaring_class: RefTo<BuiltinString>,
    method_name: RefTo<BuiltinString>,
    file_name: RefTo<BuiltinString>,
    line_number: types::Int,
    format: types::Byte,
}

impl JavaObject<StackTraceElement> for StackTraceElement {
    fn header(&self) -> &Object {
        &self.object
    }

    fn header_mut(&mut self) -> &mut Object {
        &mut self.object
    }

    fn type_name() -> &'static str {
        "java/lang/StackTraceElement"
    }
}

module_base!(LangThrowable);
impl NativeModule for LangThrowable {
    fn classname(&self) -> &'static str {
        "java/lang/Throwable"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {

        fn build_frames(vm: &VM) -> Vec<RefTo<StackTraceElement>> {
            let stack_trace_element_ty = vm
                .class_loader()
                .for_name("Ljava/lang/StackTraceElement;".try_into().unwrap())
                .unwrap();

            let mut raw_elements = vec![];
            for frame in vm.frames().iter().rev() {
                raw_elements.push(RefTo::new(StackTraceElement {
                    object: Object::new(
                        stack_trace_element_ty.clone(),
                        stack_trace_element_ty.unwrap_ref().super_class(),
                    ),
                    declaring_class_object: RefTo::null(),
                    class_loader_name: RefTo::null(),
                    module_name: RefTo::null(),
                    module_version: RefTo::null(),
                    declaring_class: intern_string(frame.class_name.clone()).unwrap(),
                    method_name: intern_string(frame.method_name.clone()).unwrap(),
                    file_name: RefTo::null(),
                    line_number: -1,
                    format: 0,
                }))
            }

            raw_elements
        }

        fn fill_in_stack_trace(
            _this: RefTo<Object>,
            _: Vec<RuntimeValue>,
            vm: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let this = _this.unwrap_ref();

            let raw_elements = build_frames(vm);
            let depth_field = this
                .field::<types::Int>(&("depth", "I").try_into().unwrap())
                .unwrap();
            depth_field.write(raw_elements.len() as i32);

            let stack_trace_element_arr_ty = vm
                .class_loader()
                .for_name("[Ljava/lang/StackTraceElement;".try_into().unwrap())
                .unwrap();
            let elements = Array::from_vec(stack_trace_element_arr_ty, raw_elements);

            // Internal field to store backtrace state. We will just store StackTraceElement data here for now.
            let backtrace_field = this
                .field::<RefTo<Object>>(&("backtrace", "Ljava/lang/Object;").try_into().unwrap())
                .unwrap();
            backtrace_field.write(elements.erase());

            Ok(Some(RuntimeValue::Object(_this)))
        }

        self.set_method(
            ("fillInStackTrace", "(I)Ljava/lang/Throwable;"),
            instance_method!(fill_in_stack_trace),
        );
    }
}

module_base!(LangFloat);
impl NativeModule for LangFloat {
    fn classname(&self) -> &'static str {
        "java/lang/Float"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn float_to_raw_int_bits(
            _: RefTo<Class>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let float = args.get(0).ok_or(internal!("no arg 0"))?;
            let float = float.as_floating().ok_or(internal!("not a float"))?;

            let result = (float.value as f32).to_bits();
            Ok(Some(RuntimeValue::Integral((result as i32).into())))
        }

        self.set_method(
            ("floatToRawIntBits", "(F)I"),
            static_method!(float_to_raw_int_bits),
        );
    }
}

module_base!(LangDouble);
impl NativeModule for LangDouble {
    fn classname(&self) -> &'static str {
        "java/lang/Double"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn double_to_raw_long_bits(
            _: RefTo<Class>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let float = args.get(0).ok_or(internal!("no arg 0"))?;
            let float = float.as_floating().ok_or(internal!("not a float"))?;

            let result = float.value.to_bits() as i64;
            Ok(Some(RuntimeValue::Integral(result.into())))
        }

        self.set_method(
            ("doubleToRawLongBits", "(D)J"),
            static_method!(double_to_raw_long_bits),
        );

        fn long_bits_to_double(
            _: RefTo<Class>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let long = args.get(0).ok_or(internal!("no arg 0"))?;
            let long = long.as_integral().ok_or(internal!("not an int"))?;

            let result = long.value as f64;
            Ok(Some(RuntimeValue::Floating(result.into())))
        }

        self.set_method(
            ("longBitsToDouble", "(J)D"),
            static_method!(long_bits_to_double),
        );
    }
}

module_base!(LangThread);
impl NativeModule for LangThread {
    fn classname(&self) -> &'static str {
        "java/lang/Thread"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn register_natives(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(None)
        }

        self.set_method(("registerNatives", "()V"), static_method!(register_natives));

        fn is_alive(
            _: RefTo<Object>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            // TODO: Would check in a real thread.
            Ok(Some(RuntimeValue::Integral(FALSE)))
        }

        self.set_method(("isAlive", "()Z"), instance_method!(is_alive));

        fn start0(
            _: RefTo<Object>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            // TODO: Start a thread here
            Ok(None)
        }

        self.set_method(("start0", "()V"), instance_method!(start0));

        fn set_priority0(
            _: RefTo<Object>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            // TODO: Would set the actual priority on a thread
            Ok(None)
        }

        self.set_method(("setPriority0", "(I)V"), instance_method!(set_priority0));

        fn current_thread(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            vm: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            // TODO: Actually read the thread object when we have threading
            Ok(Some(RuntimeValue::Object(vm.main_thread())))
        }

        self.set_method(
            ("currentThread", "()Ljava/lang/Thread;"),
            static_method!(current_thread),
        );
    }
}

module_base!(LangClassLoader);
impl NativeModule for LangClassLoader {
    fn classname(&self) -> &'static str {
        "java/lang/ClassLoader"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn register_natives(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(None)
        }

        self.set_method(("registerNatives", "()V"), static_method!(register_natives));
    }
}
