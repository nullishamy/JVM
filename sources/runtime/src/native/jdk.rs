use std::{
    collections::HashMap,
    sync::atomic::{AtomicPtr, Ordering}, fmt,
};

use support::types::MethodDescriptor;

use crate::{
    error::Throwable,
    instance_method, module_base,
    object::{
        builtins::{Array, BuiltinString, Class, Object},
        interner::intern_string,
        layout::types,
        mem::RefTo,
        numeric::FALSE,
        value::RuntimeValue,
    },
    static_method,
    vm::VM,
};

use super::{NativeFunction, NativeModule};

module_base!(JdkVM);
impl NativeModule for JdkVM {
    fn classname(&self) -> &'static str {
        "jdk/internal/misc/VM"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn initialize(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(None)
        }

        self.set_method(("initialize", "()V"), static_method!(initialize));
    }
}

module_base!(JdkCDS);
impl NativeModule for JdkCDS {
    fn classname(&self) -> &'static str {
        "jdk/internal/misc/CDS"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn initialize_from_archive(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(Some(RuntimeValue::Integral(0_i64.into())))
        }

        self.set_method(
            ("initializeFromArchive", "(Ljava/lang/Class;)V"),
            static_method!(initialize_from_archive),
        );

        fn get_random_seed_for_dumping(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(Some(RuntimeValue::Integral(0_i64.into())))
        }

        self.set_method(
            ("getRandomSeedForDumping", "()J"),
            static_method!(get_random_seed_for_dumping),
        );

        fn is_dumping_class_list0(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(Some(RuntimeValue::Integral(FALSE)))
        }

        self.set_method(
            ("isDumpingClassList0", "()Z"),
            static_method!(is_dumping_class_list0),
        );

        fn is_dumping_archive0(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(Some(RuntimeValue::Integral(FALSE)))
        }

        self.set_method(
            ("isDumpingArchive0", "()Z"),
            static_method!(is_dumping_archive0),
        );

        fn is_sharing_enabled0(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(Some(RuntimeValue::Integral(FALSE)))
        }

        self.set_method(
            ("isSharingEnabled0", "()Z"),
            static_method!(is_sharing_enabled0),
        );
    }
}

module_base!(JdkReflection);
impl NativeModule for JdkReflection {
    fn classname(&self) -> &'static str {
        "jdk/internal/reflect/Reflection"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn get_caller_class(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            vm: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let mut frames = vm.frames();
            let current_frame = frames.pop().expect("no current frame");
            let current_class = current_frame.class_name;

            let first_frame_that_isnt_ours =
                frames.into_iter().find(|f| f.class_name != current_class);

            let cls = if let Some(frame) = first_frame_that_isnt_ours {
                Some(RuntimeValue::Object(
                    vm.class_loader()
                        .for_name(format!("L{};", frame.class_name).into())?
                        .erase(),
                ))
            } else {
                None
            };

            Ok(cls)
        }

        self.set_method(
            ("getCallerClass", "()Ljava/lang/Class;"),
            static_method!(get_caller_class),
        );
    }
}

/**
 * See: https://github.com/openjdk/jdk/blob/7d4b77ad9ee803d89eab5632f5c65ac843a68b3c/src/java.base/share/classes/jdk/internal/util/SystemProps.java#L217
 *
 * See: https://github.com/openjdk/jdk/blob/7d4b77ad9ee803d89eab5632f5c65ac843a68b3c/src/java.base/share/native/libjava/System.c#L107
 */
mod fields {
    pub const DISPLAY_COUNTRY_NDX: usize = 0;
    pub const DISPLAY_LANGUAGE_NDX: usize = 1 + DISPLAY_COUNTRY_NDX;
    pub const DISPLAY_SCRIPT_NDX: usize = 1 + DISPLAY_LANGUAGE_NDX;
    pub const DISPLAY_VARIANT_NDX: usize = 1 + DISPLAY_SCRIPT_NDX;

    pub const FILE_ENCODING_NDX: usize = 1 + DISPLAY_VARIANT_NDX;
    pub const FILE_SEPARATOR_NDX: usize = 1 + FILE_ENCODING_NDX;

    pub const FORMAT_COUNTRY_NDX: usize = 1 + FILE_SEPARATOR_NDX;
    pub const FORMAT_LANGUAGE_NDX: usize = 1 + FORMAT_COUNTRY_NDX;
    pub const FORMAT_SCRIPT_NDX: usize = 1 + FORMAT_LANGUAGE_NDX;
    pub const FORMAT_VARIANT_NDX: usize = 1 + FORMAT_SCRIPT_NDX;

    pub const FTP_NON_PROXY_HOSTS_NDX: usize = 1 + FORMAT_VARIANT_NDX;
    pub const FTP_PROXY_HOST_NDX: usize = 1 + FTP_NON_PROXY_HOSTS_NDX;
    pub const FTP_PROXY_PORT_NDX: usize = 1 + FTP_PROXY_HOST_NDX;

    pub const HTTP_NON_PROXY_HOSTS_NDX: usize = 1 + FTP_PROXY_PORT_NDX;
    pub const HTTP_PROXY_HOST_NDX: usize = 1 + HTTP_NON_PROXY_HOSTS_NDX;
    pub const HTTP_PROXY_PORT_NDX: usize = 1 + HTTP_PROXY_HOST_NDX;
    pub const HTTPS_PROXY_HOST_NDX: usize = 1 + HTTP_PROXY_PORT_NDX;
    pub const HTTPS_PROXY_PORT_NDX: usize = 1 + HTTPS_PROXY_HOST_NDX;

    pub const JAVA_IO_TMPDIR_NDX: usize = 1 + HTTPS_PROXY_PORT_NDX;
    pub const LINE_SEPARATOR_NDX: usize = 1 + JAVA_IO_TMPDIR_NDX;

    pub const OS_ARCH_NDX: usize = 1 + LINE_SEPARATOR_NDX;
    pub const OS_NAME_NDX: usize = 1 + OS_ARCH_NDX;
    pub const OS_VERSION_NDX: usize = 1 + OS_NAME_NDX;

    pub const PATH_SEPARATOR_NDX: usize = 1 + OS_VERSION_NDX;

    pub const SOCKS_NON_PROXY_HOSTS_NDX: usize = 1 + PATH_SEPARATOR_NDX;
    pub const SOCKS_PROXY_HOST_NDX: usize = 1 + SOCKS_NON_PROXY_HOSTS_NDX;
    pub const SOCKS_PROXY_PORT_NDX: usize = 1 + SOCKS_PROXY_HOST_NDX;

    pub const SUN_ARCH_ABI_NDX: usize = 1 + SOCKS_PROXY_PORT_NDX;
    pub const SUN_ARCH_DATA_MODEL_NDX: usize = 1 + SUN_ARCH_ABI_NDX;
    pub const SUN_CPU_ENDIAN_NDX: usize = 1 + SUN_ARCH_DATA_MODEL_NDX;
    pub const SUN_CPU_ISALIST_NDX: usize = 1 + SUN_CPU_ENDIAN_NDX;
    pub const SUN_IO_UNICODE_ENCODING_NDX: usize = 1 + SUN_CPU_ISALIST_NDX;
    pub const SUN_JNU_ENCODING_NDX: usize = 1 + SUN_IO_UNICODE_ENCODING_NDX;
    pub const SUN_OS_PATCH_LEVEL_NDX: usize = 1 + SUN_JNU_ENCODING_NDX;
    pub const SUN_STDERR_ENCODING_NDX: usize = 1 + SUN_OS_PATCH_LEVEL_NDX;
    pub const SUN_STDOUT_ENCODING_NDX: usize = 1 + SUN_STDERR_ENCODING_NDX;

    pub const USER_DIR_NDX: usize = 1 + SUN_STDOUT_ENCODING_NDX;
    pub const USER_HOME_NDX: usize = 1 + USER_DIR_NDX;
    pub const USER_NAME_NDX: usize = 1 + USER_HOME_NDX;

    pub const FIXED_LENGTH: usize = 1 + USER_NAME_NDX;
}

module_base!(JdkSystemPropsRaw);
impl NativeModule for JdkSystemPropsRaw {
    fn classname(&self) -> &'static str {
        "jdk/internal/util/SystemProps$Raw"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn vm_properties(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            vm: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            // TODO: Populate these properly

            let array_ty = vm
                .class_loader()
                .for_name("[Ljava/lang/String;".try_into().unwrap())?;
            let array: RefTo<Array<RefTo<BuiltinString>>> = Array::from_vec(
                array_ty,
                vec![
                    intern_string("java.home".to_string())?,
                    intern_string("unknown".to_string())?,
                    intern_string("native.encoding".to_string())?,
                    intern_string("UTF-8".to_string())?,
                    RefTo::null(),
                ],
            );

            Ok(Some(RuntimeValue::Object(array.erase())))
        }

        self.set_method(
            ("vmProperties", "()[Ljava/lang/String;"),
            static_method!(vm_properties),
        );

        fn platform_properties(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            vm: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let mut arr = Vec::with_capacity(fields::FIXED_LENGTH);
            arr.resize(fields::FIXED_LENGTH, RefTo::null());

            // TODO: Ask OS for temp file
            arr[fields::JAVA_IO_TMPDIR_NDX] = intern_string("/tmp/javaio.tmp".to_string())?;

            // TODO: Make this platform specific
            arr[fields::LINE_SEPARATOR_NDX] = intern_string("\n".to_string())?;
            arr[fields::PATH_SEPARATOR_NDX] = intern_string(":".to_string())?;
            arr[fields::FILE_SEPARATOR_NDX] = intern_string("/".to_string())?;

            // TODO: Resolve these
            arr[fields::USER_HOME_NDX] = intern_string("/GARBAGE".to_string())?;
            arr[fields::USER_DIR_NDX] = intern_string("/GARABAGE".to_string())?;

            // TODO: Actual username
            arr[fields::USER_NAME_NDX] = intern_string("admin".to_string())?;

            arr[fields::FILE_ENCODING_NDX] = intern_string("UTF-8".to_string())?;
            arr[fields::SUN_JNU_ENCODING_NDX] = intern_string("UTF-8".to_string())?;

            let array_ty = vm
                .class_loader()
                .for_name("[Ljava/lang/String;".try_into().unwrap())?;
            let array: RefTo<Array<RefTo<BuiltinString>>> = Array::from_vec(array_ty, arr);

            Ok(Some(RuntimeValue::Object(array.erase())))
        }

        self.set_method(
            ("platformProperties", "()[Ljava/lang/String;"),
            static_method!(platform_properties),
        );
    }
}
module_base!(JdkUnsafe);
impl NativeModule for JdkUnsafe {
    fn classname(&self) -> &'static str {
        "jdk/internal/misc/Unsafe"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn object_field_offset1(
            _: RefTo<Object>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let cls = {
                let val = args.get(1).unwrap();
                let val = val.as_object().unwrap();
                unsafe { val.cast::<Class>() }
            };

            let field = {
                let val = args.get(2).unwrap();
                let val = val.as_object().unwrap();
                unsafe { val.cast::<BuiltinString>() }
            };

            let layout = cls.unwrap_ref().instance_layout();
            let str = field.unwrap_ref().string()?;
            let info = layout.field_info(&str).expect("TODO: internal error");

            let offset = info.location.offset as i64;

            Ok(Some(RuntimeValue::Integral(offset.into())))
        }

        self.set_method(
            (
                "objectFieldOffset1",
                "(Ljava/lang/Class;Ljava/lang/String;)J",
            ),
            instance_method!(object_field_offset1),
        );

        fn register_natives(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(None)
        }

        self.set_method(("registerNatives", "()V"), static_method!(register_natives));

        fn store_fence(
            _: RefTo<Object>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(None)
        }

        self.set_method(("storeFence", "()V"), instance_method!(store_fence));

        fn compare_and_set<T: PartialEq + fmt::Debug>(
            object: &RefTo<Object>,
            offset: usize,
            expected: T,
            desired: T,
        ) -> bool {
            let ptr = object.unwrap_mut() as *mut Object;
            let ptr = unsafe { ptr.byte_add(offset) };
            let ptr = ptr.cast::<T>();

            let val = unsafe { ptr.read() };
            if expected == val {
                unsafe { ptr.write(desired) };
                true
            } else {
                false
            }
            // FIXME: Make atomic. Cannot figure out the semantics for this one. compare_exchange writes into the expected, which isn't what we want

            // let atomic_ptr = make_atomic_field_ptr(object, offset);

            // let old_value = atomic_ptr.load(Ordering::SeqCst);
            // dbg!(old_value, &expected as *const T);
            // if old_value == &mut expected {
            //     dbg!("here");
            //     atomic_ptr.store(&mut desired, Ordering::SeqCst);
            //     true
            // } else {
            //     false
            // }
        }

        fn make_atomic_field_ptr<T>(object: &RefTo<Object>, offset: usize) -> AtomicPtr<T> {
            let raw_ptr = object.unwrap_mut() as *mut Object;
            let raw_ptr = unsafe { raw_ptr.byte_add(offset) };
            let raw_ptr = raw_ptr.cast::<T>();

            AtomicPtr::new(raw_ptr)
        }

        fn check_cmp_args<T>(
            args: &[RuntimeValue],
            val_to_t: impl Fn(RuntimeValue) -> T,
        ) -> (&RefTo<Object>, usize, T, T) {
            let object = {
                let val = args.get(1).unwrap();
                val.as_object().unwrap()
            };

            let offset = {
                let val = args.get(2).unwrap();
                val.as_integral().unwrap().value
            };

            // Careful. Skip a slot (don't use 3), `long`s (offset) take 2 slots.
            let expected = {
                let val = args.get(4).unwrap().clone();
                val_to_t(val)
            };

            let desired = {
                let val = args.get(5).unwrap().clone();
                val_to_t(val)
            };

            (object, offset as usize, expected, desired)
        }

        fn compare_and_set_int(
            _: RefTo<Object>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let (object, offset, expected, desired) =
                check_cmp_args(&args, |v| v.as_integral().unwrap().value as types::Int);

            let did_write = compare_and_set::<types::Int>(object, offset, expected, desired);

            Ok(Some(RuntimeValue::Integral((did_write as i32).into())))
        }

        self.set_method(
            ("compareAndSetInt", "(Ljava/lang/Object;JII)Z"),
            instance_method!(compare_and_set_int),
        );

        fn compare_and_set_reference(
            _: RefTo<Object>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let (object, offset, expected, desired) =
                check_cmp_args(&args, |v| v.as_object().unwrap().clone());

            let did_write = compare_and_set::<RefTo<Object>>(object, offset, expected, desired);

            Ok(Some(RuntimeValue::Integral((did_write as i32).into())))
        }

        self.set_method(
            (
                "compareAndSetReference",
                "(Ljava/lang/Object;JLjava/lang/Object;Ljava/lang/Object;)Z",
            ),
            instance_method!(compare_and_set_reference),
        );

        fn compare_and_set_long(
            _: RefTo<Object>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            // Don't use the helpers, longs are finicky.

            let object = {
                let val = args.get(1).unwrap();
                val.as_object().unwrap()
            };

            let offset = {
                let val = args.get(2).unwrap();
                val.as_integral().unwrap().value
            };

            // Careful. Skip a slot. `long`s take up 2.
            let expected = {
                let val = args.get(4).unwrap();
                val.as_integral().unwrap().value
            };

            // Careful. Skip a slot. `long`s take up 2.
            let desired = {
                let val = args.get(6).unwrap();
                val.as_integral().unwrap().value
            };

                let did_write = compare_and_set::<types::Long>(object, offset as usize, expected, desired);

            Ok(Some(RuntimeValue::Integral((did_write as i32).into())))
        }

        self.set_method(
            ("compareAndSetLong", "(Ljava/lang/Object;JJJ)Z"),
            instance_method!(compare_and_set_long),
        );

        fn get_reference_volatile(
            _: RefTo<Object>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let object = {
                let val = args.get(1).unwrap();
                val.as_object().unwrap()
            };

            let offset = {
                let val = args.get(2).unwrap();
                val.as_integral().unwrap().value
            };

            let atomic_ptr = make_atomic_field_ptr::<RefTo<Object>>(object, offset as usize);
            let val = unsafe { atomic_ptr.load(Ordering::SeqCst).as_ref().unwrap().clone() };

            Ok(Some(RuntimeValue::Object(val)))
        }

        self.set_method(
            (
                "getReferenceVolatile",
                "(Ljava/lang/Object;J)Ljava/lang/Object;",
            ),
            instance_method!(get_reference_volatile),
        );

        fn get_reference(
            _: RefTo<Object>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let object = {
                let val = args.get(1).unwrap();
                val.as_object().unwrap()
            };

            let offset = {
                let val = args.get(2).unwrap();
                val.as_integral().unwrap().value
            };

            let raw_ptr = object.unwrap_mut() as *mut Object;
            let raw_ptr = unsafe { raw_ptr.byte_add(offset as usize) };
            let raw_ptr = raw_ptr.cast::<RefTo<Object>>();
            let val = unsafe { raw_ptr.as_ref().unwrap() }.clone();

            Ok(Some(RuntimeValue::Object(val)))
        }

        self.set_method(
            ("getReference", "(Ljava/lang/Object;J)Ljava/lang/Object;"),
            instance_method!(get_reference),
        );

        fn get_int_volatile(
            _: RefTo<Object>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let object = {
                let val = args.get(1).unwrap();
                val.as_object().unwrap()
            };

            let offset = {
                let val = args.get(2).unwrap();
                val.as_integral().unwrap().value
            };

            let atomic_ptr = make_atomic_field_ptr::<types::Int>(object, offset as usize);
            let val = unsafe { atomic_ptr.load(Ordering::SeqCst).read() };

            Ok(Some(RuntimeValue::Integral(val.into())))
        }

        self.set_method(
            ("getIntVolatile", "(Ljava/lang/Object;J)I"),
            instance_method!(get_int_volatile),
        );

        fn put_reference_volatile(
            _: RefTo<Object>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let object = {
                let val = args.get(1).unwrap();
                val.as_object().unwrap()
            };

            let offset = {
                let val = args.get(2).unwrap();
                val.as_integral().unwrap().value
            };

            let value = {
                let val = args.get(4).unwrap();
                val.as_object().unwrap().clone()
            };

            let raw_ptr = object.unwrap_mut() as *mut Object;
            let raw_ptr = unsafe { raw_ptr.byte_add(offset as usize) };
            let raw_ptr = raw_ptr.cast::<RefTo<Object>>();
            unsafe { raw_ptr.write(value) };

            Ok(None)
        }

        self.set_method(
            (
                "putReferenceVolatile",
                "(Ljava/lang/Object;JLjava/lang/Object;)V",
            ),
            instance_method!(put_reference_volatile),
        );

        fn array_index_scale0(
            _: RefTo<Object>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            use crate::object::layout::types::*;
            let cls = args.get(1).unwrap();
            let cls = cls.as_object().unwrap();
            let cls = unsafe { cls.cast::<Class>() };

            let component = cls.unwrap_ref().component_type();
            let component = component.unwrap_ref();

            let res = if !component.is_primitive() {
                Array::<RefTo<Object>>::element_scale()
            } else {
                match component.name() {
                    n if { n == types::BOOL.name } => Array::<Bool>::element_scale(),
                    n if { n == types::BYTE.name } => Array::<Byte>::element_scale(),
                    n if { n == types::SHORT.name } => Array::<Short>::element_scale(),
                    n if { n == types::CHAR.name } => Array::<Char>::element_scale(),
                    n if { n == types::INT.name } => Array::<Int>::element_scale(),
                    n if { n == types::LONG.name } => Array::<Long>::element_scale(),
                    n if { n == types::DOUBLE.name } => Array::<Double>::element_scale(),
                    n if { n == types::FLOAT.name } => Array::<Float>::element_scale(),
                    n => todo!("implement {n}"),
                }
            };

            Ok(Some(RuntimeValue::Integral((res as i32).into())))
        }

        self.set_method(
            ("arrayIndexScale0", "(Ljava/lang/Class;)I"),
            instance_method!(array_index_scale0),
        );

        fn array_base_offset0(
            _: RefTo<Object>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            use crate::object::layout::types::*;
            let cls = args.get(1).unwrap();
            let cls = cls.as_object().unwrap();
            let cls = unsafe { cls.cast::<Class>() };

            let component = cls.unwrap_ref().component_type();
            let component = component.unwrap_ref();

            let res = if !component.is_primitive() {
                Array::<RefTo<Object>>::elements_offset()
            } else {
                match component.name() {
                    n if { n == types::BOOL.name } => Array::<Bool>::elements_offset(),
                    n if { n == types::BYTE.name } => Array::<Byte>::elements_offset(),
                    n if { n == types::SHORT.name } => Array::<Short>::elements_offset(),
                    n if { n == types::CHAR.name } => Array::<Char>::elements_offset(),
                    n if { n == types::INT.name } => Array::<Int>::elements_offset(),
                    n if { n == types::LONG.name } => Array::<Long>::elements_offset(),
                    n if { n == types::DOUBLE.name } => Array::<Double>::elements_offset(),
                    n if { n == types::FLOAT.name } => Array::<Float>::elements_offset(),
                    n => todo!("implement {n}"),
                }
            };

            Ok(Some(RuntimeValue::Integral((res as i32).into())))
        }

        self.set_method(
            ("arrayBaseOffset0", "(Ljava/lang/Class;)I"),
            instance_method!(array_base_offset0),
        );
    }
}

module_base!(JdkScopedMemoryAccess);
impl NativeModule for JdkScopedMemoryAccess {
    fn classname(&self) -> &'static str {
        "jdk/internal/misc/ScopedMemoryAccess"
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

module_base!(JdkSignal);
impl NativeModule for JdkSignal {
    fn classname(&self) -> &'static str {
        "jdk/internal/misc/Signal"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn handle0(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            // TODO:
            Ok(Some(RuntimeValue::Integral(0_i64.into())))
        }

        self.set_method(("handle0", "(IJ)J"), static_method!(handle0));

        fn find_signal0(
            _: RefTo<Class>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let sig = args.get(0).unwrap();
            let sig = sig.as_object().unwrap();
            let sig = unsafe { sig.cast::<BuiltinString>() };
            let sig = sig.unwrap_ref().string()?;

            // TODO: Get actual signals
            let code: i32 = match sig.as_str() {
                "ABRT" => 0,
                "FPE" => 1,
                "ILL" => 2,
                "INT" => 3,
                "SEGV" => 4,
                "TERM" => 5,
                "HUP" => 6,
                _ => -1,
            };

            Ok(Some(RuntimeValue::Integral(code.into())))
        }

        self.set_method(
            ("findSignal0", "(Ljava/lang/String;)I"),
            static_method!(find_signal0),
        );
    }
}

module_base!(JdkBootLoader);
impl NativeModule for JdkBootLoader {
    fn classname(&self) -> &'static str {
        "jdk/internal/loader/BootLoader"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn set_boot_loader_unnamed_module0(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(None)
        }

        self.set_method(
            ("setBootLoaderUnnamedModule0", "(Ljava/lang/Module;)V"),
            static_method!(set_boot_loader_unnamed_module0),
        );
    }
}
