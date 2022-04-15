use crate::structs::loaded::constant_pool::ClassData as LoadedClassData;
use crate::structs::JVMPointer;
use std::rc::Rc;

pub type Boolean = bool;
pub type Byte = i8;
pub type Short = i16;
pub type Int = i32;
pub type Long = i64;
pub type Char = char;
pub type Float = f32;
pub type Double = f64;
pub type ReturnAddress = JVMPointer;

#[derive(PartialEq, Clone)]
pub enum PrimitiveWithValue {
    Boolean(Boolean),
    Byte(Byte),
    Short(Short),
    Int(Int),
    Long(Long),
    Char(Char),
    Float(Float),
    Double(Double),
}

#[derive(Clone, PartialEq)]
pub enum PrimitiveType {
    Boolean,
    Byte,
    Short,
    Int,
    Long,
    Char,
    Float,
    Double,
}

pub struct ClassData {
    pub class: Rc<LoadedClassData>,
    pub ptr: JVMPointer,
}

pub enum ReferenceType {
    Class(ClassData),
    Null,
}

pub enum Type {
    Reference(ReferenceType),
    Primitive(PrimitiveWithValue),
}