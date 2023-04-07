- Feature Name: `std::ops::Range/RangeInclusive::get_value and conversion methods`)
- Start Date: 2023-04-07
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a method to get a value from range, which is tied to the range.
Later that value may be converted into another range of values,
preserving the relative position within the new range.

# Motivation
[motivation]: #motivation

<!-- Why are we doing this? What use cases does it support? What is the expected outcome? -->

It is useful to have a range of possible values and being able to quickly
obtain a value from this range. Currently, there is no way to get a value
from a range of types `std::ops::Range` and `std::ops::RangeInclusive`;
there is only a method called `contains()` which can be called with a
proposed value to check if it lies within the range. Later, if the
value lies within the range, there is no way to tell that the value
checked actually does that within the code: additional logic is required.

A possibility for a value to be tied to a "parent" range it was got from
will allow a value-to-new-range conversion. For example, we may want to
have a thread priority value, which we may want to be "user-friendly" by
having values in the range of `[0; 100]`. Later, we may pick a value out
of this range, for example, `50`. However, on different operating systems
the thread priority ranges are different and depend on many things; in
other words, it is almost certainly not the `[0; 100]` range we wanted.
Let's assume we want to change a Linux niceness of a thread. On Linux,
the niceness values are in the range of `[-20; 19]`. A certain calculation
is required to map a value `50` from range `[0; 100]` to the range
`[-20; 19]`, to preserve the relative (middle) position, which would be
`0` in this case (`40` allowed values in total). This can be avoided
as these calculations can be all written once and just used. Such a
mechanism within an already existing type like `std::ops::Range` and/or
`std::ops::RangeInclusive` would greatly simplify this process of
mapping values from certain ranges.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

<!-- Explain the proposal as if it was already included in the language and you were teaching it to another Rust programmer. That generally means: -->

By introducing a new type called `RangeValue`, we can have a type which
declares a value tied to its parent range it was taken from:

```rust
use std::ops::Deref;

/// A Range value which is tied to the range object and its lifetime.
#[derive(Debug, Copy, Clone)]
struct RangeValue<'r, V> {
    value: V,
    range: &'r std::ops::Range<V>,
}
impl<'r, V> RangeValue<'r, V>
 {
    fn get(&self) -> &V {
        &self.value
    }

    fn range(&self) -> &std::ops::Range<V> {
        &self.range
    }
}

impl<'r, V> AsRef<V> for RangeValue<'r, V> {
    fn as_ref(&self) -> &V {
        self.get()
    }
}

impl<'r, V> Deref for RangeValue<'r, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}
```

Such a value will always be known as a value which lies within the range
and this fact will never be "forgotten" within the code, as it is can be
"promised" and ensured at compile time.

By introducing a new method called `get_value()` to both, `std::ops::Range` and
`std::ops::RangeInclusive`, it becomes possible to get a value tied to
the range it was taken from:

```rust
trait GetRangeValue<'r, V> where V: ToOwned {
    /// Returns a [`RangeValue`] if it lies within the range, otherwise,
    /// [`None`].
    ///
    /// The returned value is bound to this Range.
    fn get_value(&'r self, v: &V) -> Option<RangeValue<'r, V>>;
}

impl<'r, V> GetRangeValue<'r, V> for std::ops::Range<V> where V: ToOwned<Owned = V> + PartialEq + PartialOrd {
    fn get_value(&'r self, v: &V) -> Option<RangeValue<'r, V>> {
        if self.contains(v) {
            Some(RangeValue {
                value: v.to_owned(),
                range: &self,
            })
        } else {
            None
        }
    }
}
```

Later we introduce a method for `RangeValue` which would convert the
value from one range to another range's value:

```rust
use std::ops::{Add, Sub, Div, Mul};

impl<'r1, 'r2, V> RangeValue<'r1, V>
    where
    V: Copy + ToOwned<Owned = V> + Sub<Output = V> + Mul<Output = V> + Div<Output = V> + Add<Output = V> + PartialEq + PartialOrd,
{
    /// Convert into another range of values while preserving the relative
    /// position.
    fn convert(&'r1 self, range: &'r2 std::ops::Range<V>) -> Option<RangeValue<'r2, V>>
    {
        let out_range: V = range.end - range.start;
        let in_range: V = self.range.end - self.range.start;
        let new_possible_value: V = (self.value - self.range.start) * out_range / in_range + range.start;
        range.get_value(&new_possible_value)
    }
}

fn main() {
    // This provides a value tied to its parent range it is taken from.
    // This value is tied to its range's lifetime.
    let range = 0..10;
    let value = range.get_value(&5);
    assert_eq!(value.unwrap().get(), &5);
    assert_eq!(*value.unwrap(), 5);

    // This successfully converts a value `5` from range `0..10` to
    // the range of `0..100`, which would be equal to `50`.
    let new_range = 0..100;
    let value = value.unwrap().convert(&new_range);
    assert_eq!(value.unwrap().get(), &50);

    // This value is out of scope of the allowed values, so a `None`
    // value is returned.
    let value = new_range.get_value(&500);
    assert!(value.is_none());
}
```

The value returned from the range is guaranteed to lie within the allowed
range of values represented by the range the method is used on.

This feature would allow to:

1. Easily know whether a value is within some range or not and such fact
   will never be able to be "forgotten" in the code as the `RangeValue`
   types make sure it is bound to the parent range and this is ensured
   at compile time.
2. Easily map a value from one range to another range, preserving the
   relative position within the range.

In the end, the feature brings:

1. A new type `RangeValue` which defines a type bound to a range.
2. A new trait `GetRangeValue` implementors of which return a
`RangeValue`.
3. A new way to obtain a value from a range based on the `Option` type:
when `Some` is returned, a value returned is guaranteed to lie
within the range and it can't change. As opposed to using the
`contains()` method, this allows the developer to work with a type
having a guarantee that this value can't be changed and lies within
the scope of allowed values by the range, and this fact can't be
forgotten or abused in the code. It also brings a slightly more
convenient way of getting a value from the range. Consider a use-case
when a `Result` type is used. Now it is possible to use the `try!` macro
or the "question-mark" operator `?` to quickly exit the function when a
value doesn't lie within the range:

    ```rust
    fn set_thread_priority(priority: u8) -> Result<(), &'static str> {
        let value = (0..100)
            .get_value(&priority)
            .ok_or_else(|| "The priority doesn't lie within the user-allowed range")?;

        // The same value but mapped to the allowed values range for
        // the niceness:
        let mapped = value.convert(&(-20..20))
            .ok_or_else(|| "The priority doesn't lie within the niceness range")?;

        set_niceness_for_current_thread(*mapped)
    }
    ```

<!--
- Introducing new named concepts.
- Explaining the feature largely in terms of examples.
- Explaining how Rust programmers should *think* about the feature, and how it should impact the way they use Rust. It should explain the impact as concretely as possible.
- If applicable, provide sample error messages, deprecation warnings, or migration guidance.
- If applicable, describe the differences between teaching this to existing Rust programmers and new Rust programmers.
- Discuss how this impacts the ability to read, understand, and maintain Rust code. Code is read and modified far more often than written; will the proposed feature make code easier to maintain?

For implementation-oriented RFCs (e.g. for compiler internals), this section should focus on how compiler contributors should think about the change, and give examples of its concrete impact. For policy RFCs, this section should provide an example-driven introduction to the policy, and explain its impact in concrete terms.
-->

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.

# Drawbacks
[drawbacks]: #drawbacks

<!-- Why should we *not* do this? -->
No known and reasonable drawbacks.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Why is this design the best in the space of possible designs?

It is a simple as possible. Should also be fast enough.

- What other designs have been considered and what is the rationale for not choosing them?

For the sake of this RFC, a special trait has been provided to allow the
readers to understand how it is supposed faster, by providing a
fully-working code; for the implementation we may avoid using traits in
favour of using struct methods.

- What is the impact of not doing this?

When it comes to conversion of a value from one range to another, -
everyone who needs to perform the same operation will have to spend time
googling and calculating everything on his own, possibly doing mistakes.

When it comes to improving usability of the Range structures, this RFC
suggests a way to guarantee a certain value lies within the range by
providing a specific type, which is supposed to only be created by a
range object when a value lies within the range. By having it as a
separate type with lifetime bounds to its parent range and the ability
not only be immutable, the developer never has to guess and carefully
re-read the code to understand he did the things right.

When it comes to the interface, returning an `Option` when getting the
range value lying within the range allows to easily use the question-mark
operator `?` to greatly simplify the workflow when any compatible type
used (which implements the `std::ops::FromResidual` trait).

When converting a value from one range to another, the calculated value
should lie within the range, but it may not be when the new range to
which the mapping was done is empty. To handle this case, the `Option`
is also used the same way.


# Prior art
[prior-art]: #prior-art

Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?

I don't know that.

- For community proposals: Is this done by some other community and what were their experiences with it?

I am not aware of that.

- For other teams: What lessons can we learn from what other communities have done here?

I don't know.

- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

I am not aware of this.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged?

I suggest to get rid of the `trait GetRangeValue` used in this RFC in
favour of having `std::ops::Range` and `std::ops::RangeInclusive`
methods instead.

I also suggest to carefully think about the naming of the methods and
types used for this RFC.

- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?

All the corner-cases when it comes the value calculation: if we can
guarantee that the new range to which the mapping is done can't be empty
and is always valid, we may avoid returning `Option` from there.

- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

Don't know.

# Future possibilities
[future-possibilities]: #future-possibilities

<!-- Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how this all fits into the roadmap for the project
and of the relevant sub-team. -->

Perhaps, it makes sense to implement the `std::iter::FromIterator` trait
for the `RangeValue` type, so that it could create a new `Range` based
on the values collected: the lowest value is the start of the new range
and the highest value is the end. I am not sure how useful this is
though.

<!-- This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information. -->
