#pragma once
#include <stdio.h>

#define TYPEDEF(name) typedef struct llhd_##name llhd_##name##_t
TYPEDEF(module);
TYPEDEF(value);
TYPEDEF(unit);
TYPEDEF(func);
TYPEDEF(proc);
TYPEDEF(arg);
TYPEDEF(entity);
TYPEDEF(basic_block);
TYPEDEF(inst);
TYPEDEF(drive_inst);
TYPEDEF(branch_inst);
TYPEDEF(compare_inst);
TYPEDEF(type);
#undef TYPEDEF

typedef enum llhd_compare_mode llhd_compare_mode_t;


struct llhd_value {
	struct llhd_value_intf *_intf;
	char *name;
	llhd_type_t *type;
};

typedef void (*llhd_value_intf_dispose_fn)(void*);
typedef void (*llhd_value_intf_dump_fn)(void*, FILE*);
struct llhd_value_intf {
	llhd_value_intf_dispose_fn dispose;
	llhd_value_intf_dump_fn dump;
};

struct llhd_arg {
	llhd_value_t _base;
	llhd_unit_t *parent;
};

struct llhd_func {
	llhd_value_t base;
	llhd_module_t *parent;
	llhd_basic_block_t *bb_head;
	llhd_basic_block_t *bb_tail;
};

struct llhd_proc {
	llhd_value_t _base;
	llhd_module_t *parent;
	llhd_basic_block_t *bb_head;
	llhd_basic_block_t *bb_tail;
	unsigned num_in;
	llhd_arg_t **in;
	unsigned num_out;
	llhd_arg_t **out;
};

struct llhd_entity {
	llhd_value_t base;
	llhd_module_t *parent;
	llhd_inst_t *inst_head;
	llhd_inst_t *inst_tail;
};

// basic_block:
// - append to unit
// - insert before block
// - insert after block
// - remove from parent
// - dump
struct llhd_basic_block {
	llhd_value_t _base;
	llhd_unit_t *parent;
	llhd_basic_block_t *prev;
	llhd_basic_block_t *next;
	llhd_inst_t *inst_head;
	llhd_inst_t *inst_tail;
};

// inst:
// - append to basic block
// - insert after inst
// - insert before inst
// - remove from parent
// - dump
struct llhd_inst {
	llhd_value_t _base;
	llhd_basic_block_t *parent;
	llhd_inst_t *prev;
	llhd_inst_t *next;
};

struct llhd_drive_inst {
	llhd_inst_t _base;
	llhd_value_t *target;
	llhd_value_t *value;
};

struct llhd_branch_inst {
	llhd_inst_t _base;
	llhd_value_t *cond; // NULL if uncond
	llhd_basic_block_t *dst1; // dst if uncond
	llhd_basic_block_t *dst0; // NULL if uncond
};

enum llhd_compare_mode {
	LLHD_EQ,
	LLHD_NE,
	LLHD_UGT,
	LLHD_ULT,
	LLHD_UGE,
	LLHD_ULE,
	LLHD_SGT,
	LLHD_SLT,
	LLHD_SGE,
	LLHD_SLE,
};

struct llhd_compare_inst {
	llhd_inst_t _base;
	llhd_compare_mode_t mode;
	llhd_value_t *lhs;
	llhd_value_t *rhs;
};

void llhd_init_value(llhd_value_t *V, const char *name, llhd_type_t *type);
void llhd_dispose_value(void*);
void llhd_destroy_value(void*);
void llhd_dump_value(void*, FILE*);
void llhd_dump_value_name(void*, FILE*);
void llhd_value_set_name(void*, const char*);
const char *llhd_value_get_name(void*);

llhd_proc_t *llhd_make_proc(const char *name, llhd_arg_t **in, unsigned num_in, llhd_arg_t **out, unsigned num_out, llhd_basic_block_t *entry);

llhd_basic_block_t *llhd_make_basic_block(const char *name);
void llhd_insert_basic_block_before(llhd_basic_block_t *BB, llhd_basic_block_t *before);
void llhd_insert_basic_block_after(llhd_basic_block_t *BB, llhd_basic_block_t *after);

llhd_drive_inst_t *llhd_make_drive_inst(llhd_value_t *target, llhd_value_t *value);
llhd_compare_inst_t *llhd_make_compare_inst(llhd_compare_mode_t mode, llhd_value_t *lhs, llhd_value_t *rhs);
llhd_branch_inst_t *llhd_make_conditional_branch_inst(llhd_value_t *cond, llhd_basic_block_t *dst1, llhd_basic_block_t *dst0);
llhd_branch_inst_t *llhd_make_unconditional_branch_inst(llhd_basic_block_t *dst);

llhd_arg_t *llhd_make_arg(const char *name, llhd_type_t *type);

void llhd_add_inst(llhd_inst_t *I, llhd_basic_block_t *BB);
