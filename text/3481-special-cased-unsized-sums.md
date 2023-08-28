- Feature Name: special_cased_unsized_sums
- Start Date: 2023-08-28
- RFC PR: [rust-lang/rfcs#3481](https://github.com/rust-lang/rfcs/pull/3481)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Currently `Iterator::sum` does a relatively slow iterative sum using 
`Iterator::fold` internally. For sequences of positive integers, the sum of the sequence 
has been proven down to a simple formula that computes much faster than an iterative sum.
This RFC suggests a special-case implementation of `Iterator::sum` for `Range<u*>` and 
`RangeInclusive<u*>` (where `u*` is `u8`, `u16`, `u32`, `u64`, `u128`, and `usize`). 

The base formula to be used is available on Wikipedia [here](https://en.wikipedia.org/wiki/1_%2B_2_%2B_3_%2B_4_%2B_%E2%8B%AF).

# Motivation
[motivation]: #motivation

This should signifigantly improve the performance of any code that uses some variant of 
`(0..n).sum()`. This may be somewhat uncommon, but is still worth the performance increase for 
the small number of cases. 

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

For any consecutive sequence of positive integers from 1 to n inclusive (`1..=n`), the sum will 
equal `n*(n+1)/2`. This also works for ranges from 0 to n (inclusive; `0..=n`) because 
`sum(0..=n) == 0 + sum(1..=n)`. We can generalize this to work for any range by simply 
subtracting the lower sum from the upper sum. For example, to do `sum(5..=10)` we would do 
`sum(1..=10) - sum(1..=4)`: `10*11/2 - 4*5/2 == 55-10 == 45 == 5+6+7+8+9+10 == sum(5..=10)`. 

In this way, any sum over a `RangeInclusive<u*>`: `range.start..=range.end` can be implemented 
with `range.end*(range.end+1)/2 - range.start*(range.start-1)/2`. 

The same solution can be extended to `Range<u*>` by turning it into a `RangeInclusive<u*>` by 
decrementing `range.end` by 1.

Rust programmers should largely not have to think about this feature. `Iterator::sum` in most 
cases should continue to function as normal. In the few cases that `(m..=n).sum()` is used, where 
`m` and `n` are unsigned integers, there should be a noticable performance boost for large 
ranges. 

`Iterator::sum` for ranges will continue to error out in the same places as previously. Any 
overflows on addition may be replaced with an overflow on multiplication, but this will be 
implemented carefully to avoid introducing any new overflow conditions. 

Rust code will look identical to a reader before and after this change. 

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This will be implemented by separating the `Range<u*>` and `RangeInclusive<u*>` from the existing
`Step` based `Iterator` implementation and implementing them manually (perhaps with help from a 
macro to ensure uniform implementation accross different unsized types). Unsized integer types 
will still implement `Step` but will not use it in their implementation of 
`Iterator for Range(Inclusive)<u*>`.

To prevent the intermediate values used in calculating the formula from overflowing, it may be 
useful to cast up to a 'larger' unsized integer type and then cast back down when the calculation
is complete. This works for all unsigned integer types except `u128`. This was suggested by Rust 
Internals user [pitaj](https://internals.rust-lang.org/u/pitaj) in 
[this comment](https://internals.rust-lang.org/t/special-case-unsigned-integer-range-iterator-sum/19436/5?u=alfriadox). 

User [toc](https://internals.rust-lang.org/u/toc) on the Rust Internals forum also suggested a 
solution to prevent overflow in advance by doing the division first: 
```
pub fn sum_to(n: u64) -> u64 {
    let odd: u64 = (n % 2 == 1).into();
    ((n + (!odd)) / 2) * (n + odd)
}
```
This is perhaps preferable to me, to avoid type casts and needing to special-case `u128`. 

# Drawbacks
[drawbacks]: #drawbacks

This may cause a loss in performance for extremely small ranges (<5 integers). This may still be 
faster even in those cases because we avoid calling `Iterator::fold`. 

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This design offers an improvement from `O(n)` to `O(1)` in applicable uses of `Iterator::sum`. 
This makes it entirely preferable over the current implementation, even if there is a slight
loss in performance for extremely small ranges. I do not expect this change to have any noticable 
impact on the vast majority of Rust programs, but those that it does impact will almost certainly
see a performance improvement. 

# Prior art
[prior-art]: #prior-art

- [Rust Internals thread](https://internals.rust-lang.org/t/special-case-unsigned-integer-range-iterator-sum/19436/8). 

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- [reference-level-explanation]: Solution to mitigate possible overflow of intermediate values.
- Number of use-cases where the possible change from adding-overflow to multiplication-overflow matters (suspected to be none). 

# Future possibilities
[future-possibilities]: #future-possibilities

Some possibilities that are out of the scope of this RFC but may be considered for Rust 
eventually include other performance improvements made possible by formulas such as this 
one. 
