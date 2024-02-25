use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read, Seek, Write},
    os::fd::{AsRawFd, FromRawFd},
};

use parking_lot::Mutex;
use support::types::MethodDescriptor;

use crate::{
    error::Throwable,
    instance_method, module_base,
    object::{
        builtins::{Array, BuiltinString, Class, Object},
        layout::types::Int,
        mem::{FieldRef, RefTo},
        numeric::FALSE,
        value::RuntimeValue,
    },
    static_method,
    vm::VM,
};

use super::{NativeFunction, NativeModule};

module_base!(IOFileDescriptor);
impl NativeModule for IOFileDescriptor {
    fn classname(&self) -> &'static str {
        "java/io/FileDescriptor"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn init_ids(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(None)
        }

        self.set_method(("initIDs", "()V"), static_method!(init_ids));

        fn get_handle(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            // Noop on Unix, would return handle on Windows.
            Ok(Some(RuntimeValue::Integral((-1_i64).into())))
        }

        self.set_method(("getHandle", "(I)J"), static_method!(get_handle));

        fn get_append(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            // TODO: Figure this one out
            Ok(Some(RuntimeValue::Integral(FALSE)))
        }

        self.set_method(("getAppend", "(I)Z"), static_method!(get_append));
    }
}

module_base!(IOFileOutputStream);
impl NativeModule for IOFileOutputStream {
    fn classname(&self) -> &'static str {
        "java/io/FileOutputStream"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn init_ids(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(None)
        }

        self.set_method(("initIDs", "()V"), static_method!(init_ids));

        fn write_bytes(
            this: RefTo<Object>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let fd = {
                let field: FieldRef<RefTo<Object>> = this
                    .unwrap_ref()
                    .field(&("fd", "Ljava/io/FileDescriptor;").try_into().unwrap())
                    .unwrap();

                let fd_obj = field.unwrap_ref();
                let fd_int: FieldRef<Int> = fd_obj
                    .unwrap_ref()
                    .field(&("fd", "I").try_into().unwrap())
                    .unwrap();

                fd_int.copy_out()
            };

            let bytes = {
                let val = args.get(1).unwrap();
                let val = val.as_object().unwrap();

                unsafe { val.cast::<Array<u8>>() }
            };

            let offset = {
                let val = args.get(2).unwrap();
                let val = val.as_integral().unwrap();
                val.value as i32
            };

            let length = {
                let val = args.get(3).unwrap();
                let val = val.as_integral().unwrap();
                val.value as i32
            };

            let _append = {
                let val = args.get(4).unwrap();
                let val = val.as_integral().unwrap();
                val.value as i32 != 0
            };

            let mut file = unsafe { File::from_raw_fd(fd) };
            let data_ptr = bytes.unwrap_ref().data_ptr();
            let data_start = unsafe { data_ptr.byte_add(offset as usize) };
            let buf = unsafe { &*std::ptr::slice_from_raw_parts(data_start, length as usize) };
            file.write_all(buf).unwrap();

            // Don't auto drop
            std::mem::forget(file);

            Ok(None)
        }

        self.set_method(("writeBytes", "([BIIZ)V"), instance_method!(write_bytes));
    }
}

module_base!(IOUnixFileSystem);
impl NativeModule for IOUnixFileSystem {
    fn classname(&self) -> &'static str {
        "java/io/UnixFileSystem"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn init_ids(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(None)
        }

        self.set_method(("initIDs", "()V"), static_method!(init_ids));
    }
}

lazy_static::lazy_static! {
    // FD -> File
    static ref FILES: Mutex<HashMap<Int, BufReader<File>>> = {
        Mutex::new(HashMap::from([
            (0, BufReader::new(unsafe { File::from_raw_fd(0) }) ),
            (1, BufReader::new(unsafe { File::from_raw_fd(1) }) ),
            (2, BufReader::new(unsafe { File::from_raw_fd(2) }) )
        ]))
    };
}

fn get_fd(this: &RefTo<Object>) -> FieldRef<Int> {
    let field: FieldRef<RefTo<Object>> = this
        .unwrap_ref()
        .field(&("fd", "Ljava/io/FileDescriptor;").try_into().unwrap())
        .unwrap();

    let fd_obj = field.unwrap_ref();
    let fd_int: FieldRef<Int> = fd_obj
        .unwrap_ref()
        .field(&("fd", "I").try_into().unwrap())
        .unwrap();

    fd_int
}

module_base!(IOFileInputStream);
impl NativeModule for IOFileInputStream {
    fn classname(&self) -> &'static str {
        "java/io/FileInputStream"
    }

    fn methods(&self) -> &HashMap<MethodDescriptor, NativeFunction> {
        &self.methods
    }

    fn methods_mut(&mut self) -> &mut HashMap<MethodDescriptor, NativeFunction> {
        &mut self.methods
    }

    fn init(&mut self) {
        fn init_ids(
            _: RefTo<Class>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            Ok(None)
        }

        self.set_method(("initIDs", "()V"), static_method!(init_ids));

        fn open0(
            this: RefTo<Object>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let fd_ref = get_fd(&this);

            let path = {
                let val = args.get(1).unwrap();
                let val = val.as_object().unwrap();
                unsafe { val.cast::<BuiltinString>() }
            };

            let file = File::open(path.unwrap_ref().string()?).unwrap();
            let raw = file.as_raw_fd();
            fd_ref.write(raw);

            let file = BufReader::new(file);
            let mut files = FILES.lock();
            files.insert(raw, file);

            Ok(None)
        }

        self.set_method(("open0", "(Ljava/lang/String;)V"), instance_method!(open0));

        fn length0(
            this: RefTo<Object>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let files = FILES.lock();
            let fd_ref = get_fd(&this);
            let file = files.get(&(fd_ref.copy_out())).unwrap();

            let meta = file.get_ref().metadata().unwrap();

            Ok(Some(RuntimeValue::Integral((meta.len() as i64).into())))
        }

        self.set_method(("length0", "()J"), instance_method!(length0));

        fn position0(
            this: RefTo<Object>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let mut files = FILES.lock();
            let fd_ref = get_fd(&this);
            let file = files.get_mut(&(fd_ref.copy_out())).unwrap();

            let pos = file.stream_position().unwrap();
            Ok(Some(RuntimeValue::Integral((pos as i64).into())))
        }

        self.set_method(("position0", "()J"), instance_method!(position0));

        fn read_bytes(
            this: RefTo<Object>,
            args: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let mut files = FILES.lock();
            let fd_ref = get_fd(&this);
            let file = files.get_mut(&(fd_ref.copy_out())).unwrap();

            let output_array = {
                let arr = args.get(1).unwrap().as_object().unwrap();
                unsafe { arr.cast::<Array<u8>>() }
            };

            let offset = {
                let num = args.get(2).unwrap().as_integral().unwrap();
                num.value as i32
            };

            let len = {
                let num = args.get(3).unwrap().as_integral().unwrap();
                num.value as i32
            };

            if len == 0 {
                return Ok(Some(RuntimeValue::Integral(0_i32.into())));
            }

            let read_n = output_array.with_lock(|array| {
                let subslice = {
                    let slice = array.slice_mut();
                    &mut slice[(offset as usize)..(len as usize)]
                };

                file.read(subslice).unwrap()
            });

            Ok(Some(RuntimeValue::Integral((read_n as i32).into())))
        }

        self.set_method(("readBytes", "([BII)I"), instance_method!(read_bytes));

        fn read0(
            this: RefTo<Object>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            let mut files = FILES.lock();
            let fd_ref = get_fd(&this);
            let file = files.get_mut(&(fd_ref.copy_out())).unwrap();

            let mut slice: [u8; 1] = [0];
            let read_n = file.read(&mut slice).unwrap();
            if read_n == 0 {
                // EOF
                Ok(Some(RuntimeValue::Integral((-1_i32).into())))
            } else {
                Ok(Some(RuntimeValue::Integral((slice[0] as i32).into())))
            }
        }

        self.set_method(("read0", "()I"), instance_method!(read0));

        fn available0(
            _: RefTo<Object>,
            _: Vec<RuntimeValue>,
            _: &VM,
        ) -> Result<Option<RuntimeValue>, Throwable> {
            // Returns an estimate of the number of remaining bytes that can be read (or skipped over) from this input stream without blocking by the
            // next invocation of a method for this input stream.
            // Returns 0 when the file position is beyond EOF. The next invocation might be the same thread or another thread.
            // A single read or skip of this many bytes will not block, but may read or skip fewer bytes.

            // 0 seems to work, idk?
            // TODO:
            Ok(Some(RuntimeValue::Integral((0_i32).into())))
        }

        self.set_method(("available0", "()I"), instance_method!(available0));
    }
}
