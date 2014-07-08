/* Copyright (c) 2014 Fabian Schuiki */
#pragma once
#include "llhd/SourceLocation.hpp"

namespace llhd {

class SourceBuffer;
class TokenContext;

namespace vhdl {

class Lexer {
	TokenContext& ctx;
public:
	bool skipWhitespaces;
	bool skipComments;

	Lexer(TokenContext& ctx): ctx(ctx) {
		skipWhitespaces = true;
		skipComments = true;
	}
	void lex(const SourceBuffer& src, SourceLocation loc);
};

} // namespace vhdl
} // namespace llhd
