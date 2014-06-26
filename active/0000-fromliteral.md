- Start Date: 2014-06-26
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Create new traits, FromStringLiteral, FromVectorLiteral<T>. Types implementing these traits can be automatically constructed from literals automatically. Literals will effectively have their actual types inferred statically.

# Motivation

The biggest usecase for this will be that creating a String from a literal will no longer require calling .to_string(). It also eliminates the need for the vec! macro. In general, it saves typing and adds substantial extensibility to the language.

Before, this compiles:

let x = "Hello, world!".to_string(); 

while this doesn't:
let x: String = "Hello, world!";

If this is implemented, both would compile.

# Detailed design

trait FromStringLiteral {
	fn from_string_literal(lit: &'static str) -> Self;
}
trait FromVectorLiteral<T> {
	fn from_vector_literal(lit: &[T]) -> Self;
}

A string or vector literal would be considered a "generic string" or "generic vector", similar to generic ints. All types implementing FromStringLiteral/FromVectorLiteral would be considered subtypes of generic strings/vectors (respectively); if the compiler encounters a literal being used where a type implementing one of these traits is expected, the compiler generates code to call the trait's from_string_literal/from_vector_literal method with a reference to a string or vector containing the literal data.

Although from_string_literal takes a reference with static lifetime, from_vector_literal does not constrain the lifetime. This is because vector literals may have elements constructed dynamically.

# Drawbacks
Complexity is added to the compiler, as it must infer the type of literals.

Integral / floating point literals are not included in this, and would behave as they do now. This may be confusing; if it is, traits could be added to include all literals.

A type could potentially implement FromStringLiteral/FromVectorLiteral in an expensive manner, surprising the user. Programmers should be told not to do this.

Wrapper types around strings/vectors which carry semantic information in their type could implement FromStringLiteral/FromVectorLiteral, thus allowing one to "accidentally" construct a type which might represent "safe" unsanitized data, etc. Programmers should be told not to implement FromStringLiteral/FromVectorLiteral on types where accidental construction is a concern.

# Alternatives

Not implementing this leaves nasty calls to .to_string/.to_owned littered around code. This gets annoying quickly.

A full C++-style constructor system could be added, allowing types to be secretly constructed from any other type. This is overly general and very prone to mistakes.

Literals could be treated as String/Vec instead of &'static str and fixed-size vectors. This ties the language far too closely to the standard library, and should be avoided.

Add generic literal types to the language officially (not just in the compiler), and create a single FromLiteral<T: GenericLiteral> type or reuse FromPrimitive. 

# Unresolved questions

As always, naming is bikesheddable.

Is there enough of a use-case for a FromIntegerLiteral or FromFloatLiteral? 