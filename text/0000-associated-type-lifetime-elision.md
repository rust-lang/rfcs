- Feature Name: associate_type_lifetime_elision
- Start Date: 2018-09-21
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC extends lifetime elision to associated types, treating associated
types as output positions.

This is particularly helpful for the usual `IntoIterator` pattern:

```rust
impl<T> IntoIterator for &Container<T> {
    type Item = &T;
    type IntoIter = Iter<'_, T>;
    fn into_iter(self) -> Self::IntoIter { self.iter() }
}
```

No `'a`s needed!

# Motivation
[motivation]: #motivation

[RFC #141]: https://github.com/rust-lang/rfcs/blob/master/text/0141-lifetime-elision.md
[RFC #195]: https://github.com/rust-lang/rfcs/blob/master/text/0195-associated-items.md

The current lifetime elision system comes from [RFC #141], which was merged way
back on 2014-07-09.  At that time associated types didn't exist.  For example,
the `Add` trait used an extra type parameter for the output type:

```rust
pub trait Add<RHS,Result> {
    /// The method for the `+` operator
    fn add(&self, rhs: &RHS) -> Result;
}
```

Associated types came slightly later, in [RFC #195], merged 2014-09-16.

While that later RFC didn't update the elision rules, it does talk about the
roles in trait matching of the two kinds of types:

> This RFC clarifies trait matching by:
>
> - Treating all trait type parameters as input types, and
> - Providing associated types, which are output types.

Conveniently, this input/output distinction aligns perfectly with the
input/output position definitions used by elision.

In current elision, function parameters are inputs and return types are outputs:

```rust
fn foo(input: InputType) -> OutputType { ... }
```

This same pattern plays out in trait implementations, where trait parameters
(including the `Self` parameter) are input positions and associated types
are outputs:

```rust
impl Foo for InputType {
    type Output = OutputType
}
```

This is particularly helpful when it comes time to implement `IntoIterator` for
one's type.  For example, here's some code from `core` today:

```rust
impl<'a, T> IntoIterator for &'a Option<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Option<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> IterMut<'a, T> {
        self.iter_mut()
    }
}
```

This lifetime elision extension allows us to completely avoid naming `'a`:

```rust
impl<T> IntoIterator for &Option<T> {
    type Item = &T;
    type IntoIter = Iter<'_, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T> IntoIterator for &mut Option<T> {
    type Item = &mut T;
    type IntoIter = IterMut<'_, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
```

There's just the single input-position lifetime, which the associated types will
automatically now use.

(Note how this is exactly the same elision you currently get in
`fn(&Option<T>)->(&T, Iter<'_, T>)` and `fn(&mut Option<T>)->(&mut T, IterMut<'_, T>)`.)

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Suppose you've just implemented a cool new container class, `SkipList<T>`,
complete with `.iter()` and `.iter_mut()` methods.  To support `for x in &my_list`
and `for x in &mut my_list`, like the standard containers do, you should implement
the `IntoIterator` trait for shared and mutable borrows of your container.

Written out fully-explicitly, that looks like this:

```rust
impl<'a, T> IntoIterator for &'a SkipList<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>; // the same type you made for `.iter()`
    fn into_iter(self) -> Self::IntoIter { self.iter() }
}
```

But like how you probably just wrote your `.iter()` method without `'a`s

```rust
impl<T> SkipList<T> {
    fn iter(&self) -> Iter<'_, T> { ... }
}
```

You can also elide the lifetimes in the trait implementation:

```rust
impl<T> IntoIterator for &SkipList<T> {
    type Item = &T;
    type IntoIter = Iter<'_, T>; // the same type you made for `.iter()`
    fn into_iter(self) -> Self::IntoIter { self.iter() }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

[existing rules]: https://github.com/rust-lang/rfcs/blob/master/text/0141-lifetime-elision.md#the-rules

This RFC only applies the [existing rules] in a new location.  It is explicitly
a non-goal to add new rules.  As such, the behaviour of any example can always
be understood by translation to the analogous `fn` situation.

That said, this RFC does not consider the `Self` position in a trait
implementation to fall under the "`&self` or `&mut self`" position extra rule.
It's not obvious that the trait case has the same "typically borrowing from
`self`, not the other parameters" special case that methods do.  (If further
experience demonstrates that this would be valuable, it can compatibly be added
later, as it only enables things that this RFC leaves as errors.)

Like in function return position, it's important to know when an associated type
is using lifetime elision, so elided lifetime parameters must be marked with
`'_`, such as `type Output = Ref<'_, T>;`.  Omitting a lifetime parameter, such
as `type Output = Ref<T>;`, is an error.

Examples:

```rust
impl Substr<u32> for &str { type Output = &str; }               // elided
fn substr(s: &str, until: u32) -> &str;                         // fn analog
impl<'a> Substr<u32> for &'a str { type Output = &'a str; }     // expanded

impl Finder<&str> for u32 { type Output = &str; }               // elided
fn finder(x: u32, y: &str) -> &str;                             // fn analog
impl<'a> Finder<&'a str> for u32 { type Output = &'a str; }     // elided

impl GetStr for u32 { type Output = &str; }                     // ILLEGAL
fn get_str(x: u32) -> &str;                                     // fn analog
// No input position

impl Frob<&str> for &str { type Output = &str; }                // ILLEGAL
fn frob(x: &str, y: &str) -> &str;                              // fn analog
// `Self` isn't special

impl GetMut for &mut Foo { type Output = &mut Bar; }            // elided
fn get_mut(x: &mut Foo) -> &mut Bar;                            // fn analog
impl<'a> GetMut for &'a mut Foo { type Output = &'a mut Bar; }  // expanded

impl New for &mut [u8] { type Output = BufWriter<'_>; }         // elided
fn new(buf: &mut [u8]) -> BufWriter<'_>;                        // fn analog
impl<'a> New for &'a mut [u8] { type Output = BufWriter<'a>; }  // expanded

impl New for &mut [u8] { type Output = BufWriter; }             // ILLEGAL
// Hidden lifetime parameter

impl Two for &str { type A = &str; type B = &str; }             // elided
fn two(x: &str) -> (&str, &str);                                // fn analog
impl<'a> Two for &'a str { type A = &'a str; type B = &'a str; }// expanded
```

# Drawbacks
[drawbacks]: #drawbacks

As this is an extension of elision, it shares the same drawbacks: there's no
lifetime name to mention in error messages, it delays a rigorous understanding
of lifetimes, the rules need to be understood to comprehend a signature, etc.
The trade-off has proven worth-while for `fn`, however, and re-using the same
rules here should keep the learning cost low.

This does make associated types somewhat special.  Notably, the input positions
from the impl header don't apply to `fn` return type outputs, so something like
the following doesn't work, though one might think it should:

```rust
impl Foo<&str> for u32 {
    fn bar(self) -> &str; // ERROR: missing lifetime specifier
    // help: this function's return type contains a borrowed value with an
    // elided lifetime, but the lifetime cannot be derived from the arguments.
}
```

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This RFC is inspired by comments on the in-band lifetimes tracking issue, such
as [this one](https://github.com/rust-lang/rust/issues/44524#issuecomment-408456816):

> TL;DR: We should extend lifetime elision rather than elide lifetime declaration.

It's possible that a different scheme could be better here.  But the input/output
position meanings and corresponding rules fit very well, so it'd have to be much
better to overcome the cost of having another ruleset to learn.

This proposal covers almost all of the associated types with lifetimes in libcore.
There are lots of `IntoIterator` cases, like we've already seen, as well as a
number of `Iterator` cases, such as these:

```rust
impl Iterator for Utf8LossyChunksIter<'_> {
    type Item = Utf8LossyChunk<'_>;
    ...
}
```

```rust
impl Iterator for Lines<'_> {
    type Item = &str;
    ...
}
```

There are a bunch of cases in str/pattern.rs that don't elide, however.  Some
involve multiple lifetimes, so may never elide:

```rust
impl<'a, 'b> Pattern<'a> for &'b str {
    type Searcher = StrSearcher<'a, 'b>;
    ...
}
```

But even some simpler ones don't:

```rust
impl<'a> Pattern<'a> for char {
    type Searcher = CharSearcher<'a>;
    fn into_searcher(self, haystack: &'a str) -> Self::Searcher { ... }
    ...
}
```

(That example is like `fn pattern<'a>(x: char) -> CharSearcher<'a>;`, which
doesn't elide under the existing rules, as there's no lifetime in input position.
It also cannot use `'_` in the impl header anyway, as the lifetime is used
in a method parameter later.)

Some additional examples from rustc, after applying this:

```rust
impl super::ForestObligation for &str {
    type Predicate = &str;
    ...
}
```
```rust
impl Lift for PlaceElem<'_> {
    type Abstract = AbstractElem<'_>;
    ...
}
```
```rust
impl DepTrackingMapConfig for TraitSelectionCache<'_> {
    type Key = (ty::ParamEnv<'_>, ty::PolyTraitRef<'_>);
    type Value = Vtable<'_, ()>;
    ...
}
```
```rust
impl Index<CanonicalVar> for CanonicalVarValues<'_> {
    type Output = Kind<'_>;
    ...
}
```
```rust
impl<Ty> Deref for TyLayout<'_, Ty> {
    type Target = &LayoutDetails;
    ...
}
```

As mentioned above, we could extend the "`&self` or `&mut self`" rule somehow to
make it apply logically to trait impls.  There are a variety of possibilities
for that.  We could special-case only `... for &Bar` and `... for &mut Bar`.  We
could also allow it for anything in that position, so `impl Foo<&str> for Bar<'_>`
would allow associated types to take their lifetimes from `Bar<'_>` despite there
being multiple lifetimes in input position.  But these possibilities are left for later.

# Prior art
[prior-art]: #prior-art

This RFC chooses to keep the existing lifetime rules (including the more recent
preferred idiom) to leverage all the research and experience from the previous
Rust work on lifetime elision.  Its author is unaware of non-Rust prior art.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

(None for now.)
