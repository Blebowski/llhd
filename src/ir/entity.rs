// Copyright (c) 2017-2019 Fabian Schuiki

//! Representation of LLHD processes.

use crate::{
    ir::{
        Block, DataFlowGraph, EntityInsertPos, Inst, InstData, InstLayout, Signature, Unit,
        UnitBuilder, UnitKind, UnitName, ValueData,
    },
    ty::{signal_ty, Type},
};

/// An entity.
pub struct Entity {
    pub name: UnitName,
    pub sig: Signature,
    pub dfg: DataFlowGraph,
    pub layout: InstLayout,
}

impl Entity {
    /// Create a new entity.
    pub fn new(name: UnitName, sig: Signature) -> Self {
        assert!(!sig.has_return_type());
        let mut ent = Self {
            name,
            sig,
            dfg: DataFlowGraph::new(),
            layout: InstLayout::new(),
        };
        ent.dfg.make_args_for_signature(&ent.sig);
        ent
    }
}

impl Unit for Entity {
    fn kind(&self) -> UnitKind {
        UnitKind::Entity
    }

    fn dfg(&self) -> &DataFlowGraph {
        &self.dfg
    }

    fn dfg_mut(&mut self) -> &mut DataFlowGraph {
        &mut self.dfg
    }

    fn sig(&self) -> &Signature {
        &self.sig
    }

    fn sig_mut(&mut self) -> &mut Signature {
        &mut self.sig
    }

    fn dump_fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "entity {} {} {{\n", self.name, self.sig.dump(&self.dfg))?;
        for inst in self.layout.insts() {
            write!(f, "    {}\n", inst.dump(&self.dfg))?;
        }
        write!(f, "}}")?;
        Ok(())
    }
}

/// Temporary object used to build a single `Entity`.
pub struct EntityBuilder<'u> {
    /// The entity currently being built.
    pub entity: &'u mut Entity,
    /// The position where we are currently inserting instructions.
    pos: EntityInsertPos,
}

impl<'u> EntityBuilder<'u> {
    /// Create a new entity builder.
    pub fn new(entity: &mut Entity) -> EntityBuilder {
        EntityBuilder {
            entity,
            pos: EntityInsertPos::Append,
        }
    }
}

impl UnitBuilder for EntityBuilder<'_> {
    type Unit = Entity;

    fn unit(&self) -> &Entity {
        self.entity
    }

    fn unit_mut(&mut self) -> &mut Entity {
        self.entity
    }

    fn build_inst(&mut self, data: InstData, ty: Type) -> Inst {
        let ty = if !ty.is_signal() && !ty.is_void() && !data.opcode().is_const() {
            signal_ty(ty)
        } else {
            ty
        };
        let inst = self.entity.dfg.insts.add(data);
        if !ty.is_void() {
            let result = self.entity.dfg.values.add(ValueData::Inst { ty, inst });
            self.entity.dfg.results.add(inst, result);
        }
        match self.pos {
            EntityInsertPos::Append => self.entity.layout.append_inst(inst),
            EntityInsertPos::Prepend => self.entity.layout.prepend_inst(inst),
            EntityInsertPos::After(other) => self.entity.layout.insert_inst_after(inst, other),
            EntityInsertPos::Before(other) => self.entity.layout.insert_inst_before(inst, other),
        }
        inst
    }

    fn remove_inst(&mut self, inst: Inst) {
        self.entity.dfg.remove_inst(inst);
        self.entity.layout.remove_inst(inst);
    }

    fn block(&mut self) -> Block {
        panic!("block() called on entity");
    }

    fn insert_at_end(&mut self) {
        self.pos = EntityInsertPos::Append;
    }

    fn insert_at_beginning(&mut self) {
        self.pos = EntityInsertPos::Prepend;
    }

    fn append_to(&mut self, _: Block) {
        panic!("append_to() called on entity");
    }

    fn prepend_to(&mut self, _: Block) {
        panic!("prepend_to() called on entity");
    }

    fn insert_after(&mut self, inst: Inst) {
        self.pos = EntityInsertPos::After(inst);
    }

    fn insert_before(&mut self, inst: Inst) {
        self.pos = EntityInsertPos::Before(inst);
    }
}
