- Start Date: 2014-06-13
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Overloaded arithmetic and logical operators should take `self` and their arguments by value.

# Motivation

Expensive objects that support arithmetic and logical operations—bignums, primarily—would benefit from not having to copy whenever arithmetic is used. This particular case was one of the motivations for rvalue references in C++, in fact. One can work around it with `RefCell`, but it's a pain and introduces dynamic failures. The easiest way to fix this is to make the traits that define arithmetic and operators take `self` and any arguments by value.

# Detailed design

The declarations of the traits `Add`, `Sub`, `Mul`, `Div`, `Rem`, `Neg`, `Not`, `BitAnd`, `BitOr`, `BitXor`, `Shl`, and `Shr` change to:

    pub trait Add<RHS,Result> {
        fn add(self, rhs: RHS) -> Result;
    }

    pub trait Sub<RHS,Result> {
        fn sub(self, rhs: RHS) -> Result;
    }

    pub trait Mul<RHS,Result> {
        fn mul(self, rhs: RHS) -> Result;
    }

    pub trait Div<RHS,Result> {
        fn div(self, rhs: RHS) -> Result;
    }

    pub trait Rem<RHS,Result> {
        fn rem(self, rhs: RHS) -> Result;
    }

    pub trait Neg<Result> {
        fn neg(self) -> Result;
    }

    pub trait Not<Result> {
        fn not(self) -> Result;
    }

    pub trait BitAnd<RHS,Result> {
        fn bitand(self, rhs: RHS) -> Result;
    }

    pub trait BitOr<RHS,Result> {
        fn bitor(self, rhs: RHS) -> Result;
    }

    pub trait BitXor<RHS,Result> {
        fn bitxor(self, rhs: RHS) -> Result;
    }

    pub trait Shl<RHS,Result> {
        fn shl(self, rhs: RHS) -> Result;
    }

    pub trait Shr<RHS,Result> {
        fn shr(self, rhs: RHS) -> Result;
    }

The `AutorefArgs` stuff in `typeck` will be removed; all overloaded operators will typecheck as though they were `DontAutorefArgs`.

# Drawbacks

Some use cases of `+`, such as string and array concatentation, may become more verbose. It is likely that many of the use cases of `+` today will be compatible with these new semantics.

# Alternatives

As an alternative, each of these operators could have two methods, one for by-reference and one for by-value. This adds complexity, however.

Not doing this will mean that the issues in "Motivation" will remain unsolved.

# Unresolved questions

None.
