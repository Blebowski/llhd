// Copyright (c) 2017-2020 Fabian Schuiki

//! Verilog output writer

use anyhow::{bail, Result};
use itertools::Itertools;
use llhd::ir::Unit;
use std::{
    collections::{HashMap, HashSet},
    io::Write,
    iter::repeat,
    rc::Rc,
};

/// Emit a module as Verilog code.
pub fn write(output: &mut impl Write, module: &llhd::ir::Module) -> Result<()> {
    debug!("Emitting Verilog code");
    let mut skipped = vec![];
    for mod_unit in module.units() {
        if let Some(entity) = module.get_entity(mod_unit) {
            if entity.name().is_global() {
                write_entity(output, mod_unit, entity, &mut Context::default())?;
            }
        } else {
            let name = module[mod_unit].name();
            error!("Unit {} not supported", name);
            skipped.push(name);
        }
    }
    if !skipped.is_empty() {
        bail!(
            "Units not supported in Verilog output: {}",
            skipped.iter().format(", ")
        );
    }
    Ok(())
}

type UnitValue = (llhd::ir::ModUnit, llhd::ir::Value);

#[derive(Default)]
struct Context {
    name_map: HashMap<UnitValue, Rc<String>>,
    name_set: HashSet<Rc<String>>,
}

impl Context {
    /// Generate a printable name for a value.
    fn value_name(&mut self, dfg: &llhd::ir::DataFlowGraph, value: UnitValue) -> Rc<String> {
        if let Some(name) = self.name_map.get(&value).cloned() {
            return name;
        }
        let base_name: Rc<String> = Rc::new(match dfg.get_name(value.1) {
            Some(name) => sanitize_name(name).collect(),
            None => format!("__{}", value.1),
        });
        let mut name = base_name.clone();
        let mut i = 2;
        while self.name_set.contains(&name) {
            name = Rc::new(format!("{}_{}", base_name, i));
            i += 1;
        }
        self.name_map.insert(value, name.clone());
        self.name_set.insert(name.clone());
        name
    }
}

/// Emit an LLHD entity as a new Verilog module.
fn write_entity(
    output: &mut impl Write,
    mod_unit: llhd::ir::ModUnit,
    entity: &llhd::ir::Entity,
    ctx: &mut Context,
) -> Result<()> {
    let dfg = entity.dfg();
    let name = sanitize_unit_name(entity.name());
    debug!("Creating entity {} as `{}`", entity.name(), name);

    // Emit the module header.
    let ports = entity.args().map(|v| ctx.value_name(dfg, (mod_unit, v)));
    write!(output, "module {} ({});\n", name, ports.format(", "))?;

    // Emit the port declarations.
    let ports = entity
        .input_args()
        .zip(repeat("input"))
        .chain(entity.output_args().zip(repeat("output")));
    for (v, dir) in ports {
        let n = ctx.value_name(dfg, (mod_unit, v));
        write!(
            output,
            "    {} {} {};\n",
            dir,
            flatten_type(&dfg.value_type(v))?,
            n
        )?;
    }

    write_entity_body(output, mod_unit, entity, ctx, Default::default())?;
    write!(output, "\nendmodule\n\n")?;
    Ok(())
}

/// Emit an LLHD entity within an existing Verilog module.
fn write_entity_body(
    output: &mut impl Write,
    mod_unit: llhd::ir::ModUnit,
    entity: &llhd::ir::Entity,
    ctx: &mut Context,
    bound: HashMap<llhd::ir::Value, llhd::ir::Value>,
) -> Result<()> {
    debug!("Emitting entity {}", entity.name());
    write!(output, "\n    // Entity {}\n", entity.name())?;
    let dfg = entity.dfg();
    for inst in entity.inst_layout().insts() {
        write!(output, "    // {}\n", inst.dump(dfg, None))?;
    }
    Ok(())
}

/// Make a unit name printable in Verilog.
fn sanitize_unit_name(name: &llhd::ir::UnitName) -> String {
    let mut out = String::new();
    if !name.is_global() {
        out.push('_');
    }
    match name {
        llhd::ir::UnitName::Global(s) | llhd::ir::UnitName::Local(s) => {
            out.extend(sanitize_name(s))
        }
        llhd::ir::UnitName::Anonymous(i) => out.push_str(&i.to_string()),
    }
    out
}

/// Make a name printable in Verilog.
fn sanitize_name(name: &str) -> impl Iterator<Item = char> + '_ {
    name.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
}

/// Emit a type.
fn flatten_type(ty: &llhd::Type) -> Result<String> {
    let bits = sizeof_type(ty)?;
    Ok(if bits > 1 {
        format!("[{}:0]", bits - 1)
    } else {
        "".to_string()
    })
}

/// Compute the number of bits in a type.
fn sizeof_type(ty: &llhd::Type) -> Result<usize> {
    match ty.as_ref() {
        llhd::VoidType => Ok(0),
        llhd::IntType(w) => Ok(*w),
        llhd::EnumType(w) => {
            Ok((usize::max_value().count_ones() - w.next_power_of_two().leading_zeros()) as usize)
        }
        llhd::SignalType(ty) => Ok(sizeof_type(ty)?),
        llhd::ArrayType(w, ty) => Ok(w * sizeof_type(ty)?),
        llhd::StructType(tys) => tys.iter().map(|ty| sizeof_type(ty)).sum(),
        _ => bail!("Type `{}` not supported", ty),
    }
}
