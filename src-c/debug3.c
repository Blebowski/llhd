// Copyright (c) 2016 Fabian Schuiki
#include <llhd.h>
#include <stdint.h>
#include <assert.h>
#include <stdio.h>
#include <string.h>


int main() {
	llhd_value_t E, P, I, Q, BBentry, BBckl, BBckh, k0;
	llhd_type_t Ety, Pty, i1ty;

	i1ty = llhd_type_new_int(1);

	Ety = llhd_type_new_comp((llhd_type_t[]){i1ty,i1ty}, 2, (llhd_type_t[]){i1ty}, 1);
	E = llhd_entity_new(Ety, "LAGCE");
	llhd_value_set_name(llhd_unit_get_input(E,0), "CK");
	llhd_value_set_name(llhd_unit_get_input(E,1), "E");
	llhd_value_set_name(llhd_unit_get_output(E,0), "GCK");
	llhd_type_unref(Ety);

	Pty = llhd_type_new_comp((llhd_type_t[]){i1ty,i1ty,i1ty}, 3, (llhd_type_t[]){i1ty,i1ty}, 2);
	P = llhd_proc_new(Pty, "LAGCE_proc");
	llhd_value_set_name(llhd_unit_get_input(P,0), "CK");
	llhd_value_set_name(llhd_unit_get_input(P,1), "E");
	llhd_value_set_name(llhd_unit_get_input(P,2), "Q");
	llhd_value_set_name(llhd_unit_get_output(P,0), "GCK");
	llhd_value_set_name(llhd_unit_get_output(P,1), "Q");
	llhd_type_unref(Pty);

	Q = llhd_inst_sig_new(i1ty, "Q");
	llhd_inst_append_to(Q, E);
	llhd_value_unref(Q);
	I = llhd_inst_instance_new(P,
		(llhd_value_t[]){
			llhd_unit_get_input(E,0),
			llhd_unit_get_input(E,1),
			Q
		}, 3,
		(llhd_value_t[]){
			llhd_unit_get_output(E,0),
			Q
		}, 2,
		"p"
	);
	llhd_inst_append_to(I, E);
	llhd_value_unref(I);

	BBentry = llhd_block_new("entry");
	BBckl = llhd_block_new("ckl");
	BBckh = llhd_block_new("ckh");
	llhd_block_append_to(BBentry,P);
	llhd_block_append_to(BBckl,P);
	llhd_block_append_to(BBckh,P);
	llhd_value_unref(BBentry);
	llhd_value_unref(BBckl);
	llhd_value_unref(BBckh);

	k0 = llhd_const_int_new(0);
	I = llhd_inst_compare_new(LLHD_CMP_EQ, llhd_unit_get_input(P,0), k0, NULL);
	llhd_value_unref(k0);
	llhd_inst_append_to(I, BBentry);
	llhd_value_unref(I);
	I = llhd_inst_branch_new_cond(I, BBckl, BBckh);
	llhd_inst_append_to(I, BBentry);
	llhd_value_unref(I);

	I = llhd_inst_drive_new(llhd_unit_get_output(P,1), llhd_unit_get_input(P,1));
	llhd_inst_append_to(I, BBckl);
	llhd_value_unref(I);
	k0 = llhd_const_int_new(0);
	I = llhd_inst_drive_new(llhd_unit_get_output(P,0), k0);
	llhd_value_unref(k0);
	llhd_inst_append_to(I, BBckl);
	llhd_value_unref(I);
	I = llhd_inst_ret_new();
	llhd_inst_append_to(I, BBckl);
	llhd_value_unref(I);

	I = llhd_inst_drive_new(llhd_unit_get_output(P,0), llhd_unit_get_input(P,2));
	llhd_inst_append_to(I, BBckh);
	llhd_value_unref(I);
	I = llhd_inst_ret_new();
	llhd_inst_append_to(I, BBckh);
	llhd_value_unref(I);

	llhd_asm_write_unit(E, stdout);
	llhd_asm_write_unit(P, stdout);
	llhd_value_unref(E);
	llhd_value_unref(P);

	llhd_type_unref(i1ty);

	return 0;
}
