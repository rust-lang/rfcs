- Feature Name: `extended_hrtb`
- Start Date: 2022-05-08
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/3621)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This feature add a way to bound universal quantification of higher ranked trait bounds (HRTB) to allow more APIs.

# Motivation
[motivation]: #motivation

The HRTB construct has allowed us to talk about closures that are generic over lifetimes. However, initially this has not included a way to restrict these generic lifetimes, thus, in practice, it turned out to be useful in only a handful of scenarios.

Currently, the existing higher-rank trait bounds functionality allows generalizing bounds that outlive a given lifetime, but do not allow generalizing bounds that are outlived by other lifetimes, the goal of the proposal is to allow such generalizations.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The new syntax looks like this:\
`for<'a where 'b: 'a>` - it's called extended HRTB\
This construct reads as "for all lifetimes that `'b` outlives".

This form of HRTB allow you to make the following interface:\
(The example is taken from Sabrina's post, see Prior Art)
```rust
fn print_items<'i,I>(mut iter: I)
where
	I: LendingIterator + 'i,
	for<'a where 'i: 'a> I::Item<'a>: Debug,
    //this means that for every lifetime that doesn't outlive the iterator, `Item: Debug` is true.
{
	while let Some(item) = iter.next() {
		println!("{item:?}");
	}
}
```

This function is otherwise very limited as it can only take `'static` data - see prior art.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The difference between extended HRTB and what we'd have with plain form is that:
If we use the plain HRTB, then inference would require the associated lifetime of `I` in the example to be outlive all lifetimes <=> be `'static`.

Instead, with this feature, we add the bound restricting the part "all lifetimes" to "lifetimes lesser than `'i`", so the inference no longer requires the associated lifetime of `I` to be `'static` but just `'i`.

Syntax:
```rust
for<$list_of_lifetimes where $list_of_lifetime_bounds>
```

Any bound put inside of `$list_of_lifetime_bounds` have to refer to at least one lifetime introduced within `$list_of_lifetimes`. This way, we keep bounds of parent context separate from bounds of extended HRTB.

### Subtyping

In eHRTB, we have two lifetimes: lesser and greater bounds.

Variance is the following:
* eHRTB constructs are covariant in greater
* and contravariant in lesser bounds.

For example:

1. `for<'a: 'l where 'r1: 'a> T` is a sub type of `for<'a: 'l where 'r2: 'a> T` if `'r1: 'r2`;
2. `for<'a: 'l> T` is a sub type of `for<'a: 'r> T` if `'l: 'r`.

# Drawbacks
[drawbacks]: #drawbacks

We may not do this if we want another form for this capability.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There were other variants of syntax:
```
for<'a> where<'i: 'a> ...

for<'a> where 'i: 'a ...

for<'a; 'i: 'a> ... //the original one
```
They were not chosen because of:
1. Result of poll;
2. Most likely ambiguos;
3. Less clear.

# Prior art
[prior-art]: #prior-art

[Internals thread](https://internals.rust-lang.org/t/extending-for-a-construct/16581) - prior discussion and bikesheding.

[Sabrina's recent post](https://sabrinajewson.org/blog/the-better-alternative-to-lifetime-gats#where-real-gats-fall-short)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None

# Future possibilities
[future-possibilities]: #future-possibilities

I don't believe we want HRTB to carry even more than proposed here, at least for now.
