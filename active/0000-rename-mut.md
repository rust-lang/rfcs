- Start Date: 2014-04-30
- RFC PR #: 
- Rust Issue #: 

# Summary

`&mut` implies that `&` is not mutable, which is not accurate due to types that
implement internal mutability. So, rename `&mut` to `&only` since that better
reflects the semantics of that reference type - that it is non-aliased. Then,
allow borrowing of a non-mut value to an `&only` reference.

# Motivation

One of the most basic rules of Rust is that memory may only be mutated via
non-aliased pointers. A `&mut` reference is a reference that is guaranteed at
compile time to be non-aliased. As such, memory may be mutated via a `&mut`
reference. A `&mut` reference is not the only way to mutate memory, however. As
long as it can be dynamically guanteed that a `&` reference is non-aliased, it
is valid to mutate memory via other pointer types, such as a `&`. The `Cell`
type is an example of this.

A `mut` slot may be mutated directly or may be borrowed to a `&mut` and mutated
through that reference. A non-mut slot may not be reborrowed to a `&mut`.
However, it may be mutated if the type implements internal mutability, such as
with the `Cell` type.

The problem is that `&mut` references and `mut` slots imply that `&` refernces
and non-mut slots are immutable which is not the case. This caues quite a bit
of confusion with newcomers and medium experienced Rust users who are trying to
understand `&` references and non-mut slots as equivalent to C++ const
references. At the end of the day, `&` and `&mut` references are both mutable
and both mut slots and non-mut slots are also both mutable. Rust has no
equivalent to C++'s `const`.

So, the goal is to use terminology that is more descriptive of the sematics
that the types guarantee. Since the most significant guantee provided by `&mut`
pointers is that they are non-aliased, I propose renaming `&mut` to `&only`
since I believe that does a better job of describing the guarantee. The
language semantics of the type remain unchanged. However, I believe it becomes
significantly clearer that the ability to mutate memory through an `&only`
reference arises out of Rust's rules regarding aliasing rather than out of
`&mut` being the mutable version of `&`. I propose that the `mut` keyword on
slots remain the same and still indicate that the memory may be directly
mutated. However, I proposed that it become legal to borrow an `&only` refernce
from a non-mut slot as long as the value is not currently aliased.

# Drawbacks

This would be a massive, massive change. It also would require an extra
character for non-aliased references.

There may be other issues I haven't considered as I am not an expert in Rust's
borrowing rules.

# Detailed design

Rename `&mut` to `&only`. Allow borrowing of a non-mut slot to an `&only`
reference.

# Alternatives

Leaving things as they are.

# Unresolved questions

* Is `&only` the best name for a non-aliased reference type?
* Is it confusing that a non-mut slot can be borrowed to a `&only` reference
  and then mutated?

