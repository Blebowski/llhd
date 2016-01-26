/* Copyright (c) 2016 Fabian Schuiki */
#include "llhd/ir/context.hpp"
#include "llhd/ir/constants.hpp"

namespace llhd {

ConstantLogic * ConstantLogic::getNull(LogicType * type) {
	llhd_assert(type);
	auto * C = new ConstantLogic(type, Logic(type->getWidth(), Logic::O));
	type->getContext().values.push_back(C);
	return C;
}

ConstantLogic * ConstantLogic::get(LogicType * type, const std::string & str) {
	llhd_assert(type);
	llhd_assert(type->getWidth() == str.size());
	auto * C = new ConstantLogic(type, Logic(str));
	type->getContext().values.push_back(C);
	return C;
}

ConstantLogic::ConstantLogic(LogicType * type, const Logic & value):
	Constant(type),
	value(value) {
	llhd_assert(type);
	llhd_assert(type->getWidth() == value.getWidth());
}

ConstantInteger * ConstantInteger::getNull(IntegerType * type) {
	llhd_assert(type);

	auto * C = new ConstantInteger(type);
	type->getContext().values.push_back(C);
	C->value = 0;

	return C;
}

ConstantInteger * ConstantInteger::get(IntegerType * type, const std::string & str) {
	llhd_assert(type);

	auto * C = new ConstantInteger(type);
	type->getContext().values.push_back(C);
	C->value = std::stoll(str);

	return C;
}

ConstantInteger * ConstantInteger::get(IntegerType * type, std::intmax_t value) {
	llhd_assert(type);

	auto * C = new ConstantInteger(type);
	type->getContext().values.push_back(C);
	C->value = value;

	return C;
}

ConstantInteger::ConstantInteger(IntegerType * type):
	Constant(type) {
	llhd_assert(type);
}

} // namespace llhd
