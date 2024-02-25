use std::fmt;

use crate::Context;

use bytes::{Buf, BytesMut};
use runtime::error::Throwable;
use runtime::internal;
use runtime::object::layout::types;
use runtime::object::value::RuntimeValue;
use runtime::vm::VM;
use support::bytes_ext::SafeBuf;

use self::ops::WideFormat;

mod binary;
mod invoke;
mod load_store;
mod ops;
mod unary;

pub enum Progression {
    JumpAbs(i32),
    JumpRel(i32),
    Next,
    Return(Option<RuntimeValue>),
    Throw(Throwable),
}

pub trait Instruction: fmt::Debug {
    fn handle(&self, _vm: &VM, _ctx: &mut Context) -> Result<Progression, Throwable> {
        Ok(Progression::Next)
    }
}

/// Utility to box a value. Used below to box each instruction that we decode
#[inline(always)]
fn b<T>(v: T) -> Box<T> {
    Box::new(v)
}

#[inline(always)]
pub fn decode_instruction(
    vm: &VM,
    bytes: &mut BytesMut,
    ctx: &Context,
) -> Result<Box<dyn Instruction>, Throwable> {
    let instruction = bytes.try_get_u8()?;
    let pc = ctx.pc;

    Ok(match instruction {
        0x00 => b(ops::Nop),

        // Constants / Loads / Stores
        0x01 => b(ops::PushConst {
            value: RuntimeValue::null_ref(),
        }),
        0x02 => b(ops::PushConst {
            value: RuntimeValue::Integral((-1_i32).into()),
        }),
        0x03 => b(ops::PushConst {
            value: RuntimeValue::Integral((0_i32).into()),
        }),
        0x04 => b(ops::PushConst {
            value: RuntimeValue::Integral((1_i32).into()),
        }),
        0x05 => b(ops::PushConst {
            value: RuntimeValue::Integral((2_i32).into()),
        }),
        0x06 => b(ops::PushConst {
            value: RuntimeValue::Integral((3_i32).into()),
        }),
        0x07 => b(ops::PushConst {
            value: RuntimeValue::Integral((4_i32).into()),
        }),
        0x08 => b(ops::PushConst {
            value: RuntimeValue::Integral((5_i32).into()),
        }),

        0x09 => b(ops::PushConst {
            value: RuntimeValue::Integral((0_i64).into()),
        }),
        0x0a => b(ops::PushConst {
            value: RuntimeValue::Integral((1_i64).into()),
        }),
        0x0b => b(ops::PushConst {
            value: RuntimeValue::Floating((0.0_f32).into()),
        }),
        0x0c => b(ops::PushConst {
            value: RuntimeValue::Floating((1.0_f32).into()),
        }),
        0x0d => b(ops::PushConst {
            value: RuntimeValue::Floating((2.0_f32).into()),
        }),

        0x0e => b(ops::PushConst {
            value: RuntimeValue::Floating((0.0_f64).into()),
        }),
        0x0f => b(ops::PushConst {
            value: RuntimeValue::Floating((1.0_f64).into()),
        }),
        0x10 => b(ops::PushConst {
            value: RuntimeValue::Integral((bytes.try_get_i8()? as i32).into()),
        }),
        0x11 => b(ops::PushConst {
            // The intermediate value is then sign-extended to an int value.
            value: RuntimeValue::Integral((bytes.try_get_i16()? as i32).into()),
        }),
        0x12 => b(ops::Ldc {
            index: bytes.try_get_u8()? as u16,
        }),
        0x13 => b(ops::Ldc {
            index: bytes.try_get_u16()?,
        }),
        0x14 => b(ops::Ldc2W {
            index: bytes.try_get_u16()?,
        }),
        0x15 => b(ops::LoadLocal {
            index: bytes.try_get_u8()? as usize,
        }),
        0x16 => b(ops::LoadLocal {
            index: bytes.try_get_u8()? as usize,
        }),
        0x17 => b(ops::LoadLocal {
            index: bytes.try_get_u8()? as usize,
        }),
        0x18 => b(ops::LoadLocal {
            index: bytes.try_get_u8()? as usize,
        }),
        0x19 => b(ops::LoadLocal {
            index: bytes.try_get_u8()? as usize,
        }),

        0x1a => b(ops::LoadLocal { index: 0 }),
        0x1b => b(ops::LoadLocal { index: 1 }),
        0x1c => b(ops::LoadLocal { index: 2 }),
        0x1d => b(ops::LoadLocal { index: 3 }),

        0x1e => b(ops::LoadLocal { index: 0 }),
        0x1f => b(ops::LoadLocal { index: 1 }),
        0x20 => b(ops::LoadLocal { index: 2 }),
        0x21 => b(ops::LoadLocal { index: 3 }),

        0x22 => b(ops::LoadLocal { index: 0 }),
        0x23 => b(ops::LoadLocal { index: 1 }),
        0x24 => b(ops::LoadLocal { index: 2 }),
        0x25 => b(ops::LoadLocal { index: 3 }),

        0x26 => b(ops::LoadLocal { index: 0 }),
        0x27 => b(ops::LoadLocal { index: 1 }),
        0x28 => b(ops::LoadLocal { index: 2 }),
        0x29 => b(ops::LoadLocal { index: 3 }),

        0x2a => b(ops::LoadLocal { index: 0 }),
        0x2b => b(ops::LoadLocal { index: 1 }),
        0x2c => b(ops::LoadLocal { index: 2 }),
        0x2d => b(ops::LoadLocal { index: 3 }),

        0x2e => b(ops::ArrayLoad {
            ty: vm.class_loader().for_name(types::INT.name.into())?,
        }),
        0x2f => b(ops::ArrayLoad {
            ty: vm.class_loader().for_name(types::LONG.name.into())?,
        }),
        0x30 => b(ops::ArrayLoad {
            ty: vm.class_loader().for_name(types::FLOAT.name.into())?,
        }),
        0x31 => b(ops::ArrayLoad {
            ty: vm.class_loader().for_name(types::DOUBLE.name.into())?,
        }),
        0x32 => b(ops::ArrayLoad {
            ty: vm.class_loader().for_name("Ljava/lang/Object;".into())?,
        }),
        0x33 => b(ops::ArrayLoad {
            ty: vm.class_loader().for_name(types::BYTE.name.into())?,
        }),
        0x34 => b(ops::ArrayLoad {
            ty: vm.class_loader().for_name(types::CHAR.name.into())?,
        }),
        0x35 => b(ops::ArrayLoad {
            ty: vm.class_loader().for_name(types::SHORT.name.into())?,
        }),

        0x36 => b(ops::StoreLocal {
            index: bytes.try_get_u8()? as usize,
            store_next: false,
        }),
        0x37 => b(ops::StoreLocal {
            index: bytes.try_get_u8()? as usize,
            store_next: true,
        }),
        0x38 => b(ops::StoreLocal {
            index: bytes.try_get_u8()? as usize,
            store_next: false,
        }),
        0x39 => b(ops::StoreLocal {
            index: bytes.try_get_u8()? as usize,
            store_next: true,
        }),
        0x3a => b(ops::StoreLocal {
            index: bytes.try_get_u8()? as usize,
            store_next: false,
        }),
        0x3b => b(ops::StoreLocal {
            index: 0,
            store_next: false,
        }),
        0x3c => b(ops::StoreLocal {
            index: 1,
            store_next: false,
        }),
        0x3d => b(ops::StoreLocal {
            index: 2,
            store_next: false,
        }),
        0x3e => b(ops::StoreLocal {
            index: 3,
            store_next: false,
        }),

        0x3f => b(ops::StoreLocal {
            index: 0,
            store_next: true,
        }),
        0x40 => b(ops::StoreLocal {
            index: 1,
            store_next: true,
        }),
        0x41 => b(ops::StoreLocal {
            index: 2,
            store_next: true,
        }),
        0x42 => b(ops::StoreLocal {
            index: 3,
            store_next: true,
        }),

        0x43 => b(ops::StoreLocal {
            index: 0,
            store_next: false,
        }),
        0x44 => b(ops::StoreLocal {
            index: 1,
            store_next: false,
        }),
        0x45 => b(ops::StoreLocal {
            index: 2,
            store_next: false,
        }),
        0x46 => b(ops::StoreLocal {
            index: 3,
            store_next: false,
        }),

        0x47 => b(ops::StoreLocal {
            index: 0,
            store_next: false,
        }),
        0x48 => b(ops::StoreLocal {
            index: 1,
            store_next: false,
        }),
        0x49 => b(ops::StoreLocal {
            index: 2,
            store_next: false,
        }),
        0x4a => b(ops::StoreLocal {
            index: 3,
            store_next: true,
        }),

        0x4b => b(ops::StoreLocal {
            index: 0,
            store_next: false,
        }),
        0x4c => b(ops::StoreLocal {
            index: 1,
            store_next: false,
        }),
        0x4d => b(ops::StoreLocal {
            index: 2,
            store_next: false,
        }),
        0x4e => b(ops::StoreLocal {
            index: 3,
            store_next: false,
        }),

        0x4f => b(ops::ArrayStore {
            ty: vm.class_loader().for_name(types::INT.name.into())?,
        }),
        0x50 => b(ops::ArrayStore {
            ty: vm.class_loader().for_name(types::LONG.name.into())?,
        }),
        0x51 => b(ops::ArrayStore {
            ty: vm.class_loader().for_name(types::FLOAT.name.into())?,
        }),
        0x52 => b(ops::ArrayStore {
            ty: vm.class_loader().for_name(types::DOUBLE.name.into())?,
        }),
        0x53 => b(ops::ArrayStore {
            ty: vm.class_loader().for_name("Ljava/lang/Object;".into())?,
        }),
        0x54 => b(ops::ArrayStore {
            ty: vm.class_loader().for_name(types::BYTE.name.into())?,
        }),
        0x55 => b(ops::ArrayStore {
            ty: vm.class_loader().for_name(types::CHAR.name.into())?,
        }),
        0x56 => b(ops::ArrayStore {
            ty: vm.class_loader().for_name(types::SHORT.name.into())?,
        }),

        // Stack Math Conversions
        0x57 => b(ops::Pop { amount: 1 }),
        0x58 => b(ops::Pop { amount: 2 }),
        0x59 => b(ops::Dup),
        0x5a => b(ops::DupX1),
        0x5b => b(ops::DupX2),
        0x5c => b(ops::Dup2),
        //  0x5d => Opcode::DUP2_X1,
        //  0x5e => Opcode::DUP2_X2,
        //  0x5f => Opcode::SWAP,
        0x60 => b(ops::Iadd),
        0x61 => b(ops::Ladd),
        0x62 => b(ops::Fadd),
        0x63 => b(ops::Dadd),
        0x64 => b(ops::Isub),
        0x65 => b(ops::Lsub),
        0x66 => b(ops::Fsub),
        0x67 => b(ops::Dsub),
        0x68 => b(ops::Imul),
        0x69 => b(ops::Lmul),
        0x6a => b(ops::Fmul),
        0x6b => b(ops::Dmul),
        0x6c => b(ops::Idiv),
        0x6d => b(ops::Ldiv),
        0x6e => b(ops::Fdiv),
        0x6f => b(ops::Ddiv),
        0x70 => b(ops::Irem),
        0x71 => b(ops::Lrem),
        0x72 => b(ops::Frem),
        0x73 => b(ops::Drem),
        0x74 => b(ops::Ineg),
        0x75 => b(ops::Lneg),
        0x76 => b(ops::Fneg),
        0x77 => b(ops::Dneg),
        0x78 => b(ops::Ishl),
        0x79 => b(ops::Lshl),
        0x7a => b(ops::Ishr),
        0x7b => b(ops::Lshr),
        0x7c => b(ops::Iushr),
        0x7d => b(ops::Lushr),
        0x7e => b(ops::Iand),
        0x7f => b(ops::Land),
        0x80 => b(ops::Ior),
        0x81 => b(ops::Lor),
        0x82 => b(ops::Ixor),
        0x83 => b(ops::Lxor),
        0x84 => b(ops::Iinc {
            index: bytes.try_get_u8()?,
            constant: bytes.try_get_i8()?,
        }),
        0x85 => b(ops::I2l),
        0x86 => b(ops::I2f),
        0x87 => b(ops::I2d),
        0x88 => b(ops::L2i),
        0x89 => b(ops::L2f),
        0x8a => b(ops::L2d),
        0x8b => b(ops::F2i),
        //  0x8c => Opcode::F2L,
        0x8d => b(ops::F2d),
        0x8e => b(ops::D2i),
        0x8f => b(ops::D2l),
        0x90 => b(ops::D2f),
        0x91 => b(ops::I2b),
        0x92 => b(ops::I2c),
        // 0x93 => Opcode::I2S,

        // Comparisons
        0x94 => b(ops::Lcmp),
        0x95 => b(ops::Fcmpl),
        0x96 => b(ops::Fcmpg),
        0x97 => b(ops::Dcmpl),
        0x98 => b(ops::Dcmpg),
        0x99 => b(ops::IfEq {
            jump_to: bytes.try_get_i16()?,
        }),
        0x9a => b(ops::IfNe {
            jump_to: bytes.try_get_i16()?,
        }),
        0x9b => b(ops::IfLt {
            jump_to: bytes.try_get_i16()?,
        }),
        0x9c => b(ops::IfGe {
            jump_to: bytes.try_get_i16()?,
        }),
        0x9d => b(ops::IfGt {
            jump_to: bytes.try_get_i16()?,
        }),
        0x9e => b(ops::IfLe {
            jump_to: bytes.try_get_i16()?,
        }),
        0x9f => b(ops::Ieq {
            jump_to: bytes.try_get_i16()?,
        }),
        0xa0 => b(ops::Ine {
            jump_to: bytes.try_get_i16()?,
        }),
        0xa1 => b(ops::Ilt {
            jump_to: bytes.try_get_i16()?,
        }),
        0xa2 => b(ops::Ige {
            jump_to: bytes.try_get_i16()?,
        }),
        0xa3 => b(ops::Igt {
            jump_to: bytes.try_get_i16()?,
        }),
        0xa4 => b(ops::Ile {
            jump_to: bytes.try_get_i16()?,
        }),
        0xa5 => b(ops::IfRefEq {
            jump_to: bytes.try_get_i16()?,
        }),
        0xa6 => b(ops::IfRefNe {
            jump_to: bytes.try_get_i16()?,
        }),

        // Control
        0xa7 => b(ops::Goto {
            jump_to: bytes.try_get_i16()?,
        }),
        //  0xa8 => Opcode::JSR,
        //  0xa9 => Opcode::RET,
        0xaa => {
            let pad_len = 3 - (pc % 4);
            bytes.advance(pad_len as usize);

            let default = bytes.try_get_i32()?;

            let low = bytes.try_get_i32()?;
            let high = bytes.try_get_i32()?;

            let len = (high - low + 1) as usize;
            let mut table: Vec<i32> = Vec::with_capacity(len);
            for _ in 0..len {
                table.push(bytes.try_get_i32()?);
            }

            b(ops::TableSwitch {
                default,
                low,
                high,
                table,
            })
        }
        0xab => {
            let pad_len = 3 - (pc % 4);
            bytes.advance(pad_len as usize);

            let default = bytes.try_get_i32()?;
            let npairs = bytes.try_get_i32()?;

            let mut pairs: Vec<(i32, i32)> = Vec::with_capacity(npairs as usize);
            for _ in 0..npairs {
                pairs.push((bytes.try_get_i32()?, bytes.try_get_i32()?));
            }

            b(ops::LookupSwitch { default, pairs })
        }
        0xac => b(ops::ValueReturn),
        0xad => b(ops::ValueReturn),
        0xae => b(ops::ValueReturn),
        0xaf => b(ops::ValueReturn),
        0xb0 => b(ops::ValueReturn),
        0xb1 => b(ops::VoidReturn),

        // References
        0xb2 => b(ops::GetStatic {
            index: bytes.try_get_u16()?,
        }),
        0xb3 => b(ops::PutStatic {
            index: bytes.try_get_u16()?,
        }),
        0xb4 => b(ops::GetField {
            index: bytes.try_get_u16()?,
        }),
        0xb5 => b(ops::PutField {
            index: bytes.try_get_u16()?,
        }),
        0xb6 => b(ops::InvokeVirtual {
            index: bytes.try_get_u16()?,
        }),
        0xb7 => b(ops::InvokeSpecial {
            index: bytes.try_get_u16()?,
        }),
        0xb8 => b(ops::InvokeStatic {
            index: bytes.try_get_u16()?,
        }),
        0xb9 => b(ops::InvokeInterface {
            index: bytes.try_get_u16()?,
            count: bytes.try_get_u8()?,
            zero: bytes.try_get_u8()?,
        }),
        0xba => b(ops::InvokeDynamic {
            _index: bytes.try_get_u16()?,
            _zeroes: bytes.try_get_u16()?,
        }),
        0xbb => b(ops::New {
            index: bytes.try_get_u16()?,
        }),
        0xbc => b(ops::NewArray {
            type_tag: bytes.try_get_u8()?,
        }),
        0xbd => b(ops::ANewArray {
            type_index: bytes.try_get_u16()?,
        }),
        0xbe => b(ops::ArrayLength),
        0xbf => b(ops::Athrow),
        0xc0 => b(ops::CheckCast {
            type_index: bytes.try_get_u16()?,
        }),
        0xc1 => b(ops::InstanceOf {
            type_index: bytes.try_get_u16()?,
        }),
        0xc2 => b(ops::MonitorEnter),
        0xc3 => b(ops::MonitorExit),

        // Extended
        0xc4 => {
            let opcode = bytes.try_get_u8()?;
            let format = match opcode {
                // iinc form
                0x84 => WideFormat::Format2 {
                    index: bytes.try_get_u16()?,
                    const_val: bytes.try_get_i16()?,
                },
                e => panic!("unknown opcode {} for WIDE", e),
            };

            b(ops::Wide { format })
        }
        0xc5 => b(ops::MultiANewArray {
            type_index: bytes.try_get_u16()?,
            dimensions: bytes.try_get_u8()?,
        }),
        0xc6 => b(ops::IfNull {
            jump_to: bytes.try_get_i16()?,
        }),
        0xc7 => b(ops::IfNotNull {
            jump_to: bytes.try_get_i16()?,
        }),
        //  0xc8 => Opcode::GOTO_W,
        //  0xc9 => Opcode::JSR_W,

        // Reserved
        0xca => b(ops::Nop),
        0xfe => b(ops::Nop),
        0xff => b(ops::Nop),
        e => return Err(internal!("unknown opcode {:#01x}", e)),
    })
}
