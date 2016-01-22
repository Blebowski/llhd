/* Copyright (c) 2016 Fabian Schuiki */
#include "llhd/ir/constant.hpp"
#include "llhd/ir/constants.hpp"
#include "llhd/ir/types.hpp"

namespace llhd {

Constant * Constant::getNullValue(Type * type) {
	llhd_assert(type);
	switch (type->getTypeId()) {
		case Type::LogicTypeId:
			return ConstantLogic::getNullValue(dynamic_cast<LogicType*>(type));
		default:
			llhd_abort_msg("No corresponding null value for type");
			return nullptr;
	}
}

Constant::Constant(Type * type):
	Value(type) {
}

} // namespace llhd
