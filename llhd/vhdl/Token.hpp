/* Copyright (c) 2014 Fabian Schuiki */
#pragma once
#include "llhd/vhdl/TokenType.hpp"
#include <ostream>

namespace llhd {
namespace vhdl {

struct TokenPosition
{
	int line;
	int column;
	TokenPosition(): line(0), column(0) {}
};

struct TokenRange
{
	TokenPosition start;
	TokenPosition end;
	TokenRange() {}
	TokenRange(TokenPosition& s, TokenPosition& e): start(s), end(e) {}
};

struct Token
{
	TokenType type;
	std::string value;
	TokenRange range;
};

const char* tokenTypeToString(TokenType t);

std::ostream& operator<< (std::ostream& o, const TokenPosition& p);
std::ostream& operator<< (std::ostream& o, const TokenRange& r);
std::ostream& operator<< (std::ostream& o, const Token& tkn);

} // namespace vhdl
} // namespace llhd
