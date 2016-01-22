/* Copyright (c) 2016 Fabian Schuiki */
#pragma once
#include "llhd/common.hpp"
#include "llhd/ir/value.hpp"

namespace llhd {

class BasicBlock;

class Instruction : public Value, public OwnedBy<BasicBlock> {
public:
	Instruction(Type * type, BasicBlock * parent = nullptr);
	virtual ~Instruction();

private:
};

} // namespace llhd
