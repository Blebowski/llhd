// Copyright (c) 2017-2019 Fabian Schuiki

//! Representation of LLHD instructions.
//!
//! This module implements the various instructions of the intermediate
//! representation.

use crate::{
    ir::{Block, DataFlowGraph, ExtUnit, Inst, Unit, UnitBuilder, Value},
    ty::{array_ty, int_ty, pointer_ty, signal_ty, struct_ty, time_ty, void_ty, Type},
    ConstTime,
};
use bitflags::bitflags;
use num::BigInt;

/// A temporary object used to construct a single instruction.
pub struct InstBuilder<B> {
    builder: B,
}

impl<B> InstBuilder<B> {
    /// Create a new instruction builder that inserts into `builder`.
    pub fn new(builder: B) -> Self {
        Self { builder }
    }
}

impl<B: UnitBuilder> InstBuilder<&mut B> {
    /// `a = const iN imm`
    pub fn const_int(&mut self, width: usize, value: impl Into<BigInt>) -> Value {
        let data = InstData::ConstInt {
            opcode: Opcode::ConstInt,
            imm: value.into(),
        };
        let inst = self.build(data, int_ty(width));
        self.inst_result(inst)
    }

    /// `a = const time imm`
    pub fn const_time(&mut self, value: impl Into<ConstTime>) -> Value {
        let data = InstData::ConstTime {
            opcode: Opcode::ConstTime,
            imm: value.into(),
        };
        let inst = self.build(data, time_ty());
        self.inst_result(inst)
    }

    /// `a = x`
    pub fn alias(&mut self, x: Value) -> Value {
        let ty = self.value_type(x);
        let inst = self.build_unary(Opcode::Alias, ty, x);
        self.inst_result(inst)
    }

    /// `a = array imm, type x`
    pub fn array_uniform(&mut self, imm: usize, x: Value) -> Value {
        let ty = array_ty(imm, self.value_type(x));
        let inst = self.build(
            InstData::Array {
                opcode: Opcode::ArrayUniform,
                imm: imm,
                args: [x],
            },
            ty,
        );
        self.inst_result(inst)
    }

    /// `a = array args`
    pub fn array(&mut self, args: Vec<Value>) -> Value {
        assert!(!args.is_empty());
        let ty = array_ty(args.len(), self.value_type(args[0]));
        let inst = self.build(
            InstData::Aggregate {
                opcode: Opcode::Array,
                args: args,
            },
            ty,
        );
        self.inst_result(inst)
    }

    /// `a = struct args`
    pub fn strukt(&mut self, args: Vec<Value>) -> Value {
        let ty = struct_ty(
            args.iter()
                .cloned()
                .map(|arg| self.value_type(arg))
                .collect(),
        );
        let inst = self.build(
            InstData::Aggregate {
                opcode: Opcode::Struct,
                args: args,
            },
            ty,
        );
        self.inst_result(inst)
    }

    /// `a = not type x, y`
    pub fn not(&mut self, x: Value) -> Value {
        let ty = self.value_type(x);
        let inst = self.build_unary(Opcode::Not, ty, x);
        self.inst_result(inst)
    }

    /// `a = neg type x, y`
    pub fn neg(&mut self, x: Value) -> Value {
        let ty = self.value_type(x);
        let inst = self.build_unary(Opcode::Neg, ty, x);
        self.inst_result(inst)
    }

    /// `a = add type x, y`
    pub fn add(&mut self, x: Value, y: Value) -> Value {
        let ty = self.value_type(x);
        let inst = self.build_binary(Opcode::Add, ty, x, y);
        self.inst_result(inst)
    }

    /// `a = sub type x, y`
    pub fn sub(&mut self, x: Value, y: Value) -> Value {
        let ty = self.value_type(x);
        let inst = self.build_binary(Opcode::Sub, ty, x, y);
        self.inst_result(inst)
    }

    /// `a = and type x, y`
    pub fn and(&mut self, x: Value, y: Value) -> Value {
        let ty = self.value_type(x);
        let inst = self.build_binary(Opcode::And, ty, x, y);
        self.inst_result(inst)
    }

    /// `a = or type x, y`
    pub fn or(&mut self, x: Value, y: Value) -> Value {
        let ty = self.value_type(x);
        let inst = self.build_binary(Opcode::Or, ty, x, y);
        self.inst_result(inst)
    }

    /// `a = xor type x, y`
    pub fn xor(&mut self, x: Value, y: Value) -> Value {
        let ty = self.value_type(x);
        let inst = self.build_binary(Opcode::Xor, ty, x, y);
        self.inst_result(inst)
    }

    /// `a = smul type x, y`
    pub fn smul(&mut self, x: Value, y: Value) -> Value {
        let ty = self.value_type(x);
        let inst = self.build_binary(Opcode::Smul, ty, x, y);
        self.inst_result(inst)
    }

    /// `a = sdiv type x, y`
    pub fn sdiv(&mut self, x: Value, y: Value) -> Value {
        let ty = self.value_type(x);
        let inst = self.build_binary(Opcode::Sdiv, ty, x, y);
        self.inst_result(inst)
    }

    /// `a = smod type x, y`
    pub fn smod(&mut self, x: Value, y: Value) -> Value {
        let ty = self.value_type(x);
        let inst = self.build_binary(Opcode::Smod, ty, x, y);
        self.inst_result(inst)
    }

    /// `a = srem type x, y`
    pub fn srem(&mut self, x: Value, y: Value) -> Value {
        let ty = self.value_type(x);
        let inst = self.build_binary(Opcode::Srem, ty, x, y);
        self.inst_result(inst)
    }

    /// `a = umul type x, y`
    pub fn umul(&mut self, x: Value, y: Value) -> Value {
        let ty = self.value_type(x);
        let inst = self.build_binary(Opcode::Umul, ty, x, y);
        self.inst_result(inst)
    }

    /// `a = udiv type x, y`
    pub fn udiv(&mut self, x: Value, y: Value) -> Value {
        let ty = self.value_type(x);
        let inst = self.build_binary(Opcode::Udiv, ty, x, y);
        self.inst_result(inst)
    }

    /// `a = umod type x, y`
    pub fn umod(&mut self, x: Value, y: Value) -> Value {
        let ty = self.value_type(x);
        let inst = self.build_binary(Opcode::Umod, ty, x, y);
        self.inst_result(inst)
    }

    /// `a = urem type x, y`
    pub fn urem(&mut self, x: Value, y: Value) -> Value {
        let ty = self.value_type(x);
        let inst = self.build_binary(Opcode::Urem, ty, x, y);
        self.inst_result(inst)
    }

    /// `a = eq type x, y`
    pub fn eq(&mut self, x: Value, y: Value) -> Value {
        let inst = self.build_binary(Opcode::Eq, int_ty(1), x, y);
        self.inst_result(inst)
    }

    /// `a = neq type x, y`
    pub fn neq(&mut self, x: Value, y: Value) -> Value {
        let inst = self.build_binary(Opcode::Neq, int_ty(1), x, y);
        self.inst_result(inst)
    }

    /// `a = slt type x, y`
    pub fn slt(&mut self, x: Value, y: Value) -> Value {
        let inst = self.build_binary(Opcode::Slt, int_ty(1), x, y);
        self.inst_result(inst)
    }

    /// `a = sgt type x, y`
    pub fn sgt(&mut self, x: Value, y: Value) -> Value {
        let inst = self.build_binary(Opcode::Sgt, int_ty(1), x, y);
        self.inst_result(inst)
    }

    /// `a = sle type x, y`
    pub fn sle(&mut self, x: Value, y: Value) -> Value {
        let inst = self.build_binary(Opcode::Sle, int_ty(1), x, y);
        self.inst_result(inst)
    }

    /// `a = sge type x, y`
    pub fn sge(&mut self, x: Value, y: Value) -> Value {
        let inst = self.build_binary(Opcode::Sge, int_ty(1), x, y);
        self.inst_result(inst)
    }

    /// `a = ult type x, y`
    pub fn ult(&mut self, x: Value, y: Value) -> Value {
        let inst = self.build_binary(Opcode::Ult, int_ty(1), x, y);
        self.inst_result(inst)
    }

    /// `a = ugt type x, y`
    pub fn ugt(&mut self, x: Value, y: Value) -> Value {
        let inst = self.build_binary(Opcode::Ugt, int_ty(1), x, y);
        self.inst_result(inst)
    }

    /// `a = ule type x, y`
    pub fn ule(&mut self, x: Value, y: Value) -> Value {
        let inst = self.build_binary(Opcode::Ule, int_ty(1), x, y);
        self.inst_result(inst)
    }

    /// `a = uge type x, y`
    pub fn uge(&mut self, x: Value, y: Value) -> Value {
        let inst = self.build_binary(Opcode::Uge, int_ty(1), x, y);
        self.inst_result(inst)
    }

    /// `a = shl type x, y, z`
    pub fn shl(&mut self, x: Value, y: Value, z: Value) -> Value {
        let ty = self.value_type(x);
        let inst = self.build_ternary(Opcode::Shl, ty, x, y, z);
        self.inst_result(inst)
    }

    /// `a = shr type x, y, z`
    pub fn shr(&mut self, x: Value, y: Value, z: Value) -> Value {
        let ty = self.value_type(x);
        let inst = self.build_ternary(Opcode::Shr, ty, x, y, z);
        self.inst_result(inst)
    }

    /// `a = mux type x, y`
    pub fn mux(&mut self, x: Value, y: Value) -> Value {
        let ty = self.value_type(x);
        assert!(ty.is_array(), "argument to `mux` must be of array type");
        let ty = ty.unwrap_array().1.clone();
        let inst = self.build_binary(Opcode::Mux, ty, x, y);
        self.inst_result(inst)
    }

    /// `a = reg type init (, data mode trigger)*`
    pub fn reg(&mut self, x: Value, data: Vec<(Value, RegMode, Value)>) -> Value {
        let ty = self.value_type(x);
        let mut args = vec![x];
        let mut modes = vec![];
        for (data, mode, trigger) in data {
            args.push(data);
            args.push(trigger);
            modes.push(mode);
        }
        assert_eq!(args.len(), modes.len() * 2 + 1);
        let inst = self.build(
            InstData::Reg {
                opcode: Opcode::Reg,
                args,
                modes,
            },
            ty,
        );
        self.inst_result(inst)
    }

    /// `a = insf type x, y, imm`
    pub fn ins_field(&mut self, x: Value, y: Value, imm: usize) -> Value {
        let ty = self.value_type(x);
        let inst = self.build(
            InstData::InsExt {
                opcode: Opcode::InsField,
                args: [x, y],
                imms: [imm, 0],
            },
            ty,
        );
        self.inst_result(inst)
    }

    /// `a = inss type x, y, imm0, imm1`
    pub fn ins_slice(&mut self, x: Value, y: Value, imm0: usize, imm1: usize) -> Value {
        let ty = self.value_type(x);
        let inst = self.build(
            InstData::InsExt {
                opcode: Opcode::InsSlice,
                args: [x, y],
                imms: [imm0, imm1],
            },
            ty,
        );
        self.inst_result(inst)
    }

    /// `a = extf type x, imm`
    pub fn ext_field(&mut self, x: Value, imm: usize) -> Value {
        let ty = self.value_type(x);
        let (ty, is_ptr, is_sig) = if ty.is_pointer() {
            (ty.unwrap_pointer(), true, false)
        } else if ty.is_signal() {
            (ty.unwrap_signal(), false, true)
        } else {
            (&ty, false, false)
        };
        let ty = if ty.is_struct() {
            let fields = ty.unwrap_struct();
            assert!(imm < fields.len(), "field index in `extf` out of range");
            fields[imm].clone()
        } else if ty.is_array() {
            ty.unwrap_array().1.clone()
        } else {
            panic!("argument to `extf` must be of struct or array type");
        };
        let ty = if is_ptr {
            pointer_ty(ty)
        } else if is_sig {
            signal_ty(ty)
        } else {
            ty
        };
        let inst = self.build(
            InstData::InsExt {
                opcode: Opcode::ExtField,
                args: [x, Value::invalid()],
                imms: [imm, 0],
            },
            ty,
        );
        self.inst_result(inst)
    }

    /// `a = exts type x, imm0, imm1`
    pub fn ext_slice(&mut self, x: Value, imm0: usize, imm1: usize) -> Value {
        let ty = self.value_type(x);
        let ty = if ty.is_array() {
            array_ty(imm1, ty.unwrap_array().1.clone())
        } else if ty.is_int() {
            int_ty(imm1)
        } else {
            panic!("argument to `exts` must be of array or integer type");
        };
        let inst = self.build(
            InstData::InsExt {
                opcode: Opcode::ExtSlice,
                args: [x, Value::invalid()],
                imms: [imm0, imm1],
            },
            ty,
        );
        self.inst_result(inst)
    }

    /// `con type x, y`
    pub fn con(&mut self, x: Value, y: Value) -> Inst {
        self.build_binary(Opcode::Con, void_ty(), x, y)
    }

    /// `a = del type x, y`
    pub fn del(&mut self, x: Value, y: Value) -> Inst {
        let ty = self.value_type(x);
        self.build_binary(Opcode::Del, ty, x, y)
    }

    /// `a = call type unit (args...)`
    pub fn call(&mut self, unit: ExtUnit, args: Vec<Value>) -> Value {
        let ty = self.builder.unit().extern_sig(unit).return_type();
        let data = InstData::Call {
            opcode: Opcode::Call,
            unit,
            ins: args.len() as u16,
            args,
        };
        let inst = self.build(data, ty);
        self.inst_result(inst)
    }

    /// `inst unit (inputs...) -> (outputs...)`
    pub fn inst(&mut self, unit: ExtUnit, mut inputs: Vec<Value>, outputs: Vec<Value>) -> Inst {
        let ins = inputs.len() as u16;
        inputs.extend(outputs);
        let data = InstData::Call {
            opcode: Opcode::Inst,
            unit,
            ins,
            args: inputs,
        };
        self.build(data, void_ty())
    }

    /// `a = sig type x`
    pub fn sig(&mut self, x: Value) -> Value {
        let ty = signal_ty(self.value_type(x));
        let inst = self.build_unary(Opcode::Sig, ty, x);
        self.inst_result(inst)
    }

    /// `a = prb type x`
    pub fn prb(&mut self, x: Value) -> Value {
        let ty = self.value_type(x);
        assert!(ty.is_signal(), "argument to `prb` must be of signal type");
        let ty = ty.unwrap_signal().clone();
        let inst = self.build_unary(Opcode::Prb, ty, x);
        self.inst_result(inst)
    }

    /// `drv type x, y, z`
    pub fn drv(&mut self, x: Value, y: Value, z: Value) -> Inst {
        self.build_ternary(Opcode::Drv, void_ty(), x, y, z)
    }

    /// `a = var type x`
    pub fn var(&mut self, x: Value) -> Value {
        let ty = pointer_ty(self.value_type(x));
        let inst = self.build_unary(Opcode::Var, ty, x);
        self.inst_result(inst)
    }

    /// `a = ld type x`
    pub fn ld(&mut self, x: Value) -> Value {
        let ty = self.value_type(x);
        assert!(ty.is_pointer(), "argument to `ld` must be of pointer type");
        let ty = ty.unwrap_pointer().clone();
        let inst = self.build_unary(Opcode::Ld, ty, x);
        self.inst_result(inst)
    }

    /// `st type x, y`
    pub fn st(&mut self, x: Value, y: Value) -> Inst {
        self.build_binary(Opcode::St, void_ty(), x, y)
    }

    /// `halt`
    pub fn halt(&mut self) -> Inst {
        self.build_nullary(Opcode::Halt)
    }

    /// `ret`
    pub fn ret(&mut self) -> Inst {
        self.build_nullary(Opcode::Ret)
    }

    /// `ret type x`
    pub fn ret_value(&mut self, x: Value) -> Inst {
        self.build_unary(Opcode::RetValue, void_ty(), x)
    }

    /// `br bb`
    pub fn br(&mut self, bb: Block) -> Inst {
        let data = InstData::Jump {
            opcode: Opcode::Br,
            bbs: [bb],
        };
        self.build(data, void_ty())
    }

    /// `br x, bb0, bb1`
    pub fn br_cond(&mut self, x: Value, bb0: Block, bb1: Block) -> Inst {
        let data = InstData::Branch {
            opcode: Opcode::BrCond,
            args: [x],
            bbs: [bb0, bb1],
        };
        self.build(data, void_ty())
    }

    /// `wait bb, args`
    pub fn wait(&mut self, bb: Block, args: Vec<Value>) -> Inst {
        let data = InstData::Wait {
            opcode: Opcode::Wait,
            bbs: [bb],
            args: args,
        };
        self.build(data, void_ty())
    }

    /// `wait bb, time, args`
    pub fn wait_time(&mut self, bb: Block, time: Value, mut args: Vec<Value>) -> Inst {
        args.insert(0, time);
        let data = InstData::Wait {
            opcode: Opcode::WaitTime,
            bbs: [bb],
            args: args,
        };
        self.build(data, void_ty())
    }
}

/// Convenience functions to construct the different instruction formats.
impl<B: UnitBuilder> InstBuilder<&mut B> {
    /// `opcode`
    fn build_nullary(&mut self, opcode: Opcode) -> Inst {
        let data = InstData::Nullary { opcode };
        self.build(data, void_ty())
    }

    /// `a = opcode type x`
    fn build_unary(&mut self, opcode: Opcode, ty: Type, x: Value) -> Inst {
        let data = InstData::Unary { opcode, args: [x] };
        self.build(data, ty)
    }

    /// `a = opcode type x, y`
    fn build_binary(&mut self, opcode: Opcode, ty: Type, x: Value, y: Value) -> Inst {
        let data = InstData::Binary {
            opcode,
            args: [x, y],
        };
        self.build(data, ty)
    }

    /// `a = opcode type x, y, z`
    fn build_ternary(&mut self, opcode: Opcode, ty: Type, x: Value, y: Value, z: Value) -> Inst {
        let data = InstData::Ternary {
            opcode,
            args: [x, y, z],
        };
        self.build(data, ty)
    }
}

/// Fundamental convenience forwards to the wrapped builder.
impl<B: UnitBuilder> InstBuilder<&mut B> {
    /// Convenience forward to `UnitBuilder`.
    fn build(&mut self, data: InstData, ty: Type) -> Inst {
        self.builder.build_inst(data, ty)
    }

    /// Convenience forward to `Unit`.
    fn value_type(&self, value: Value) -> Type {
        self.builder.unit().value_type(value)
    }

    /// Convenience forward to `Unit`.
    fn inst_result(&self, inst: Inst) -> Value {
        self.builder.unit().inst_result(inst)
    }
}

/// An instruction format.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub enum InstData {
    /// `a = const iN imm`
    ConstInt { opcode: Opcode, imm: BigInt },
    /// `a = const time imm`
    ConstTime { opcode: Opcode, imm: ConstTime },
    /// `opcode imm, type x`
    Array {
        opcode: Opcode,
        imm: usize,
        args: [Value; 1],
    },
    /// `opcode args`
    Aggregate { opcode: Opcode, args: Vec<Value> },
    /// `opcode`
    Nullary { opcode: Opcode },
    /// `opcode type x`
    Unary { opcode: Opcode, args: [Value; 1] },
    /// `opcode type x, y`
    Binary { opcode: Opcode, args: [Value; 2] },
    /// `opcode type x, y, z`
    Ternary { opcode: Opcode, args: [Value; 3] },
    /// `opcode bb`
    Jump { opcode: Opcode, bbs: [Block; 1] },
    /// `opcode x, bb0, bb1`
    Branch {
        opcode: Opcode,
        args: [Value; 1],
        bbs: [Block; 2],
    },
    /// `opcode bb, args`
    Wait {
        opcode: Opcode,
        bbs: [Block; 1],
        args: Vec<Value>,
    },
    /// `a = opcode type unit (inputs) -> (outputs)`
    Call {
        opcode: Opcode,
        unit: ExtUnit,
        ins: u16,
        args: Vec<Value>,
    },
    /// `a = opcode type x, y, imm0, imm1`
    InsExt {
        opcode: Opcode,
        args: [Value; 2],
        imms: [usize; 2],
    },
    /// `a = reg type x (, data mode trigger)*`
    Reg {
        opcode: Opcode,
        args: Vec<Value>,
        modes: Vec<RegMode>,
    },
}

impl InstData {
    /// Get the opcode of the instruction.
    pub fn opcode(&self) -> Opcode {
        match *self {
            InstData::ConstInt { opcode, .. } => opcode,
            InstData::ConstTime { opcode, .. } => opcode,
            InstData::Array { opcode, .. } => opcode,
            InstData::Aggregate { opcode, .. } => opcode,
            InstData::Nullary { opcode, .. } => opcode,
            InstData::Unary { opcode, .. } => opcode,
            InstData::Binary { opcode, .. } => opcode,
            InstData::Ternary { opcode, .. } => opcode,
            InstData::Jump { opcode, .. } => opcode,
            InstData::Branch { opcode, .. } => opcode,
            InstData::Wait { opcode, .. } => opcode,
            InstData::Call { opcode, .. } => opcode,
            InstData::InsExt { opcode, .. } => opcode,
            InstData::Reg { opcode, .. } => opcode,
        }
    }

    /// Get the arguments of an instruction.
    pub fn args(&self) -> &[Value] {
        match self {
            InstData::ConstInt { .. } => &[],
            InstData::ConstTime { .. } => &[],
            InstData::Array { args, .. } => args,
            InstData::Aggregate { args, .. } => args,
            InstData::Nullary { .. } => &[],
            InstData::Unary { args, .. } => args,
            InstData::Binary { args, .. } => args,
            InstData::Ternary { args, .. } => args,
            InstData::Jump { .. } => &[],
            InstData::Branch { args, .. } => args,
            InstData::Wait { args, .. } => args,
            InstData::Call { args, .. } => args,
            InstData::InsExt {
                opcode: Opcode::ExtField,
                args,
                ..
            }
            | InstData::InsExt {
                opcode: Opcode::ExtSlice,
                args,
                ..
            } => &args[0..1],
            InstData::InsExt { args, .. } => args,
            InstData::Reg { args, .. } => args,
        }
    }

    /// Mutable access to the arguments of an instruction.
    pub fn args_mut(&mut self) -> &mut [Value] {
        match self {
            InstData::ConstInt { .. } => &mut [],
            InstData::ConstTime { .. } => &mut [],
            InstData::Array { args, .. } => args,
            InstData::Aggregate { args, .. } => args,
            InstData::Nullary { .. } => &mut [],
            InstData::Unary { args, .. } => args,
            InstData::Binary { args, .. } => args,
            InstData::Ternary { args, .. } => args,
            InstData::Jump { .. } => &mut [],
            InstData::Branch { args, .. } => args,
            InstData::Wait { args, .. } => args,
            InstData::Call { args, .. } => args,
            InstData::InsExt {
                opcode: Opcode::ExtField,
                args,
                ..
            }
            | InstData::InsExt {
                opcode: Opcode::ExtSlice,
                args,
                ..
            } => &mut args[0..1],
            InstData::InsExt { args, .. } => args,
            InstData::Reg { args, .. } => args,
        }
    }

    /// Get the input arguments of a call instruction.
    pub fn input_args(&self) -> &[Value] {
        match *self {
            InstData::Call { ref args, ins, .. } => &args[0..ins as usize],
            _ => &[],
        }
    }

    /// Get the output arguments of a call instruction.
    pub fn output_args(&self) -> &[Value] {
        match *self {
            InstData::Call { ref args, ins, .. } => &args[ins as usize..],
            _ => &[],
        }
    }

    /// Get the data arguments of a register instruction.
    pub fn data_args(&self) -> &[Value] {
        match self {
            InstData::Reg { args, modes, .. } => &args[1..1 + modes.len()],
            _ => &[],
        }
    }

    /// Get the trigger arguments of a register instruction.
    pub fn trigger_args(&self) -> &[Value] {
        match self {
            InstData::Reg { args, modes, .. } => &args[1 + modes.len()..],
            _ => &[],
        }
    }

    /// Get the modes of a register instruction.
    pub fn mode_args(&self) -> &[RegMode] {
        match self {
            InstData::Reg { modes, .. } => modes,
            _ => &[],
        }
    }

    /// Get the BBs of an instruction.
    pub fn blocks(&self) -> &[Block] {
        match self {
            InstData::ConstInt { .. } => &[],
            InstData::ConstTime { .. } => &[],
            InstData::Array { .. } => &[],
            InstData::Aggregate { .. } => &[],
            InstData::Nullary { .. } => &[],
            InstData::Unary { .. } => &[],
            InstData::Binary { .. } => &[],
            InstData::Ternary { .. } => &[],
            InstData::Jump { bbs, .. } => bbs,
            InstData::Branch { bbs, .. } => bbs,
            InstData::Wait { bbs, .. } => bbs,
            InstData::Call { .. } => &[],
            InstData::InsExt { .. } => &[],
            InstData::Reg { .. } => &[],
        }
    }

    /// Mutable access to the BBs of an instruction.
    pub fn blocks_mut(&mut self) -> &mut [Block] {
        match self {
            InstData::ConstInt { .. } => &mut [],
            InstData::ConstTime { .. } => &mut [],
            InstData::Array { .. } => &mut [],
            InstData::Aggregate { .. } => &mut [],
            InstData::Nullary { .. } => &mut [],
            InstData::Unary { .. } => &mut [],
            InstData::Binary { .. } => &mut [],
            InstData::Ternary { .. } => &mut [],
            InstData::Jump { bbs, .. } => bbs,
            InstData::Branch { bbs, .. } => bbs,
            InstData::Wait { bbs, .. } => bbs,
            InstData::Call { .. } => &mut [],
            InstData::InsExt { .. } => &mut [],
            InstData::Reg { .. } => &mut [],
        }
    }

    /// Replace all uses of a value with another.
    pub fn replace_value(&mut self, from: Value, to: Value) -> usize {
        let mut count = 0;
        for arg in self.args_mut() {
            if *arg == from {
                *arg = to;
                count += 1;
            }
        }
        count
    }

    /// Replace all uses of a block with another.
    pub fn replace_block(&mut self, from: Block, to: Block) -> usize {
        let mut count = 0;
        for bb in self.blocks_mut() {
            if *bb == from {
                *bb = to;
                count += 1;
            }
        }
        count
    }

    /// Return the const int constructed by this instruction.
    pub fn get_const_int(&self) -> Option<&BigInt> {
        match self {
            InstData::ConstInt { imm, .. } => Some(imm),
            _ => None,
        }
    }

    /// Return the const time constructed by this instruction.
    pub fn get_const_time(&self) -> Option<&ConstTime> {
        match self {
            InstData::ConstTime { imm, .. } => Some(imm),
            _ => None,
        }
    }

    /// Return the external unit being called or instantiated by this
    /// instruction.
    pub fn get_ext_unit(&self) -> Option<ExtUnit> {
        match self {
            InstData::Call { unit, .. } => Some(*unit),
            _ => None,
        }
    }
}

bitflags! {
    /// A set of flags identifying a unit.
    #[derive(Default)]
    pub struct UnitFlags: u8 {
        const FUNCTION = 0b001;
        const PROCESS = 0b010;
        const ENTITY = 0b100;
        const ALL = 0b111;
    }
}

/// The trigger modes for register data acquisition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegMode {
    /// The register is transparent if the trigger is low.
    Low,
    /// The register is transparent if the trigger is high.
    High,
    /// The register stores data on the rising edge of the trigger.
    Rise,
    /// The register stores data on the falling edge of the trigger.
    Fall,
    /// The register stores data on any edge of the trigger.
    Both,
}

impl std::fmt::Display for RegMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RegMode::Low => write!(f, "low"),
            RegMode::High => write!(f, "high"),
            RegMode::Rise => write!(f, "rise"),
            RegMode::Fall => write!(f, "fall"),
            RegMode::Both => write!(f, "both"),
        }
    }
}

/// An instruction opcode.
///
/// This enum represents the actual instruction, whereas `InstData` covers the
/// format and arguments of the instruction.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Opcode {
    ConstInt,
    ConstTime,
    Alias,
    ArrayUniform,
    Array,
    Struct,

    Not,
    Neg,

    Add,
    Sub,
    And,
    Or,
    Xor,
    Smul,
    Sdiv,
    Smod,
    Srem,
    Umul,
    Udiv,
    Umod,
    Urem,

    Eq,
    Neq,
    Slt,
    Sgt,
    Sle,
    Sge,
    Ult,
    Ugt,
    Ule,
    Uge,

    Shl,
    Shr,
    Mux,
    Reg,
    InsField,
    InsSlice,
    ExtField,
    ExtSlice,
    Con,
    Del,

    Call,
    Inst,

    Sig,
    Drv,
    Prb,

    Var,
    Ld,
    St,

    Halt,
    Ret,
    RetValue,
    Br,
    BrCond,
    Wait,
    WaitTime,
}

impl std::fmt::Display for Opcode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Opcode::ConstInt => "const",
                Opcode::ConstTime => "const",
                Opcode::Alias => "alias",
                Opcode::ArrayUniform => "array",
                Opcode::Array => "array",
                Opcode::Struct => "struct",
                Opcode::Not => "not",
                Opcode::Neg => "neg",
                Opcode::Add => "add",
                Opcode::Sub => "sub",
                Opcode::And => "and",
                Opcode::Or => "or",
                Opcode::Xor => "xor",
                Opcode::Smul => "smul",
                Opcode::Sdiv => "sdiv",
                Opcode::Smod => "smod",
                Opcode::Srem => "srem",
                Opcode::Umul => "umul",
                Opcode::Udiv => "udiv",
                Opcode::Umod => "umod",
                Opcode::Urem => "urem",
                Opcode::Eq => "eq",
                Opcode::Neq => "neq",
                Opcode::Slt => "slt",
                Opcode::Sgt => "sgt",
                Opcode::Sle => "sle",
                Opcode::Sge => "sge",
                Opcode::Ult => "ult",
                Opcode::Ugt => "ugt",
                Opcode::Ule => "ule",
                Opcode::Uge => "uge",
                Opcode::Shl => "shl",
                Opcode::Shr => "shr",
                Opcode::Mux => "mux",
                Opcode::Reg => "reg",
                Opcode::InsField => "insf",
                Opcode::InsSlice => "inss",
                Opcode::ExtField => "extf",
                Opcode::ExtSlice => "exts",
                Opcode::Con => "con",
                Opcode::Del => "del",
                Opcode::Call => "call",
                Opcode::Inst => "inst",
                Opcode::Sig => "sig",
                Opcode::Drv => "drv",
                Opcode::Prb => "prb",
                Opcode::Var => "var",
                Opcode::Ld => "ld",
                Opcode::St => "st",
                Opcode::Halt => "halt",
                Opcode::Ret => "ret",
                Opcode::RetValue => "ret",
                Opcode::Br => "br",
                Opcode::BrCond => "br",
                Opcode::Wait => "wait",
                Opcode::WaitTime => "wait",
            }
        )
    }
}

impl Opcode {
    /// Return a set of flags where this instruction is valid.
    pub fn valid_in(self) -> UnitFlags {
        match self {
            Opcode::Halt => UnitFlags::PROCESS,
            Opcode::Ret | Opcode::RetValue => UnitFlags::FUNCTION,
            Opcode::Br | Opcode::BrCond => UnitFlags::FUNCTION | UnitFlags::PROCESS,
            Opcode::Con => UnitFlags::ENTITY,
            Opcode::Del => UnitFlags::ENTITY,
            Opcode::Reg => UnitFlags::ENTITY,
            _ => UnitFlags::ALL,
        }
    }

    /// Check if this instruction can appear in a `Function`.
    pub fn valid_in_function(self) -> bool {
        self.valid_in().contains(UnitFlags::FUNCTION)
    }

    /// Check if this instruction can appear in a `Process`.
    pub fn valid_in_process(self) -> bool {
        self.valid_in().contains(UnitFlags::PROCESS)
    }

    /// Check if this instruction can appear in a `Entity`.
    pub fn valid_in_entity(self) -> bool {
        self.valid_in().contains(UnitFlags::ENTITY)
    }

    /// Check if this instruction is a constant.
    pub fn is_const(self) -> bool {
        match self {
            Opcode::ConstInt => true,
            Opcode::ConstTime => true,
            _ => false,
        }
    }

    /// Check if this instruction is a terminator.
    pub fn is_terminator(self) -> bool {
        match self {
            Opcode::Halt
            | Opcode::Ret
            | Opcode::RetValue
            | Opcode::Br
            | Opcode::BrCond
            | Opcode::Wait
            | Opcode::WaitTime => true,
            _ => false,
        }
    }

    /// Check if this is a return instruction.
    pub fn is_return(self) -> bool {
        match self {
            Opcode::Ret | Opcode::RetValue => true,
            _ => false,
        }
    }
}

impl Inst {
    pub fn dump(self, dfg: &DataFlowGraph) -> InstDumper {
        InstDumper(self, dfg)
    }
}

/// Temporary object to dump an `Inst` in human-readable form for debugging.
pub struct InstDumper<'a>(Inst, &'a DataFlowGraph);

impl std::fmt::Display for InstDumper<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let inst = self.0;
        let dfg = self.1;
        let data = &dfg[inst];
        if dfg.has_result(inst) {
            let result = dfg.inst_result(inst);
            write!(
                f,
                "%{} = {} {}",
                result,
                data.opcode(),
                dfg.value_type(result)
            )?;
        } else {
            write!(f, "{}", data.opcode())?;
        }
        if let InstData::Call { unit, .. } = *data {
            write!(f, " {}", dfg[unit].name)?;
            write!(f, " (")?;
            let mut comma = false;
            for arg in data.input_args() {
                if comma {
                    write!(f, ", ")?;
                }
                write!(f, "%{}", arg)?;
                comma = true;
            }
            write!(f, ")")?;
            if data.opcode() == Opcode::Inst {
                write!(f, " -> (")?;
                let mut comma = false;
                for arg in data.output_args() {
                    if comma {
                        write!(f, ", ")?;
                    }
                    write!(f, "%{}", arg)?;
                    comma = true;
                }
                write!(f, ")")?;
            }
        } else if let InstData::Reg { .. } = *data {
            write!(f, " {}", data.args()[0])?;
            for arg in data.data_args() {
                write!(f, ", %{}", arg)?;
            }
            for arg in data.mode_args() {
                write!(f, ", {}", arg)?;
            }
            for arg in data.trigger_args() {
                write!(f, ", %{}", arg)?;
            }
        } else {
            let mut comma = false;
            for arg in data.args() {
                if comma {
                    write!(f, ",")?;
                }
                write!(f, " %{}", arg)?;
                comma = true;
            }
            for block in data.blocks() {
                if comma {
                    write!(f, ",")?;
                }
                write!(f, " %{}", block)?;
                comma = true;
            }
            match data {
                InstData::ConstInt { imm, .. } => write!(f, " {}", imm)?,
                InstData::ConstTime { imm, .. } => write!(f, " {}", imm)?,
                InstData::Array { imm, .. } => write!(f, ", {}", imm)?,
                _ => (),
            }
        }
        Ok(())
    }
}
