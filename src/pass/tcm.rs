// Copyright (c) 2017-2019 Fabian Schuiki

//! Temporal Code Motion

use crate::ir::prelude::*;
use crate::opt::prelude::*;
use crate::{
    ir::{DataFlowGraph, FunctionLayout, InstData},
    pass::gcse::{DominatorTree, PredecessorTable},
    value::IntValue,
};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    ops::Index,
};

/// Temporal Code Motion
///
/// This pass rearranges temporal instructions. It does the following:
///
/// - Merge multiple identical waits into one (in a new block).
/// - Move `prb` instructions up to the top of the time region.
/// - Move `drv` instructions down to the end of the time region, where
///   possible. Failure to do so hints at conditionally-driven signals, such as
///   storage elements.
///
pub struct TemporalCodeMotion;

impl Pass for TemporalCodeMotion {
    fn run_on_cfg(ctx: &PassContext, unit: &mut impl UnitBuilder) -> bool {
        info!("TCM [{}]", unit.unit().name());
        let mut modified = false;

        // Build the temporal region graph.
        let trg = TemporalRegionGraph::new(unit.dfg(), unit.func_layout());

        // Hoist `prb` instructions which directly operate on input signals to
        // the head block of their region.
        let temp_pt = PredecessorTable::new_temporal(unit.dfg(), unit.func_layout());
        let temp_dt = DominatorTree::new(unit.cfg(), unit.func_layout(), &temp_pt);
        for tr in &trg.regions {
            let dfg = unit.dfg();
            let layout = unit.func_layout();
            if tr.head_blocks.len() != 1 {
                trace!("Skipping {} for prb move (multiple head blocks)", tr.id);
                continue;
            }
            let head_bb = tr.head_blocks().next().unwrap();
            let mut hoist = vec![];
            for bb in tr.blocks() {
                for inst in layout.insts(bb) {
                    if dfg[inst].opcode() == Opcode::Prb
                        && dfg.get_value_inst(dfg[inst].args()[0]).is_none()
                    {
                        // Check if the new prb location would dominate its old
                        // location temporally.
                        let mut dominates = temp_dt.dominates(head_bb, bb);

                        // Only move when the move instruction would still
                        // dominate all its uses.
                        for (user_inst, _) in dfg.uses(dfg.inst_result(inst)) {
                            let user_bb = unit.func_layout().inst_block(user_inst).unwrap();
                            let dom = temp_dt.dominates(head_bb, user_bb);
                            dominates &= dom;
                        }
                        if dominates {
                            hoist.push(inst);
                        } else {
                            trace!(
                                "Skipping {} for prb move (would not dominate uses)",
                                inst.dump(dfg, unit.try_cfg())
                            );
                        }
                    }
                }
            }
            hoist.sort();
            for inst in hoist {
                debug!(
                    "Hoisting {} into {}",
                    inst.dump(unit.dfg(), unit.try_cfg()),
                    head_bb.dump(unit.cfg())
                );
                let layout = unit.func_layout_mut();
                layout.remove_inst(inst);
                layout.prepend_inst(inst, head_bb);
                modified = true;
            }
        }

        // Fuse equivalent wait instructions.
        let trg = TemporalRegionGraph::new(unit.dfg(), unit.func_layout());
        for tr in &trg.regions {
            if tr.tail_insts.len() <= 1 {
                trace!("Skipping {} for wait merge (single wait inst)", tr.id);
                continue;
            }
            let mut merge = HashMap::<&InstData, Vec<Inst>>::new();
            for inst in tr.tail_insts() {
                merge.entry(&unit.dfg()[inst]).or_default().push(inst);
            }
            let merge: Vec<_> = merge.into_iter().map(|(_, is)| is).collect();
            for insts in merge {
                if insts.len() <= 1 {
                    trace!(
                        "Skipping {} (no equivalents)",
                        insts[0].dump(unit.dfg(), unit.try_cfg())
                    );
                    continue;
                }
                trace!("Merging:",);
                for i in &insts {
                    trace!("  {}", i.dump(unit.dfg(), unit.try_cfg()));
                }

                // Create a new basic block for the singleton wait inst.
                let unified_bb = unit.block();

                // Replace all waits with branches into the unified block.
                for &inst in &insts {
                    unit.insert_after(inst);
                    unit.ins().br(unified_bb);
                }

                // Add one of the instructions to the unified block and delete
                // the rest.
                unit.func_layout_mut().remove_inst(insts[0]);
                unit.func_layout_mut().append_inst(insts[0], unified_bb);
                for &inst in &insts[1..] {
                    unit.remove_inst(inst);
                }
                modified = true;
            }
        }

        // Introduce auxiliary exit blocks if multiple edges leave a temporal
        // region into the same target block in a different region. This is
        // needed to ensure that drives have a dedicated block to be pushed
        // down into ahead of the next temporal region.
        modified |= add_aux_blocks(ctx, unit);

        // Push `drv` instructions towards the tails of their temporal regions.
        modified |= push_drives(ctx, unit);

        // TODO: Coalesce drives to the same signal.

        modified
    }
}

/// Introduce auxiliary exit blocks if multiple edges leave a temporal region
/// into the same target block in a different region. This is needed to ensure
/// that drives have a dedicated block to be pushed down into ahead of the next
/// temporal region.
fn add_aux_blocks(_ctx: &PassContext, unit: &mut impl UnitBuilder) -> bool {
    let pt = PredecessorTable::new(unit.dfg(), unit.func_layout());
    let trg = TemporalRegionGraph::new(unit.dfg(), unit.func_layout());
    let mut modified = false;

    // Make a list of head blocks. This will allow us to change the unit
    // underneath.
    let head_bbs: Vec<_> = unit
        .func_layout()
        .blocks()
        .filter(|&bb| trg.is_head(bb))
        .collect();

    // Process each block separately.
    for bb in head_bbs {
        trace!("Adding aux blocks into {}", bb.dump(unit.cfg()));
        let tr = trg[bb];

        // Gather a list of predecessor instructions per region, which branch
        // into this block.
        let mut insts_by_region = HashMap::<TemporalRegion, Vec<Inst>>::new();
        for pred in pt.pred(bb) {
            let pred_tr = trg[pred];
            if pred_tr != tr {
                let inst = unit.func_layout().terminator(pred);
                insts_by_region.entry(pred_tr).or_default().push(inst);
            }
        }

        // For each entry with more than one instruction, create an auxiliary
        // entry block.
        for (src_tr, insts) in insts_by_region {
            if insts.len() < 2 {
                trace!("  Skipping {} (single head inst)", src_tr);
                continue;
            }
            let aux_bb = unit.named_block("aux");
            unit.append_to(aux_bb);
            unit.ins().br(bb);
            trace!("  Adding {} from {}", aux_bb.dump(unit.cfg()), src_tr);
            for inst in insts {
                trace!(
                    "    Replacing {} in {}",
                    bb.dump(unit.cfg()),
                    inst.dump(unit.dfg(), unit.try_cfg())
                );
                unit.dfg_mut()[inst].replace_block(bb, aux_bb);
            }
            modified = true;
        }
    }

    modified
}

/// Push `drv` instructions downards into the tails of their temporal regions.
fn push_drives(ctx: &PassContext, unit: &mut impl UnitBuilder) -> bool {
    let mut modified = false;

    // We need the dominator tree of the current CFG.
    let pt = PredecessorTable::new(unit.dfg(), unit.func_layout());
    let dt = DominatorTree::new(unit.cfg(), unit.func_layout(), &pt);

    // Build an alias table of all signals, which indicates which signals are
    // aliases (e.g. extf/exts) of another. As we encounter drives, keep track
    // of their sequential dependency.
    let mut aliases = HashMap::<Value, Value>::new();
    let mut drv_seq = HashMap::<Value, Vec<Inst>>::new();
    let dfg = unit.dfg();
    let cfg = unit.cfg();
    for &bb in dt.blocks_post_order().iter().rev() {
        trace!("Checking {} for aliases", bb.dump(unit.cfg()));
        for inst in unit.func_layout().insts(bb) {
            let data = &dfg[inst];
            if let Opcode::Drv | Opcode::DrvCond = data.opcode() {
                // Gather drive sequences to the same signal.
                let signal = data.args()[0];
                let signal = aliases.get(&signal).cloned().unwrap_or(signal);
                trace!(
                    "  Drive {} ({})",
                    signal.dump(dfg),
                    inst.dump(dfg, Some(cfg))
                );
                drv_seq.entry(signal).or_default().push(inst);
            } else if let Some(value) = dfg.get_inst_result(inst) {
                // Gather signal aliases.
                if !dfg.value_type(value).is_signal() {
                    continue;
                }
                for &arg in data.args() {
                    if !dfg.value_type(arg).is_signal() {
                        continue;
                    }
                    let arg = aliases.get(&arg).cloned().unwrap_or(arg);
                    trace!(
                        "  Alias {} of {} ({})",
                        value.dump(dfg),
                        arg.dump(dfg),
                        inst.dump(dfg, Some(cfg))
                    );
                    aliases.insert(value, arg);
                }
            }
        }
    }

    // Build the temporal region graph.
    let trg = TemporalRegionGraph::new(unit.dfg(), unit.func_layout());

    // Try to migrate drive instructions into the tails of their respective
    // temporal regions.
    for (&signal, drives) in &drv_seq {
        trace!("Moving drives on signal {}", signal.dump(unit.dfg()));
        // TODO: Don't directly move drives, but track if move is possible and what
        // the conditions are. Then do post-processing down below.
        for &drive in drives.iter().rev() {
            // Skip drives that are already in the right place.
            let drive_bb = unit.func_layout().inst_block(drive).unwrap();
            if trg.is_tail(drive_bb) {
                trace!(
                    "  Skipping {} (already in tail block)",
                    drive.dump(unit.dfg(), unit.try_cfg()),
                );
                continue;
            }
            if trg[trg[drive_bb]].tail_blocks.is_empty() {
                trace!(
                    "  Skipping {} (no tail blocks)",
                    drive.dump(unit.dfg(), unit.try_cfg()),
                );
                continue;
            }

            // Perform the move.
            // trace!("  Checking {}", drive.dump(unit.dfg(), unit.try_cfg()));
            let moved = push_drive(ctx, drive, unit, &dt, &trg);
            modified |= moved;

            // If the move was not possible, abort all other drives since we
            // cannot move over them.
            if !moved {
                break;
            }
        }
    }

    // Coalesce drives. We do this one aliasing group at a time.
    for block in unit.func_layout().blocks().collect::<Vec<_>>() {
        modified |= coalesce_drives(ctx, block, unit);
    }

    modified
}

fn push_drive(
    _ctx: &PassContext,
    drive: Inst,
    unit: &mut impl UnitBuilder,
    dt: &DominatorTree,
    trg: &TemporalRegionGraph,
) -> bool {
    let dfg = unit.dfg();
    let cfg = unit.cfg();
    let layout = unit.func_layout();
    let src_bb = layout.inst_block(drive).unwrap();
    let tr = trg[src_bb];
    let mut moves = Vec::new();

    // For each tail block that this drive moves to, find the branch conditions
    // along the way that lead to the drive being executed, and check if the
    // arguments for the drive are available in the destination block.
    for dst_bb in trg[tr].tail_blocks() {
        // trace!("    Will have to move to {}", dst_bb.dump(cfg));

        // First check if all arguments of the drive instruction dominate the
        // destination block. If not, the move is not possible.
        for &arg in dfg[drive].args() {
            if !dt.value_dominates_block(dfg, layout, arg, dst_bb) {
                trace!(
                    "  Skipping {} ({} does not dominate {})",
                    drive.dump(dfg, Some(cfg)),
                    arg.dump(dfg),
                    dst_bb.dump(cfg)
                );
                return false;
            }
        }

        // Find the branch conditions that lead to src_bb being executed on the
        // way to dst_bb. We do this by stepping up the dominator tree until we
        // find the common dominator. For every step of src_bb, we record the
        // branch condition.
        let mut src_finger = src_bb;
        let mut dst_finger = dst_bb;
        let mut conds = Vec::<(Value, bool)>::new();
        while src_finger != dst_finger {
            let i1 = dt.block_order(src_finger);
            let i2 = dt.block_order(dst_finger);
            if i1 < i2 {
                let parent = dt.dominator(src_finger);
                if src_finger == parent {
                    break;
                }
                let term = layout.terminator(parent);
                if dfg[term].opcode() == Opcode::BrCond {
                    let cond_val = dfg[term].args()[0];
                    if !dt.value_dominates_block(dfg, layout, cond_val, dst_bb) {
                        trace!(
                            "  Skipping {} (branch cond {} does not dominate {})",
                            drive.dump(dfg, Some(cfg)),
                            cond_val.dump(dfg),
                            dst_bb.dump(cfg)
                        );
                        return false;
                    }
                    let cond_pol = dfg[term].blocks().iter().position(|&bb| bb == src_finger);
                    if let Some(cond_pol) = cond_pol {
                        conds.push((cond_val, cond_pol != 0));
                        trace!(
                            "    {} -> {} ({} == {})",
                            parent.dump(cfg),
                            src_finger.dump(cfg),
                            cond_val.dump(dfg),
                            cond_pol
                        );
                    }
                } else {
                    trace!("    {} -> {}", parent.dump(cfg), src_finger.dump(cfg));
                }
                src_finger = parent;
            } else if i2 < i1 {
                let parent = dt.dominator(dst_finger);
                if dst_finger == parent {
                    break;
                }
                dst_finger = parent;
            }
        }
        if src_finger != dst_finger {
            trace!(
                "  Skipping {} (no common dominator)",
                drive.dump(dfg, Some(cfg))
            );
            return false;
        }

        // Keep note of this destination block and the conditions that must
        // hold.
        moves.push((dst_bb, conds));
    }

    // If we arrive here, all moves are possible and can now be executed.
    for (dst_bb, conds) in moves {
        debug!(
            "Moving {} to {}",
            drive.dump(unit.dfg(), unit.try_cfg()),
            dst_bb.dump(unit.cfg())
        );

        // Start by assembling the drive condition in the destination block. The
        // order is key here to allow for easy constant folding and subexpr
        // elimination: the conditions are in reverse CFG order, so and them
        // together in reverse order to reflect the CFG, which allows for most
        // of these conditions to be shared.
        unit.prepend_to(dst_bb);
        let mut cond = unit.ins().const_int(IntValue::all_ones(1));
        for (value, polarity) in conds.into_iter().rev() {
            let value = match polarity {
                true => value,
                false => unit.ins().not(value),
            };
            cond = unit.ins().and(cond, value);
        }

        // Add the drive condition, if any.
        if unit.dfg()[drive].opcode() == Opcode::DrvCond {
            let arg = unit.dfg()[drive].args()[3];
            cond = unit.ins().and(cond, arg);
        }

        // Insert the new drive.
        let args = unit.dfg()[drive].args();
        let signal = args[0];
        let value = args[1];
        let delay = args[2];
        unit.ins().drv_cond(signal, value, delay, cond);
    }

    // Remove the old drive instruction.
    unit.remove_inst(drive);

    true
}

fn coalesce_drives(_ctx: &PassContext, block: Block, unit: &mut impl UnitBuilder) -> bool {
    let mut modified = false;
    let dfg = unit.dfg();

    // Group the drives by delay.
    let mut delay_groups = HashMap::<Value, Vec<Inst>>::new();
    for inst in unit.func_layout().insts(block) {
        if let Opcode::Drv | Opcode::DrvCond = dfg[inst].opcode() {
            let delay = dfg[inst].args()[2];
            delay_groups.entry(delay).or_default().push(inst);
        }
    }

    // Coalesce each delay group individually. Split the instructions into runs
    // of drives to the exact same signal.
    for (delay, drives) in delay_groups {
        let runs: Vec<_> = drives
            .into_iter()
            .group_by(|&inst| unit.dfg()[inst].args()[0])
            .into_iter()
            .map(|(target, drives)| (target, drives.collect::<Vec<_>>()))
            .collect();
        for (target, drives) in runs {
            if drives.len() <= 1 {
                continue;
            }
            debug!(
                "Coalescing {} drives on {}",
                drives.len(),
                target.dump(unit.dfg())
            );
            let mut drives = drives.into_iter();

            // Get the first drive's value and condition, and remove the drive.
            let first = drives.next().unwrap();
            unit.insert_before(first);
            let mut cond = drive_cond(unit, first);
            let mut value = unit.dfg()[first].args()[1];
            unit.remove_inst(first);

            // Accumulate subsequent drive conditions and values, and remove.
            for drive in drives {
                unit.insert_before(drive);
                let c = drive_cond(unit, drive);
                let v = unit.dfg()[drive].args()[1];
                if cond != c {
                    cond = unit.ins().or(cond, c);
                }
                if value != v {
                    let vs = unit.ins().array(vec![value, v]);
                    value = unit.ins().mux(vs, c);
                }
                unit.remove_inst(drive);
            }

            // Build the final drive.
            unit.ins().drv_cond(target, value, delay, cond);
            modified = true;
        }
    }

    // TODO: Collapse drives with the same value and delay.
    // TODO: Build discriminator table for all drives, then build corresponding
    // mux to select driven value, and use or of all drive conditions as new
    // drive condition.

    modified
}

fn drive_cond(unit: &mut impl UnitBuilder, inst: Inst) -> Value {
    if unit.dfg()[inst].opcode() == Opcode::DrvCond {
        unit.dfg()[inst].args()[3]
    } else {
        unit.ins().const_int(IntValue::all_ones(1))
    }
}

/// A data structure that temporally groups blocks and instructions.
#[derive(Debug)]
pub struct TemporalRegionGraph {
    /// Map that assigns blocks into a region.
    blocks: HashMap<Block, TemporalRegion>,
    /// Actual region information.
    regions: Vec<TemporalRegionData>,
}

impl TemporalRegionGraph {
    /// Compute the TRG of a process.
    pub fn new(dfg: &DataFlowGraph, layout: &FunctionLayout) -> Self {
        trace!("Constructing TRG:");

        // Populate the worklist with the entry block, as well as any blocks
        // that are targeted by `wait` instructions.
        let mut todo = VecDeque::new();
        let mut seen = HashSet::new();
        todo.push_back(layout.entry());
        seen.insert(layout.entry());
        trace!("  Root {:?} (entry)", layout.entry());
        for bb in layout.blocks() {
            let term = layout.terminator(bb);
            if dfg[term].opcode().is_temporal() {
                for &target in dfg[term].blocks() {
                    if seen.insert(target) {
                        trace!("  Root {:?} (wait target)", target);
                        todo.push_back(target);
                    }
                }
            }
        }

        // Assign the root temporal regions.
        let mut next_id = 0;
        let mut blocks = HashMap::<Block, TemporalRegion>::new();
        let mut head_blocks = HashSet::new();
        let mut tail_blocks = HashSet::new();
        let mut breaks = vec![];
        for &bb in &todo {
            blocks.insert(bb, TemporalRegion(next_id));
            head_blocks.insert(bb);
            next_id += 1;
        }

        // Assign temporal regions to the blocks.
        while let Some(bb) = todo.pop_front() {
            let tr = blocks[&bb];
            trace!("  Pushing {:?} ({})", bb, tr);
            let term = layout.terminator(bb);
            if dfg[term].opcode().is_temporal() {
                breaks.push(term);
                tail_blocks.insert(bb);
                continue;
            }
            for &target in dfg[term].blocks() {
                if seen.insert(target) {
                    todo.push_back(target);
                    trace!("    Assigning {:?} <- {:?}", target, tr);
                    if blocks.insert(target, tr).is_some() {
                        let tr = TemporalRegion(next_id);
                        blocks.insert(target, tr);
                        head_blocks.insert(target);
                        tail_blocks.insert(bb);
                        trace!("    Assigning {:?} <- {:?} (override)", target, tr);
                        next_id += 1;
                    }
                }
            }
        }
        trace!("  Blocks: {:#?}", blocks);

        // Create a data struct for each region.
        let mut regions: Vec<_> = (0..next_id)
            .map(|id| TemporalRegionData {
                id: TemporalRegion(id),
                blocks: Default::default(),
                entry: false,
                head_insts: Default::default(),
                head_blocks: Default::default(),
                head_tight: true,
                tail_insts: Default::default(),
                tail_blocks: Default::default(),
                tail_tight: true,
            })
            .collect();

        // Mark the entry block.
        regions[blocks[&layout.entry()].0].entry = true;

        // Build the predecessor table.
        let pt = PredecessorTable::new(dfg, layout);

        // Note the blocks in each region and build the head/tail information.
        for (&bb, &id) in &blocks {
            let mut reg = &mut regions[id.0];
            reg.blocks.insert(bb);

            // Determine whether this is a head block.
            let mut is_head = head_blocks.contains(&bb);
            let mut is_tight = true;
            for pred in pt.pred(bb) {
                let diff_trs = blocks[&pred] != id;
                is_head |= diff_trs;
                is_tight &= diff_trs;
            }
            if is_head {
                reg.head_blocks.insert(bb);
                reg.head_tight &= is_tight;
            }

            // Determine whether this is a tail block.
            let mut is_tail = tail_blocks.contains(&bb);
            let mut is_tight = true;
            for succ in pt.succ(bb) {
                let diff_trs = blocks[&succ] != id;
                is_tail |= diff_trs;
                is_tight &= diff_trs;
            }
            if is_tail {
                reg.tail_blocks.insert(bb);
                reg.tail_tight &= is_tight;
            }

            // Note the head instructions.
            for pred in pt.pred(bb) {
                if blocks[&pred] != id {
                    reg.head_insts.insert(layout.terminator(pred));
                }
            }

            // Note the tail instructions.
            let term = layout.terminator(bb);
            if dfg[term].blocks().iter().any(|bb| blocks[bb] != id) {
                reg.tail_insts.insert(term);
            }
        }

        Self { blocks, regions }
    }

    /// Check if a block is a temporal head block.
    pub fn is_head(&self, bb: Block) -> bool {
        self[self[bb]].is_head(bb)
    }

    /// Check if a block is a temporal tail block.
    pub fn is_tail(&self, bb: Block) -> bool {
        self[self[bb]].is_tail(bb)
    }

    /// Get the temporal regions in the graph.
    pub fn regions(&self) -> impl Iterator<Item = (TemporalRegion, &TemporalRegionData)> {
        self.regions
            .iter()
            .enumerate()
            .map(|(i, tr)| (TemporalRegion(i), tr))
    }
}

impl Index<TemporalRegion> for TemporalRegionGraph {
    type Output = TemporalRegionData;
    fn index(&self, idx: TemporalRegion) -> &Self::Output {
        &self.regions[idx.0]
    }
}

impl Index<Block> for TemporalRegionGraph {
    type Output = TemporalRegion;
    fn index(&self, idx: Block) -> &Self::Output {
        &self.blocks[&idx]
    }
}

/// A unique identifier for a temporal region.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TemporalRegion(usize);

impl std::fmt::Display for TemporalRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "t{}", self.0)
    }
}

impl std::fmt::Debug for TemporalRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

/// Data associated with a temporal region.
#[derive(Debug, Clone)]
pub struct TemporalRegionData {
    /// The unique identifier for this region.
    pub id: TemporalRegion,

    /// The blocks in this region.
    pub blocks: HashSet<Block>,

    /// Whether this is the initial temporal region upon entering the process.
    pub entry: bool,

    /// The temporal instructions that introduce this region.
    ///
    /// Note that these reside in blocks *outside* this region, namely in the
    /// predecessors of the `head_blocks`.
    pub head_insts: HashSet<Inst>,

    /// The entry blocks into this region.
    ///
    /// These are the first blocks that are jumped into upon entering this
    /// region.
    pub head_blocks: HashSet<Block>,

    /// The head blocks are only reachable via branches from *other* regions.
    pub head_tight: bool,

    /// The temporal instructions that terminate this region.
    ///
    /// Note that these reside in blocks *inside* this region, namely in the
    /// `tail_blocks`.
    pub tail_insts: HashSet<Inst>,

    /// The exit blocks out of this region.
    ///
    /// These are the last blocks in this region, where execution either ends
    /// in a `wait` or `halt` instruction.
    pub tail_blocks: HashSet<Block>,

    /// The tail blocks only branch to *other* regions.
    pub tail_tight: bool,
}

impl TemporalRegionData {
    /// An iterator over the blocks in this region.
    pub fn blocks(&self) -> impl Iterator<Item = Block> + Clone + '_ {
        self.blocks.iter().cloned()
    }

    /// An iterator over the head instructions in this region.
    pub fn head_insts(&self) -> impl Iterator<Item = Inst> + Clone + '_ {
        self.head_insts.iter().cloned()
    }

    /// An iterator over the head blocks in this region.
    pub fn head_blocks(&self) -> impl Iterator<Item = Block> + Clone + '_ {
        self.head_blocks.iter().cloned()
    }

    /// An iterator over the tail instructions in this region.
    pub fn tail_insts(&self) -> impl Iterator<Item = Inst> + Clone + '_ {
        self.tail_insts.iter().cloned()
    }

    /// An iterator over the tail blocks in this region.
    pub fn tail_blocks(&self) -> impl Iterator<Item = Block> + Clone + '_ {
        self.tail_blocks.iter().cloned()
    }

    /// Check if a block is a temporal head block.
    pub fn is_head(&self, bb: Block) -> bool {
        self.head_blocks.contains(&bb)
    }

    /// Check if a block is a temporal tail block.
    pub fn is_tail(&self, bb: Block) -> bool {
        self.tail_blocks.contains(&bb)
    }
}
