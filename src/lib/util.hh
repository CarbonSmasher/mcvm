#pragma once
#include "print.hh"

#include <rapidjson/document.h>

#include <iostream>
#include <vector>

// Error checking

// Return if a condition is true
#define GUARD(condition) if (condition) return
// Return if a condition is false
#define ENSURE(condition) GUARD(!(condition))
// Assert that this statement will never be run
#define ASSERT_NOREACH() assert(false)

// Memory utilities

// Delete pointed to elements of a vector but do not delete the elements themselves
#define DEL_VECTOR(vec) for (uint __i = 0; __i < vec.size(); __i++) { delete vec[__i]; }
// Delete pointed to elements of a map but do not delete the elements themselves
#define DEL_MAP(map) for (auto __i = map.begin(); __i != map.end(); __i++) { delete __i->second; }
// Delete an object with a nullptr assertion
#define ASSERTED_DEL(obj) (assert(obj != nullptr); delete obj)
// Delete an object with a nullptr check
#define PROTECTED_DEL(obj) (if (obj != nullptr) delete obj)

// Attributes

#define UNUSED [[maybe_unused]]
#define FALLTHROUGH [[fallthrough]]

// Nice numbers

#define CHARBUF_SMALL 256
#define CHARBUF_MEDIUM 4096
#define CHARBUF_LARGE 65536

namespace json = rapidjson;

namespace mcvm {
	// Compute the length of a string literal at compile time
	// https://stackoverflow.com/a/26082447
	template <std::size_t N>
	constexpr std::size_t litlen(const char[N]) {
		return N - 1;
	}
	// Finds and replaces first occurrence of a string in another string and replaces it with something else
	// Will modify source
	static inline void fandr(std::string& source, const std::string& find, const std::string_view repl) {
		std::size_t pos = source.find(find);
		if (pos == std::string::npos) return;
		source.replace(pos, find.length(), repl);
	}

	// Obtain a subvector of a parent vector
	template <typename T>
	static inline std::vector<T> vec_slice(const std::vector<T>& src, std::size_t start, std::size_t len) {
		auto first = src.cbegin() + start;
		auto second = first + len;
		return std::vector<T>(first, second);
	}
};
