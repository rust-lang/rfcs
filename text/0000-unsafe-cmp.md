- Feature Name: unsafe_cmp
- Start Date: 2015-03-09
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Make the four `cmp` traits (`PartialEq`, `Eq`, `PartialOrd`, and `Ord`) become
`unsafe` traits.

# Motivation

Some algorithms and data structures (such as the `SliceExt::sort()` algorithm
in the standard library) depend on comparisons to be sane in order to be
efficient. In those cases, ill-behaved comparison traits might cause undefined
behavior. However, since the `cmp` traits are currently normal traits (i.e. not
`unsafe`), they cannot be trusted to be well-behaved. As a result, such
optimizations are not possible.

The proposed solution is to make the `PartialEq`, `Eq`, `PartialOrd`, and `Ord`
traits `unsafe`. This allows library to trust the trait implementations to
follow certain rules.

Some might argue that these traits do not invoke `unsafe` behavior. However,
this usage of `unsafe` is intended by design, as described in RFC 19:

> An *unsafe trait* is a trait that is unsafe to implement, because it
> represents some kind of trusted assertion. Note that unsafe traits are
> perfectly safe to *use*. `Send` and `Share` are examples of unsafe traits:
> implementing these traits is effectively an assertion that your type is safe
> for threading.

In the case of comparison traits, the "trusted assertion" is that they behave
sanely, as described in the Detailed design section.

The reason only the `cmp` traits are addressed here is because they have the
highest potential to be relied on by `unsafe` traits. (But see the Unresolved
questions section).

I believe that in practice, only a few `unsafe`s will be required, since most
types will simply `#[derive]` the required traits, in which case the
correctness can be guaranteed.

Additionally, the properties required are made more strict and rigourous in
this RFC.

# Detailed design

`#[deriving]` is not affected by this change. It should work the same as they
always did.

Mark the `PartialEq`, `Eq`, `PartialOrd`, and `Ord` traits as `unsafe` and
require implementations of these traits to satisfy the following properties:

**Note**:
- `=>` stands for "if-then". A property of the form `X => Y` means that "if `X`
    type-checks correctly, then `Y` must also do so. If `X` type-checks
    correctly and evaluates to `true`, then `Y` must also do so".
- `<=>` stands for "if and only if". A property of the form `X <=> Y` means
    that "`X` must type-check correctly if and only if `Y` does so. If they
    type-check correctly, they must evaluate to the same boolean value".
- Properties of other forms must evaluate to `true` if they type-check
    correctly.

For `PartialEq`:
- `a.eq(b) <=> b.eq(a)`
- `a.eq(b) && b.eq(c) => a.eq(c)`
- `a.eq(b) <=> !(a.ne(b))`

For `Eq`:
- `a.eq(a)`

For `PartialOrd`:
- `a.partial_cmp(b) == Some(Less) <=> a.lt(b)`
- `a.partial_cmp(b) == Some(Greater) <=> a.gt(b)`
- `a.partial_cmp(b) == Some(Equal) <=> a.eq(b)`
- `a.le(b) <=> a.lt(b) || a.eq(b)`
- `a.ge(b) <=> a.gt(b) || a.eq(b)`
- `a.lt(b) <=> b.gt(a)`
- `a.lt(b) && b.lt(c) => a.lt(c)`

For `Ord`:
- `Some(a.cmp(b)) == a.partial_cmp(b)`

# Drawbacks

- Some types might want to implement these traits such that they do not satisfy
    these properties. However, I consider this to be abuse of traits.
- Some people might just use `unsafe` without knowing the potential bad
    consequences.
- Might cause too many `unsafe`s in otherwise safe code.
- This is a breaking change.

# Alternatives

- The status quo.
- Have separate traits for trusted behavior and untrusted behavior e.g. `Eq` as
    a safe trait that is not trusted, and `EqStrict` that is an `unsafe` trait
    that can be trusted by `unsafe` code. The problem is that there is no
    obvious reason to implement `Eq` but not implement `EqStrict` (See the
    Drawbacks section).

# Unresolved questions

- Are the properties required here complete?
- Is this worth the number of extra `unsafe`s?
- What about the `Iterator`, `ExactSizeIterator`, `DoubleEndedIterator`, and
    `RandomAccessIterator` traits?
- Does this apply to other traits?
- Can the transitivity properties be enforced cross-crate?
- Do we need the type parameters in `PartialEq` and `PartialOrd`?
