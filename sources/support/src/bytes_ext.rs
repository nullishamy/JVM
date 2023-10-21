/**
This macro builds a set of try_get_{number_type} functions for safe reading of
bytes from a Bytes object. They return Result<T> instead of panicking
 **/
macro_rules! impl_safebuf {
    ( $($type:ty),* ) => {
        pub trait SafeBuf: bytes::Buf {
            paste::paste! {
                $(
                fn [<try_get_ $type>](&mut self) -> anyhow::Result<$type>{
                    if self.remaining() >= std::mem::size_of::<$type>() {
                        Ok(self.[<get_ $type>]())
                    } else {
                        Err(anyhow::anyhow!("out of bytes"))
                    }
                }
                )*
            }
        }

        impl<T: bytes::Buf> SafeBuf for T { }
    }
}

impl_safebuf!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);
