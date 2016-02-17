/* Copyright (c) 2016 Fabian Schuiki */
#include "llhd/ir/instructions.hpp"
#include "llhd/ir/basicblock.hpp"
#include "llhd/ir/type.hpp"

namespace llhd {

DriveInst::DriveInst(Value * target, Value * value):
	Instruction(Instruction::Drive, Type::getVoidType(target->getContext())),
	target(target),
	value(value) {
	llhd_assert_msg(equal(target->getType(), value->getType()), "target and value must be of the same type");
}

BranchInst::BranchInst(BasicBlock * ifTrue, BasicBlock * ifFalse, Value * cond):
	Instruction(Instruction::Branch, Type::getVoidType(ifTrue->getContext())),
	ifTrue(ifTrue),
	ifFalse(ifFalse),
	condition(cond) {
	llhd_assert(ifTrue);
	if (cond) {
		llhd_assert(cond->getType()->isInteger(1));
		llhd_assert(ifFalse);
	}
}

SwitchInst::SwitchInst(Value * value, BasicBlock * otherwise):
	Instruction(Instruction::Switch, Type::getVoidType(value->getContext())),
	value(value),
	otherwise(otherwise) {
}

void SwitchInst::addDestination(Value * val, BasicBlock * dst) {
	llhd_assert(val);
	llhd_assert(dst);
	destinations.push_back(Destination(val,dst));
}

BinaryInst::BinaryInst(Opcode opc, Value * lhs, Value * rhs):
	Instruction(opc, lhs->getType()),
	lhs(lhs),
	rhs(rhs) {
	llhd_assert_msg(equal(lhs->getType(), rhs->getType()), "lhs and rhs of binary op must be of same type");
}

static Type * getExtractValueType(Type * type, unsigned length) {
	switch (type->getTypeId()) {
		case Type::LogicTypeId:
			return Type::getLogicType(type->getContext(), length);
		default:
			llhd_abort_msg("extract value not supported for type");
	}
}

ExtractValueInst::ExtractValueInst(Value * target, Value * index, unsigned length):
	Instruction(Instruction::ExtractValue, getExtractValueType(target->getType(), length)),
	target(target),
	index(index),
	length(length) {
	llhd_assert(index->getType()->getTypeId() == Type::IntegerTypeId);
}

InsertValueInst::InsertValueInst(Value * target, Value * value, Value * index, unsigned length):
	Instruction(Instruction::InsertValue, target->getType()),
	target(target),
	value(value),
	index(index),
	length(length) {
	llhd_assert(index->getType()->getTypeId() == Type::IntegerTypeId);
}

CompareInst::CompareInst(Op op, Value * lhs, Value * rhs):
	Instruction(Instruction::Compare, Type::getIntegerType(lhs->getContext(),1)),
	op(op),
	lhs(lhs),
	rhs(rhs) {
	llhd_assert(lhs);
	llhd_assert(rhs);
}

} // namespace llhd
