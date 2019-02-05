// Copyright (c) 2017 Fabian Schuiki

//! Types of values.

pub use self::TypeKind::*;
use std;
use std::sync::Arc;
use crate::util::write_implode;

pub type Type = Arc<TypeKind>;

/// The different kinds of types.
#[derive(Debug, PartialEq, Eq)]
pub enum TypeKind {
    /// The `void` type.
    VoidType,
    /// The `time` type.
    TimeType,
    /// Integer types like `i32`.
    IntType(usize),
    /// Enumerated types like `n42`.
    EnumType(usize),
    /// Pointer types like `i32*`.
    PointerType(Type),
    /// Signal types like `i32$`.
    SignalType(Type),
    /// Array types like `[4 x i32]`.
    ArrayType(usize, Type),
    /// Struct types like `{i8, i32}`.
    StructType(Vec<Type>),
    /// Function types like `(i32) void`.
    FuncType(Vec<Type>, Type),
    /// Entity types like `(i8, i8; i32)`.
    EntityType(Vec<Type>, Vec<Type>),
}

impl std::fmt::Display for TypeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            VoidType => write!(f, "void"),
            TimeType => write!(f, "time"),
            IntType(l) => write!(f, "i{}", l),
            EnumType(l) => write!(f, "n{}", l),
            PointerType(ref ty) => write!(f, "{}*", ty),
            SignalType(ref ty) => write!(f, "{}$", ty),
            ArrayType(l, ref ty) => write!(f, "[{} x {}]", l, ty),
            StructType(ref tys) => {
                write!(f, "{{")?;
                write_implode(f, ", ", tys.iter())?;
                write!(f, "}}")?;
                Ok(())
            }
            FuncType(ref args, ref ret) => {
                write!(f, "(")?;
                write_implode(f, ", ", args.iter())?;
                write!(f, ") {}", ret)?;
                Ok(())
            }
            EntityType(ref ins, ref outs) => {
                write!(f, "(")?;
                write_implode(f, ", ", ins.iter())?;
                write!(f, ";")?;
                write_implode(f, ", ", outs.iter())?;
                write!(f, ")")?;
                Ok(())
            }
        }
    }
}

impl TypeKind {
    /// Unwrap the type into arguments and return type, or panic if the type is
    /// not a function.
    pub fn as_func(&self) -> (&[Type], &Type) {
        match *self {
            FuncType(ref args, ref ret) => (args, ret),
            _ => panic!("as_func called on {}", self),
        }
    }

    /// Unwrap the type into input and output arguments, or panic if the type is
    /// not an entity.
    pub fn as_entity(&self) -> (&[Type], &[Type]) {
        match *self {
            EntityType(ref ins, ref outs) => (ins, outs),
            _ => panic!("as_entity called on {}", self),
        }
    }

    /// Unwrap the type to its integer bit width, or panic if the type is not an
    /// integer.
    pub fn as_int(&self) -> usize {
        match *self {
            IntType(size) => size,
            _ => panic!("as_int called on {}", self),
        }
    }

    /// Unwrap the type to its number of enumerated states, or panic if the type
    /// is not an enum.
    pub fn as_enum(&self) -> usize {
        match *self {
            EnumType(size) => size,
            _ => panic!("as_enum called on {}", self),
        }
    }

    /// Unwrap the type to its signal data type, or panic if the type is not an
    /// integer. E.g. yields the `i8` type in `i8$`.
    pub fn as_signal(&self) -> &Type {
        match *self {
            SignalType(ref ty) => ty,
            _ => panic!("as_signal called on {}", self),
        }
    }

    /// Check if this type is a void type.
    pub fn is_void(&self) -> bool {
        match *self {
            VoidType => true,
            _ => false,
        }
    }
}

/// Create a void type.
pub fn void_ty() -> Type {
    Type::new(VoidType)
}

/// Create a time type.
pub fn time_ty() -> Type {
    Type::new(TimeType)
}

/// Create an integer type of the requested size.
pub fn int_ty(size: usize) -> Type {
    Type::new(IntType(size))
}

/// Create an enum type of the requested size.
pub fn enum_ty(size: usize) -> Type {
    Type::new(EnumType(size))
}

/// Create a pointer type with the requested data type.
pub fn pointer_ty(ty: Type) -> Type {
    Type::new(PointerType(ty))
}

/// Create a signal type with the requested data type.
pub fn signal_ty(ty: Type) -> Type {
    Type::new(SignalType(ty))
}

/// Create a array type. `size` is the number of elements in the array, and `ty`
/// the type of each individual element.
pub fn array_ty(size: usize, ty: Type) -> Type {
    Type::new(ArrayType(size, ty))
}

/// Create a struct type. `fields` is an list of types, one for each field.
pub fn struct_ty(fields: Vec<Type>) -> Type {
    Type::new(StructType(fields))
}

/// Create a function type with the given arguments and return type.
pub fn func_ty(args: Vec<Type>, ret: Type) -> Type {
    Type::new(FuncType(args, ret))
}

/// Create an entity type with the given input and output arguments.
pub fn entity_ty(ins: Vec<Type>, outs: Vec<Type>) -> Type {
    Type::new(EntityType(ins, outs))
}
