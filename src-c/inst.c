/* Copyright (c) 2016 Fabian Schuiki */
#include "value.h"
#include "inst.h"
#include <llhd.h>
#include <assert.h>
#include <string.h>

/**
 * @file
 * @author Fabian Schuiki <fabian@schuiki.ch>
 *
 * Guidelines:
 * - insts ref/unref their arguments
 * - insts use/unuse their arguments
 *
 * @todo Automate handling of uses: automatically ref/unref and use/unuse args,
 *       have one generic substitute and unlink_uses function.
 * @todo Factor handling of inst->type and inst->name out into alloc_inst and
 *       dispose_inst helper functions.
 */

static void unlink_from_parent(void*);

static void binary_dispose(void*);
static void binary_substitute(void*,void*,void*);
static void binary_unlink_uses(void*);

static void *compare_copy(void*);
static void compare_dispose(void*);
static void compare_substitute(void*,void*,void*);
static void compare_unlink_uses(void*);

static void branch_dispose(void*);
static void branch_substitute(void*,void*,void*);
static void branch_unlink_uses(void*);

static void drive_dispose(void*);
static void drive_substitute(void*,void*,void*);
static void drive_unlink_uses(void*);

static void signal_dispose(void*);

static void ret_dispose(void*);
static void ret_substitute(void*,void*,void*);
static void ret_unlink_uses(void*);

static void inst_dispose(void*);
static void inst_substitute(void*,void*,void*);
static void inst_unlink_uses(void*);

static void call_dispose(void*);
static void call_substitute(void*,void*,void*);
static void call_unlink_uses(void*);

static void unary_dispose(void*);
static void unary_substitute(void*,void*,void*);
static void unary_unlink_uses(void*);

static void extract_dispose(void*);
static void extract_substitute(void*,void*,void*);
static void extract_unlink_uses(void*);

static void insert_dispose(void*);
static void insert_substitute(void*,void*,void*);
static void insert_unlink_uses(void*);

static void reg_dispose(void*);
static void reg_substitute(void*,void*,void*);
static void reg_unlink_uses(void*);

static struct llhd_inst_vtbl vtbl_binary_inst = {
	.super = {
		.kind = LLHD_INST_BINARY,
		.type_offset = offsetof(struct llhd_inst, type),
		.name_offset = offsetof(struct llhd_inst, name),
		.kind_offset = offsetof(struct llhd_binary_inst, op),
		.dispose_fn = binary_dispose,
		.substitute_fn = binary_substitute,
		.unlink_from_parent_fn = unlink_from_parent,
		.unlink_uses_fn = binary_unlink_uses,
	},
	.num_uses = 2,
	.uses_offset = offsetof(struct llhd_binary_inst, uses),
};

static struct llhd_inst_vtbl vtbl_compare_inst = {
	.super = {
		.kind = LLHD_INST_COMPARE,
		.type_offset = offsetof(struct llhd_inst, type),
		.name_offset = offsetof(struct llhd_inst, name),
		.kind_offset = offsetof(struct llhd_compare_inst, op),
		.copy_fn = compare_copy,
		.dispose_fn = compare_dispose,
		.substitute_fn = compare_substitute,
		.unlink_from_parent_fn = unlink_from_parent,
		.unlink_uses_fn = compare_unlink_uses,
	},
	.num_uses = 2,
	.uses_offset = offsetof(struct llhd_compare_inst, uses),
};

static struct llhd_inst_vtbl vtbl_sig_inst = {
	.super = {
		.kind = LLHD_INST_SIGNAL,
		.type_offset = offsetof(struct llhd_inst, type),
		.name_offset = offsetof(struct llhd_inst, name),
		.dispose_fn = signal_dispose,
		.unlink_from_parent_fn = unlink_from_parent,
	},
};

static struct llhd_inst_vtbl vtbl_branch_inst = {
	.super = {
		.kind = LLHD_INST_BRANCH,
		.type_offset = offsetof(struct llhd_inst, type),
		.name_offset = offsetof(struct llhd_inst, name),
		.dispose_fn = branch_dispose,
		.substitute_fn = branch_substitute,
		.unlink_from_parent_fn = unlink_from_parent,
		.unlink_uses_fn = branch_unlink_uses,
	},
	.num_uses = 3,
	.uses_offset = offsetof(struct llhd_branch_inst, uses),
};

static struct llhd_inst_vtbl vtbl_drive_inst = {
	.super = {
		.kind = LLHD_INST_DRIVE,
		.dispose_fn = drive_dispose,
		.substitute_fn = drive_substitute,
		.unlink_from_parent_fn = unlink_from_parent,
		.unlink_uses_fn = drive_unlink_uses,
	},
	.num_uses = 2,
	.uses_offset = offsetof(struct llhd_drive_inst, uses),
};

static struct llhd_inst_vtbl vtbl_ret_inst = {
	.super = {
		.kind = LLHD_INST_RET,
		.dispose_fn = ret_dispose,
		.substitute_fn = ret_substitute,
		.unlink_from_parent_fn = unlink_from_parent,
		.unlink_uses_fn = ret_unlink_uses,
	},
};

static struct llhd_inst_vtbl vtbl_inst_inst = {
	.super = {
		.kind = LLHD_INST_INST,
		.name_offset = offsetof(struct llhd_inst, name),
		.dispose_fn = inst_dispose,
		.substitute_fn = inst_substitute,
		.unlink_from_parent_fn = unlink_from_parent,
		.unlink_uses_fn = inst_unlink_uses,
	},
	/// @todo add uses
};

static struct llhd_inst_vtbl vtbl_call_inst = {
	.super = {
		.kind = LLHD_INST_CALL,
		.name_offset = offsetof(struct llhd_inst, name),
		.type_offset = offsetof(struct llhd_inst, type),
		.dispose_fn = call_dispose,
		.substitute_fn = call_substitute,
		.unlink_from_parent_fn = unlink_from_parent,
		.unlink_uses_fn = call_unlink_uses,
	},
	/// @todo add uses
};

static struct llhd_inst_vtbl vtbl_unary_inst = {
	.super = {
		.kind = LLHD_INST_UNARY,
		.name_offset = offsetof(struct llhd_inst, name),
		.type_offset = offsetof(struct llhd_inst, type),
		.kind_offset = offsetof(struct llhd_unary_inst, op),
		.dispose_fn = unary_dispose,
		.substitute_fn = unary_substitute,
		.unlink_from_parent_fn = unlink_from_parent,
		.unlink_uses_fn = unary_unlink_uses,
	},
	.num_uses = 1,
	.uses_offset = offsetof(struct llhd_unary_inst, use),
};

static struct llhd_inst_vtbl vtbl_extract_inst = {
	.super = {
		.kind = LLHD_INST_EXTRACT,
		.name_offset = offsetof(struct llhd_inst, name),
		.type_offset = offsetof(struct llhd_inst, type),
		.dispose_fn = extract_dispose,
		.substitute_fn = extract_substitute,
		.unlink_from_parent_fn = unlink_from_parent,
		.unlink_uses_fn = extract_unlink_uses,
	},
	.num_uses = 1,
	.uses_offset = offsetof(struct llhd_extract_inst, use),
};

static struct llhd_inst_vtbl vtbl_insert_inst = {
	.super = {
		.kind = LLHD_INST_INSERT,
		.name_offset = offsetof(struct llhd_inst, name),
		.type_offset = offsetof(struct llhd_inst, type),
		.dispose_fn = insert_dispose,
		.substitute_fn = insert_substitute,
		.unlink_from_parent_fn = unlink_from_parent,
		.unlink_uses_fn = insert_unlink_uses,
	},
	.num_uses = 2,
	.uses_offset = offsetof(struct llhd_insert_inst, uses),
};

static struct llhd_inst_vtbl vtbl_reg_inst = {
	.super = {
		.kind = LLHD_INST_REG,
		.name_offset = offsetof(struct llhd_inst, name),
		.type_offset = offsetof(struct llhd_inst, type),
		.dispose_fn = reg_dispose,
		.substitute_fn = reg_substitute,
		.unlink_from_parent_fn = unlink_from_parent,
		.unlink_uses_fn = reg_unlink_uses,
	},
	.num_uses = 2,
	.uses_offset = offsetof(struct llhd_reg_inst, uses),
};


static const char *binary_opnames[] = {
	[LLHD_KIND_BINARY(LLHD_BINARY_ADD )] = "add",
	[LLHD_KIND_BINARY(LLHD_BINARY_SUB )] = "sub",
	[LLHD_KIND_BINARY(LLHD_BINARY_MUL )] = "mul",
	[LLHD_KIND_BINARY(LLHD_BINARY_UDIV)] = "udiv",
	[LLHD_KIND_BINARY(LLHD_BINARY_UREM)] = "urem",
	[LLHD_KIND_BINARY(LLHD_BINARY_SDIV)] = "sdiv",
	[LLHD_KIND_BINARY(LLHD_BINARY_SREM)] = "srem",
	[LLHD_KIND_BINARY(LLHD_BINARY_LSL )] = "lsl",
	[LLHD_KIND_BINARY(LLHD_BINARY_LSR )] = "lsr",
	[LLHD_KIND_BINARY(LLHD_BINARY_ASR )] = "asr",
	[LLHD_KIND_BINARY(LLHD_BINARY_AND )] = "and",
	[LLHD_KIND_BINARY(LLHD_BINARY_OR  )] = "or",
	[LLHD_KIND_BINARY(LLHD_BINARY_XOR )] = "xor",
};

static const char *compare_opnames[] = {
	[LLHD_KIND_COMPARE(LLHD_CMP_EQ )] = "eq",
	[LLHD_KIND_COMPARE(LLHD_CMP_NE )] = "ne",
	[LLHD_KIND_COMPARE(LLHD_CMP_ULT)] = "ult",
	[LLHD_KIND_COMPARE(LLHD_CMP_UGT)] = "ugt",
	[LLHD_KIND_COMPARE(LLHD_CMP_ULE)] = "ule",
	[LLHD_KIND_COMPARE(LLHD_CMP_UGE)] = "uge",
	[LLHD_KIND_COMPARE(LLHD_CMP_SLT)] = "slt",
	[LLHD_KIND_COMPARE(LLHD_CMP_SGT)] = "sgt",
	[LLHD_KIND_COMPARE(LLHD_CMP_SLE)] = "sle",
	[LLHD_KIND_COMPARE(LLHD_CMP_SGE)] = "sge",
};


static void
unlink_from_parent(void *ptr) {
	struct llhd_inst *I = ptr;
	struct llhd_value *P = I->parent;
	assert(P && P->vtbl);
	// Must go before remove_inst_fn, since that might dispose and free the
	// inst, which triggers an assert on parent == NULL in the dispose function.
	I->parent = NULL;
	if (P->vtbl->remove_inst_fn)
		P->vtbl->remove_inst_fn(P, ptr);
}


struct llhd_value *
llhd_inst_binary_new(int op, struct llhd_value *lhs, struct llhd_value *rhs, const char *name) {
	struct llhd_binary_inst *I;
	llhd_value_ref(lhs);
	llhd_value_ref(rhs);
	I = llhd_alloc_value(sizeof(*I), &vtbl_binary_inst);
	struct llhd_type *T = llhd_value_get_type(lhs);
	assert(T);
	llhd_type_ref(T);
	I->super.type = T;
	I->super.name = name ? strdup(name) : NULL;
	I->op = op;
	I->lhs = lhs;
	I->rhs = rhs;
	I->uses[0] = (struct llhd_value_use){ .user = (struct llhd_value*)I, .arg = 0 };
	I->uses[1] = (struct llhd_value_use){ .user = (struct llhd_value*)I, .arg = 1 };
	llhd_value_use(lhs, &I->uses[0]);
	llhd_value_use(rhs, &I->uses[1]);
	return (struct llhd_value *)I;
}

static void
binary_dispose(void *ptr) {
	struct llhd_binary_inst *I = ptr;
	assert(!I->super.parent);
	llhd_value_unuse(&I->uses[0]);
	llhd_value_unuse(&I->uses[1]);
	llhd_value_unref(I->lhs);
	llhd_value_unref(I->rhs);
	llhd_type_unref(I->super.type);
	llhd_free(I->super.name);
}

static void
binary_substitute(void *ptr, void *ref, void *sub) {
	struct llhd_binary_inst *I = ptr;
	if (I->lhs == ref && I->lhs != sub) {
		llhd_value_ref(sub);
		llhd_value_unuse(&I->uses[0]);
		llhd_value_use(sub, &I->uses[0]);
		llhd_value_unref(I->lhs);
		I->lhs = sub;
	}
	if (I->rhs == ref && I->rhs != sub) {
		llhd_value_ref(sub);
		llhd_value_unuse(&I->uses[1]);
		llhd_value_use(sub, &I->uses[1]);
		llhd_value_unref(I->rhs);
		I->rhs = sub;
	}
}

int
llhd_inst_binary_get_op(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_INST_BINARY));
	struct llhd_binary_inst *I = (void*)V;
	return I->op;
}

const char *
llhd_inst_binary_get_opname(struct llhd_value *V) {
	return binary_opnames[LLHD_KIND_BINARY(llhd_inst_binary_get_op(V))];
}

struct llhd_value *
llhd_inst_binary_get_lhs(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_INST_BINARY));
	struct llhd_binary_inst *I = (void*)V;
	return I->lhs;
}

struct llhd_value *
llhd_inst_binary_get_rhs(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_INST_BINARY));
	struct llhd_binary_inst *I = (void*)V;
	return I->rhs;
}


bool
llhd_inst_is(struct llhd_value *V, int kind) {
	return llhd_value_is(V, kind);
}

int
llhd_inst_get_kind(struct llhd_value *V) {
	return llhd_value_get_kind(V);
}

void
llhd_inst_append_to(struct llhd_value *V, struct llhd_value *to) {
	assert(llhd_value_is(V, LLHD_VALUE_INST));
	struct llhd_inst *I = (void*)V;
	assert(!I->parent);
	assert(to && to->vtbl && to->vtbl->add_inst_fn);
	I->parent = to;
	to->vtbl->add_inst_fn(to,V,1);
}

void
llhd_inst_prepend_to(struct llhd_value *V, struct llhd_value *to) {
	assert(llhd_value_is(V, LLHD_VALUE_INST));
	struct llhd_inst *I = (void*)V;
	assert(!I->parent);
	assert(to && to->vtbl && to->vtbl->add_inst_fn);
	I->parent = to;
	to->vtbl->add_inst_fn(to,V,0);
}

struct llhd_value *
llhd_inst_next(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_VALUE_INST));
	struct llhd_inst *I = (void*)V;
	if (!I->parent)
		return NULL;
	if (llhd_value_is(I->parent, LLHD_VALUE_BLOCK)) {
		if (llhd_block_get_last_inst(I->parent) == V)
			return NULL;
	} else {
		if (llhd_entity_get_last_inst(I->parent) == V)
			return NULL;
	}
	return (struct llhd_value*)llhd_container_of(I->link.next,I,link);
}

struct llhd_value *
llhd_inst_prev(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_VALUE_INST));
	struct llhd_inst *I = (void*)V;
	if (llhd_entity_get_first_inst(I->parent) == V)
		return NULL;
	return (struct llhd_value*)llhd_container_of(I->link.prev,I,link);
}

static void
binary_unlink_uses(void *ptr) {
	struct llhd_binary_inst *I = (struct llhd_binary_inst*)ptr;
	llhd_value_unuse(&I->uses[0]);
	llhd_value_unuse(&I->uses[1]);
}

struct llhd_value *
llhd_inst_sig_new(struct llhd_type *T, const char *name) {
	struct llhd_inst *I;
	I = llhd_alloc_value(sizeof(*I), &vtbl_sig_inst);
	assert(T);
	llhd_type_ref(T);
	I->type = T;
	I->name = name ? strdup(name) : NULL;
	return (struct llhd_value *)I;
}

static void
signal_dispose(void *ptr) {
	struct llhd_inst *I = ptr;
	assert(ptr);
	assert(!I->parent);
	llhd_type_unref(I->type);
	if (I->name)
		llhd_free(I->name);
}

struct llhd_value *
llhd_inst_compare_new(int op, struct llhd_value *lhs, struct llhd_value *rhs, const char *name) {
	struct llhd_compare_inst *I;
	llhd_value_ref(lhs);
	llhd_value_ref(rhs);
	I = llhd_alloc_value(sizeof(*I), &vtbl_compare_inst);
	I->super.type = llhd_type_new_int(1);
	I->super.name = name ? strdup(name) : NULL;
	I->op = op;
	I->lhs = lhs;
	I->rhs = rhs;
	I->uses[0] = (struct llhd_value_use){ .user = (struct llhd_value*)I, .arg = 0 };
	I->uses[1] = (struct llhd_value_use){ .user = (struct llhd_value*)I, .arg = 1 };
	llhd_value_use(lhs, &I->uses[0]);
	llhd_value_use(rhs, &I->uses[1]);
	return (struct llhd_value*)I;
}

static void *
compare_copy(void *ptr) {
	struct llhd_compare_inst *I = ptr;
	return llhd_inst_compare_new(I->op, I->lhs, I->rhs, I->super.name);
}

static void
compare_dispose(void *ptr) {
	struct llhd_compare_inst *I = ptr;
	assert(!I->super.parent);
	llhd_value_unref(I->lhs);
	llhd_value_unref(I->rhs);
	llhd_type_unref(I->super.type);
	llhd_free(I->super.name);
}

static void
compare_substitute(void *ptr, void *ref, void *sub) {
	struct llhd_compare_inst *I = ptr;
	if (I->lhs == ref && I->lhs != sub) {
		llhd_value_ref(sub);
		llhd_value_unuse(&I->uses[0]);
		llhd_value_use(sub, &I->uses[0]);
		llhd_value_unref(I->lhs);
		I->lhs = sub;
	}
	if (I->rhs == ref && I->rhs != sub) {
		llhd_value_ref(sub);
		llhd_value_unuse(&I->uses[1]);
		llhd_value_use(sub, &I->uses[1]);
		llhd_value_unref(I->rhs);
		I->rhs = sub;
	}
}

static void
compare_unlink_uses(void *ptr) {
	struct llhd_compare_inst *I = ptr;
	llhd_value_unuse(&I->uses[0]);
	llhd_value_unuse(&I->uses[1]);
}

struct llhd_value *
llhd_inst_branch_new_cond(struct llhd_value *cond, struct llhd_value *dst1, struct llhd_value *dst0) {
	struct llhd_branch_inst *I;
	assert(cond && dst1 && dst0);
	assert(llhd_value_is(dst1, LLHD_VALUE_BLOCK));
	assert(llhd_value_is(dst0, LLHD_VALUE_BLOCK));
	llhd_value_ref(cond);
	llhd_value_ref(dst1);
	llhd_value_ref(dst0);
	I = llhd_alloc_value(sizeof(*I), &vtbl_branch_inst);
	I->super.type = llhd_type_new_void();
	I->cond = cond;
	I->dst1 = (struct llhd_block *)dst1;
	I->dst0 = (struct llhd_block *)dst0;
	I->uses[0] = (struct llhd_value_use){ .user = (struct llhd_value*)I, .arg = 0 };
	I->uses[1] = (struct llhd_value_use){ .user = (struct llhd_value*)I, .arg = 1 };
	I->uses[2] = (struct llhd_value_use){ .user = (struct llhd_value*)I, .arg = 2 };
	llhd_value_use(cond, &I->uses[0]);
	llhd_value_use(dst1, &I->uses[1]);
	llhd_value_use(dst0, &I->uses[2]);
	return (struct llhd_value *)I;
}

struct llhd_value *
llhd_inst_branch_new_uncond(struct llhd_value *dst) {
	/// @todo Implement.
	assert(0 && "Not implemented");
	return NULL;
}

static void
branch_dispose(void *ptr) {
	struct llhd_branch_inst *I = ptr;
	assert(!I->super.parent);
	llhd_value_unref(I->cond);
	llhd_value_unref((struct llhd_value*)I->dst1);
	llhd_value_unref((struct llhd_value*)I->dst0);
	llhd_type_unref(I->super.type);
	llhd_free(I->super.name);
}

static void
branch_substitute(void *ptr, void *ref, void *sub) {
	struct llhd_branch_inst *I = ptr;
	if (I->cond == ref && I->cond != sub) {
		llhd_value_ref(sub);
		llhd_value_unuse(&I->uses[0]);
		llhd_value_use(sub, &I->uses[0]);
		llhd_value_unref(I->cond);
		I->cond = sub;
	}
	if (I->dst1 == ref && I->dst1 != sub) {
		llhd_value_ref(sub);
		llhd_value_unuse(&I->uses[1]);
		llhd_value_use(sub, &I->uses[1]);
		llhd_value_unref((struct llhd_value*)I->dst1);
		I->dst1 = sub;
	}
	if (I->dst0 == ref && I->dst0 != sub) {
		llhd_value_ref(sub);
		llhd_value_unuse(&I->uses[2]);
		llhd_value_use(sub, &I->uses[2]);
		llhd_value_unref((struct llhd_value*)I->dst0);
		I->dst0 = sub;
	}
}

static void
branch_unlink_uses(void *ptr) {
	struct llhd_branch_inst *I = ptr;
	llhd_value_unuse(&I->uses[0]);
	llhd_value_unuse(&I->uses[1]);
	llhd_value_unuse(&I->uses[2]);
}


struct llhd_value *
llhd_inst_branch_get_condition(struct llhd_value *V) {
	struct llhd_branch_inst *I = (void*)V;
	assert(llhd_value_is(V, LLHD_INST_BRANCH));
	return I->cond;
}

struct llhd_value *
llhd_inst_branch_get_dst(struct llhd_value *V) {
	struct llhd_branch_inst *I = (void*)V;
	assert(llhd_value_is(V, LLHD_INST_BRANCH));
	return (struct llhd_value *)I->dst0;
}

struct llhd_value *
llhd_inst_branch_get_dst0(struct llhd_value *V) {
	struct llhd_branch_inst *I = (void*)V;
	assert(llhd_value_is(V, LLHD_INST_BRANCH));
	return (struct llhd_value *)I->dst0;
}

struct llhd_value *
llhd_inst_branch_get_dst1(struct llhd_value *V) {
	struct llhd_branch_inst *I = (void*)V;
	assert(llhd_value_is(V, LLHD_INST_BRANCH));
	return (struct llhd_value *)I->dst1;
}

int
llhd_inst_compare_get_op(struct llhd_value *V) {
	struct llhd_compare_inst *I = (void*)V;
	assert(llhd_value_is(V, LLHD_INST_COMPARE));
	return I->op;
}

const char *
llhd_inst_compare_get_opname(struct llhd_value *V) {
	struct llhd_compare_inst *I = (void*)V;
	assert(llhd_value_is(V, LLHD_INST_COMPARE));
	return compare_opnames[LLHD_KIND_COMPARE(I->op)];
}

struct llhd_value *
llhd_inst_compare_get_lhs(struct llhd_value *V) {
	struct llhd_compare_inst *I = (void*)V;
	assert(llhd_value_is(V, LLHD_INST_COMPARE));
	return I->lhs;
}

struct llhd_value *
llhd_inst_compare_get_rhs(struct llhd_value *V) {
	struct llhd_compare_inst *I = (void*)V;
	assert(llhd_value_is(V, LLHD_INST_COMPARE));
	return I->rhs;
}

struct llhd_value *
llhd_inst_drive_new(struct llhd_value *sig, struct llhd_value *val) {
	struct llhd_drive_inst *I;
	assert(sig && val);
	llhd_value_ref(sig);
	llhd_value_ref(val);
	I = llhd_alloc_value(sizeof(*I), &vtbl_drive_inst);
	I->sig = sig;
	I->val = val;
	I->uses[0] = (struct llhd_value_use){ .user = (struct llhd_value*)I, .arg = 0 };
	I->uses[1] = (struct llhd_value_use){ .user = (struct llhd_value*)I, .arg = 1 };
	llhd_value_use(sig, &I->uses[0]);
	llhd_value_use(val, &I->uses[1]);
	return (struct llhd_value *)I;
}

static void
drive_dispose(void *ptr) {
	struct llhd_drive_inst *I = ptr;
	assert(!I->super.parent);
	llhd_value_unref(I->sig);
	llhd_value_unref(I->val);
}

static void
drive_substitute(void *ptr, void *ref, void *sub) {
	struct llhd_drive_inst *I = ptr;
	if (I->sig == ref && I->sig != sub) {
		llhd_value_ref(sub);
		llhd_value_unuse(&I->uses[0]);
		llhd_value_use(sub, &I->uses[0]);
		llhd_value_unref(I->sig);
		I->sig = sub;
	}
	if (I->val == ref && I->val != sub) {
		llhd_value_ref(sub);
		llhd_value_unuse(&I->uses[1]);
		llhd_value_use(sub, &I->uses[1]);
		llhd_value_unref(I->val);
		I->val = sub;
	}
}

static void
drive_unlink_uses(void *ptr) {
	struct llhd_drive_inst *I = ptr;
	llhd_value_unuse(&I->uses[0]);
	llhd_value_unuse(&I->uses[1]);
}

struct llhd_value *
llhd_inst_drive_get_sig(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_INST_DRIVE));
	struct llhd_drive_inst *I = (void*)V;
	return I->sig;
}

struct llhd_value *
llhd_inst_drive_get_val(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_INST_DRIVE));
	struct llhd_drive_inst *I = (void*)V;
	return I->val;
}

struct llhd_value *
llhd_inst_ret_new() {
	struct llhd_ret_inst *I;
	I = llhd_alloc_value(sizeof(*I), &vtbl_ret_inst);
	return (struct llhd_value *)I;
}

struct llhd_value *
llhd_inst_ret_new_one(struct llhd_value *arg) {
	return llhd_inst_ret_new_many(&arg, 1);
}

struct llhd_value *
llhd_inst_ret_new_many(struct llhd_value **args, unsigned num_args) {
	struct llhd_ret_inst *I;
	void *ptr;
	unsigned i;
	assert(num_args == 0 || args);
	ptr = llhd_alloc_value(sizeof(*I) + num_args * (sizeof(struct llhd_value*) + sizeof(struct llhd_value_use)), &vtbl_ret_inst);
	I = ptr;
	I->num_args = num_args;
	if (num_args > 0) {
		I->args = ptr + sizeof(*I);
		I->uses = ptr + sizeof(*I) + num_args * sizeof(struct llhd_value*);
		for (i = 0; i < num_args; ++i) {
			I->args[i] = args[i];
			I->uses[i] = (struct llhd_value_use){ .user = (struct llhd_value*)I, .arg = i };
			llhd_value_ref(args[i]);
			llhd_value_use(args[i], &I->uses[i]);
		}
	}
	return (struct llhd_value *)I;
}

unsigned
llhd_inst_ret_get_num_args(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_INST_RET));
	struct llhd_ret_inst *I = (void*)V;
	return I->num_args;
}

struct llhd_value *
llhd_inst_ret_get_arg(struct llhd_value *V, unsigned idx) {
	assert(llhd_value_is(V, LLHD_INST_RET));
	struct llhd_ret_inst *I = (void*)V;
	assert(idx < I->num_args);
	return I->args[idx];
}

static void
ret_dispose(void *ptr) {
	unsigned i;
	struct llhd_ret_inst *I = ptr;
	assert(!I->super.parent);
	for (i = 0; i < I->num_args; ++i) {
		llhd_value_unref(I->args[i]);
	}
}

static void
ret_substitute(void *ptr, void *ref, void *sub) {
	unsigned i;
	struct llhd_ret_inst *I = ptr;
	for (i = 0; i < I->num_args; ++i) {
		if (I->args[i] == ref && I->args[i] != sub) {
			I->args[i] = sub;
			llhd_value_ref(sub);
			llhd_value_unuse(&I->uses[i]);
			llhd_value_use(sub, &I->uses[i]);
			llhd_value_unref(ref);
		}
	}
}

static void
ret_unlink_uses(void *ptr) {
	unsigned i;
	struct llhd_ret_inst *I = ptr;
	for (i = 0; i < I->num_args; ++i) {
		llhd_value_unuse(&I->uses[i]);
	}
}

struct llhd_value *
llhd_inst_instance_new(
	struct llhd_value *comp,
	struct llhd_value **inputs,
	unsigned num_inputs,
	struct llhd_value **outputs,
	unsigned num_outputs,
	const char *name
) {
	unsigned i;
	struct llhd_inst_inst *I;
	void *ptr;
	size_t sz_uses, sz_in, sz_out;
	assert(comp);
	assert(num_inputs == 0 || inputs);
	assert(num_outputs == 0 || outputs);
	sz_uses = sizeof(struct llhd_value_use) * (1+num_inputs+num_outputs);
	sz_in   = sizeof(struct llhd_value *) * num_inputs;
	sz_out  = sizeof(struct llhd_value *) * num_outputs;
	ptr = llhd_alloc_value(sizeof(*I) + sz_uses + sz_in + sz_out, &vtbl_inst_inst);
	I = ptr;
	I->super.name = name ? strdup(name) : NULL;
	I->comp = comp;
	I->num_inputs = num_inputs;
	I->num_outputs = num_outputs;
	I->uses = ptr + sizeof(*I);
	I->params = ptr + sizeof(*I) + sz_uses;
	memcpy(I->params, inputs, sz_in);
	memcpy(I->params+num_inputs, outputs, sz_out);
	I->uses[0] = (struct llhd_value_use){ .user = (struct llhd_value*)I, .arg = 0 };
	llhd_value_ref(comp);
	llhd_value_use(comp, &I->uses[0]);
	for (i = 0; i < num_inputs+num_outputs; ++i) {
		I->uses[1+i] = (struct llhd_value_use){ .user = (struct llhd_value*)I, .arg = 1+i };
		llhd_value_ref(I->params[i]);
		llhd_value_use(I->params[i], &I->uses[1+i]);
	}
	return (struct llhd_value *)I;
}

static void
inst_dispose(void *ptr) {
	unsigned i;
	struct llhd_inst_inst *I = ptr;
	assert(ptr);
	assert(!I->super.parent);
	llhd_value_unref(I->comp);
	for (i = 0; i < I->num_inputs + I->num_outputs; ++i)
		llhd_value_unref(I->params[i]);
	llhd_free(I->super.name);
}

static void
inst_substitute(void *ptr, void *ref, void *sub) {
	unsigned i;
	struct llhd_inst_inst *I = ptr;
	assert(ptr);
	if (I->comp == ref && I->comp != sub) {
		I->comp = sub;
		llhd_value_ref(sub);
		llhd_value_unuse(&I->uses[0]);
		llhd_value_use(sub, &I->uses[0]);
		llhd_value_unref(ref);
	}
	for (i = 0; i < I->num_inputs + I->num_outputs; ++i) {
		if (I->params[i] == ref && I->params[i] != sub) {
			I->params[i] = sub;
			llhd_value_ref(sub);
			llhd_value_unuse(&I->uses[1+i]);
			llhd_value_use(sub, &I->uses[1+i]);
			llhd_value_unref(ref);
		}
	}
}

static void
inst_unlink_uses(void *ptr) {
	unsigned i;
	struct llhd_inst_inst *I = ptr;
	assert(ptr);
	llhd_value_unuse(&I->uses[0]);
	for (i = 0; i < I->num_inputs + I->num_outputs; ++i) {
		llhd_value_unuse(&I->uses[1+i]);
	}
}

struct llhd_value *
llhd_inst_inst_get_comp(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_INST_INST));
	struct llhd_inst_inst *I = (void*)V;
	return I->comp;
}

unsigned
llhd_inst_inst_get_num_inputs(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_INST_INST));
	struct llhd_inst_inst *I = (void*)V;
	return I->num_inputs;
}

unsigned
llhd_inst_inst_get_num_outputs(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_INST_INST));
	struct llhd_inst_inst *I = (void*)V;
	return I->num_outputs;
}

struct llhd_value *
llhd_inst_inst_get_input(struct llhd_value *V, unsigned idx) {
	assert(llhd_value_is(V, LLHD_INST_INST));
	struct llhd_inst_inst *I = (void*)V;
	assert(idx < I->num_inputs);
	return I->params[idx];
}

struct llhd_value *
llhd_inst_inst_get_output(struct llhd_value *V, unsigned idx) {
	assert(llhd_value_is(V, LLHD_INST_INST));
	struct llhd_inst_inst *I = (void*)V;
	assert(idx < I->num_outputs);
	return I->params[I->num_inputs+idx];
}

struct llhd_value *
llhd_inst_get_parent(struct llhd_value *V) {
	struct llhd_inst *I = (void*)V;
	assert(llhd_value_is(V, LLHD_VALUE_INST));
	return I->parent;
}

struct llhd_value *
llhd_inst_unary_new(int op, struct llhd_value *arg, const char *name) {
	struct llhd_unary_inst *I;
	struct llhd_type *T;
	assert(arg);
	llhd_value_ref(arg);
	T = llhd_value_get_type(arg);
	llhd_type_ref(T);
	I = llhd_alloc_value(sizeof(*I), &vtbl_unary_inst);
	I->super.name = name ? strdup(name) : NULL;
	I->super.type = T;
	I->op = op;
	I->arg = arg;
	llhd_value_use(arg, &I->use);
	return (struct llhd_value *)I;
}

static void
unary_dispose(void *ptr) {
	struct llhd_unary_inst *I = ptr;
	assert(!I->super.parent);
	llhd_value_unuse(&I->use);
	llhd_value_unref(I->arg);
	llhd_type_unref(I->super.type);
	llhd_free(I->super.name);
}

static void
unary_substitute(void *ptr, void *ref, void *sub) {
	struct llhd_unary_inst *I = ptr;
	if (I->arg == ref && I->arg != sub) {
		llhd_value_ref(sub);
		llhd_value_unuse(&I->use);
		llhd_value_use(sub, &I->use);
		llhd_value_unref(I->arg);
		I->arg = sub;
	}
}

static void
unary_unlink_uses(void *ptr) {
	struct llhd_unary_inst *I = ptr;
	llhd_value_unuse(&I->use);
}

int
llhd_inst_unary_get_op(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_INST_UNARY));
	struct llhd_unary_inst *I = (void*)V;
	return I->op;
}

struct llhd_value *
llhd_inst_unary_get_arg(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_INST_UNARY));
	struct llhd_unary_inst *I = (void*)V;
	return I->arg;
}

unsigned
llhd_inst_get_num_params(llhd_value_t V) {
	struct llhd_inst_vtbl *vtbl;
	assert(llhd_value_is(V, LLHD_VALUE_INST));
	vtbl = (void*)V->vtbl;
	return vtbl->num_uses;
}

llhd_value_t
llhd_inst_get_param(llhd_value_t V, unsigned idx) {
	struct llhd_inst_vtbl *vtbl;
	struct llhd_value_use *uses;
	assert(llhd_value_is(V, LLHD_VALUE_INST));
	vtbl = (void*)V->vtbl;
	assert(vtbl->num_uses > 0);
	assert(idx < vtbl->num_uses);
	uses = (void*)V + vtbl->uses_offset;
	return uses[idx].value;
}

struct llhd_value *
llhd_inst_call_new(struct llhd_value *func, struct llhd_value **args, unsigned num_args, const char *name) {
	struct llhd_call_inst *I;
	void *ptr;
	size_t sz_args;
	unsigned i, num_outputs;
	llhd_type_t func_type, T, *types;
	assert(func);
	assert(num_args == 0 || args);

	func_type = llhd_value_get_type(func);
	assert(func_type);
	num_outputs = llhd_type_get_num_outputs(func_type);
	types = llhd_zalloc(num_outputs * sizeof(llhd_type_t));
	for (i = 0; i < num_outputs; ++i) {
		types[i] = llhd_type_get_output(func_type, i);
	}
	T = llhd_type_new_struct(types, num_outputs);
	llhd_free(types);

	sz_args = num_args * sizeof(struct llhd_value *);
	ptr = llhd_alloc_value(sizeof(*I) + sz_args + (1+num_args)*sizeof(struct llhd_value_use), &vtbl_call_inst);
	I = ptr;
	I->args = ptr + sizeof(*I);
	I->uses = ptr + sizeof(*I) + sz_args;
	I->num_args = num_args;

	I->super.name = name ? strdup(name) : NULL;
	I->super.type = T;
	I->func = func;
	I->uses[0] = (struct llhd_value_use){ .user = (struct llhd_value*)I, .arg = 0 };
	llhd_value_ref(func);
	llhd_value_use(func, &I->uses[0]);

	for (i = 0; i < num_args; ++i) {
		I->args[i] = args[i];
		I->uses[1+i] = (struct llhd_value_use){ .user = (struct llhd_value*)I, .arg = 1+i };
		llhd_value_ref(args[i]);
		llhd_value_use(I->args[i], &I->uses[1+i]);
	}

	return (struct llhd_value *)I;
}

static void
call_dispose(void *ptr) {
	unsigned i;
	struct llhd_call_inst *I = ptr;
	assert(ptr);
	assert(!I->super.parent);
	llhd_value_unref(I->func);
	for (i = 0; i < I->num_args; ++i)
		llhd_value_unref(I->args[i]);
	llhd_type_unref(I->super.type);
	llhd_free(I->super.name);
}

static void
call_substitute(void *ptr, void *ref, void *sub) {
	unsigned i;
	struct llhd_call_inst *I = ptr;
	assert(ptr);
	if (I->func == ref && I->func != sub) {
		I->func = sub;
		llhd_value_ref(sub);
		llhd_value_unuse(&I->uses[0]);
		llhd_value_use(sub, &I->uses[0]);
		llhd_value_unref(ref);
	}
	for (i = 0; i < I->num_args; ++i) {
		if (I->args[i] == ref && I->args[i] != sub) {
			I->args[i] = sub;
			llhd_value_ref(sub);
			llhd_value_unuse(&I->uses[1+i]);
			llhd_value_use(sub, &I->uses[1+i]);
			llhd_value_unref(ref);
		}
	}
}

static void
call_unlink_uses(void *ptr) {
	unsigned i;
	struct llhd_call_inst *I = ptr;
	assert(ptr);
	llhd_value_unuse(&I->uses[0]);
	for (i = 0; i < I->num_args; ++i) {
		llhd_value_unuse(&I->uses[1+i]);
	}
}

struct llhd_value *
llhd_inst_call_get_func(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_INST_CALL));
	struct llhd_call_inst *I = (void*)V;
	return I->func;
}

unsigned
llhd_inst_call_get_num_args(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_INST_CALL));
	struct llhd_call_inst *I = (void*)V;
	return I->num_args;
}

struct llhd_value *
llhd_inst_call_get_arg(struct llhd_value *V, unsigned idx) {
	assert(llhd_value_is(V, LLHD_INST_CALL));
	struct llhd_call_inst *I = (void*)V;
	assert(idx < I->num_args);
	return I->args[idx];
}


struct llhd_value *
llhd_inst_extract_new(struct llhd_value *target, unsigned index, const char *name) {
	struct llhd_extract_inst *I;
	struct llhd_type *T;
	assert(target);
	T = llhd_value_get_type(target);
	assert(T);
	if (llhd_type_is(T, LLHD_TYPE_STRUCT)) {
		assert(index < llhd_type_get_num_fields(T));
		T = llhd_type_get_field(T, index);
	} else {
		T = llhd_type_get_subtype(T);
	}
	I = llhd_alloc_value(sizeof(*I), &vtbl_extract_inst);
	I->super.type = T;
	I->super.name = name ? strdup(name) : NULL;
	I->target = target;
	I->index = index;
	I->use = (struct llhd_value_use){ .user = (void*)I, .arg = 0 };
	llhd_type_ref(T);
	llhd_value_ref(target);
	llhd_value_use(target, &I->use);
	return (void*)I;
}


static void
extract_dispose(void *ptr) {
	struct llhd_extract_inst *I = ptr;
	assert(!I->super.parent);
	llhd_value_unref(I->target);
	llhd_type_unref(I->super.type);
	llhd_free(I->super.name);
}

static void
extract_substitute(void *ptr, void *ref, void *sub) {
	struct llhd_extract_inst *I = ptr;
	if (I->target == ref && I->target != sub) {
		llhd_value_ref(sub);
		llhd_value_unuse(&I->use);
		llhd_value_use(sub, &I->use);
		llhd_value_unref(I->target);
		I->target = sub;
	}
}

static void
extract_unlink_uses(void *ptr) {
	struct llhd_extract_inst *I = ptr;
	llhd_value_unuse(&I->use);
}

unsigned
llhd_inst_extract_get_index(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_INST_EXTRACT));
	struct llhd_extract_inst *I = (void*)V;
	return I->index;
}

struct llhd_value *
llhd_inst_extract_get_target(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_INST_EXTRACT));
	struct llhd_extract_inst *I = (void*)V;
	return I->target;
}


struct llhd_value *
llhd_inst_insert_new(struct llhd_value *target, unsigned index, struct llhd_value *value, const char *name) {
	struct llhd_insert_inst *I;
	struct llhd_type *T;
	assert(target);
	T = llhd_value_get_type(target);
	assert(index < llhd_type_get_num_fields(T));
	// assert(llhd_type_equal(llhd_type_get_field(T, index), llhd_value_get_type(value)));
	I = llhd_alloc_value(sizeof(*I), &vtbl_insert_inst);
	I->super.type = T;
	I->super.name = name ? strdup(name) : NULL;
	I->target = target;
	I->index = index;
	I->value = value;
	I->uses[0] = (struct llhd_value_use){ .user = (void*)I, .arg = 0 };
	I->uses[1] = (struct llhd_value_use){ .user = (void*)I, .arg = 1 };
	llhd_type_ref(T);
	llhd_value_ref(target);
	llhd_value_ref(value);
	llhd_value_use(target, &I->uses[0]);
	llhd_value_use(value, &I->uses[1]);
	return (void*)I;
}

static void
insert_dispose(void *ptr) {
	struct llhd_insert_inst *I = ptr;
	assert(!I->super.parent);
	llhd_value_unref(I->target);
	llhd_value_unref(I->value);
	llhd_type_unref(I->super.type);
	llhd_free(I->super.name);
}

static void
insert_substitute(void *ptr, void *ref, void *sub) {
	struct llhd_insert_inst *I = ptr;
	if (I->target == ref && I->target != sub) {
		llhd_value_ref(sub);
		llhd_value_unuse(&I->uses[0]);
		llhd_value_use(sub, &I->uses[0]);
		llhd_value_unref(I->target);
		I->target = sub;
	}
	if (I->value == ref && I->value != sub) {
		llhd_value_ref(sub);
		llhd_value_unuse(&I->uses[1]);
		llhd_value_use(sub, &I->uses[1]);
		llhd_value_unref(I->value);
		I->value = sub;
	}
}

static void
insert_unlink_uses(void *ptr) {
	struct llhd_insert_inst *I = ptr;
	llhd_value_unuse(&I->uses[0]);
	llhd_value_unuse(&I->uses[1]);
}

unsigned
llhd_inst_insert_get_index(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_INST_INSERT));
	struct llhd_insert_inst *I = (void*)V;
	return I->index;
}

struct llhd_value *
llhd_inst_insert_get_target(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_INST_INSERT));
	struct llhd_insert_inst *I = (void*)V;
	return I->target;
}

struct llhd_value *
llhd_inst_insert_get_value(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_INST_INSERT));
	struct llhd_insert_inst *I = (void*)V;
	return I->value;
}


struct llhd_value *
llhd_inst_reg_new(struct llhd_value *value, struct llhd_value *strobe, const char *name) {
	struct llhd_reg_inst *I;
	struct llhd_type *T, *Ts;

	assert(value && strobe);
	T = llhd_value_get_type(value);
	Ts = llhd_value_get_type(strobe);
	assert(T && Ts);
	assert(llhd_type_is(Ts, LLHD_TYPE_INT) && llhd_type_get_length(Ts) == 1); /// @todo Make this more elegant

	I = llhd_alloc_value(sizeof(*I), &vtbl_reg_inst);
	I->super.name = name ? strdup(name) : NULL;
	I->super.type = T;
	I->value = value;
	I->strobe = strobe;
	I->uses[0] = (struct llhd_value_use){ .user = (void*)I, .arg = 0 };
	I->uses[1] = (struct llhd_value_use){ .user = (void*)I, .arg = 1 };
	llhd_type_ref(T);
	llhd_value_ref(value);
	llhd_value_ref(strobe);
	llhd_value_use(value, &I->uses[0]);
	llhd_value_use(strobe, &I->uses[1]);

	return (void*)I;
}

static void
reg_dispose(void *ptr) {
	struct llhd_reg_inst *I = ptr;
	assert(!I->super.parent);
	llhd_value_unref(I->value);
	llhd_value_unref(I->strobe);
	llhd_type_unref(I->super.type);
	llhd_free(I->super.name);
}

static void
reg_substitute(void *ptr, void *ref, void *sub) {
	struct llhd_reg_inst *I = ptr;
	if (I->value == ref && I->value != sub) {
		llhd_value_ref(sub);
		llhd_value_unuse(&I->uses[0]);
		llhd_value_use(sub, &I->uses[0]);
		llhd_value_unref(I->value);
		I->value = sub;
	}
	if (I->strobe == ref && I->strobe != sub) {
		llhd_value_ref(sub);
		llhd_value_unuse(&I->uses[1]);
		llhd_value_use(sub, &I->uses[1]);
		llhd_value_unref(I->strobe);
		I->strobe = sub;
	}
}

static void
reg_unlink_uses(void *ptr) {
	struct llhd_reg_inst *I = ptr;
	llhd_value_unuse(&I->uses[0]);
	llhd_value_unuse(&I->uses[1]);
}

struct llhd_value *
llhd_inst_reg_get_value(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_INST_REG));
	struct llhd_reg_inst *I = (void*)V;
	return I->value;
}

struct llhd_value *
llhd_inst_reg_get_strobe(struct llhd_value *V) {
	assert(llhd_value_is(V, LLHD_INST_REG));
	struct llhd_reg_inst *I = (void*)V;
	return I->strobe;
}
