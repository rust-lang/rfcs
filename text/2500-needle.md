- Feature Name: `needle`
- Start Date: 2018-07-06
- RFC PR: [rust-lang/rfcs#2500](https://github.com/rust-lang/rfcs/pull/2500)
- Rust Issue: [rust-lang/rust#56345](https://github.com/rust-lang/rust/issues/56345)

# This RFC was previously approved, but later **withdrawn**

For details see the [summary comment].

[summary comment]: https://github.com/rust-lang/rust/pull/76901#issuecomment-880169952

# Summary
[summary]: #summary

Generalize the needle (née pattern) API to support `&str`, `&mut str`, `&[T]`, `&mut [T]`, `Vec<T>` and `&OsStr`.

<!-- TOC depthTo:2 -->

- [Summary](#summary)
- [Motivation](#motivation)
    - [Stabilize the Pattern API](#stabilize-the-pattern-api)
    - [Implement OMG-WTF-8](#implement-omg-wtf-8)
- [Guide-level explanation](#guide-level-explanation)
- [Reference-level explanation](#reference-level-explanation)
    - [API](#api)
    - [Standard library changes](#standard-library-changes)
    - [Performance](#performance)
- [Drawbacks](#drawbacks)
- [Rationale and alternatives](#rationale-and-alternatives)
    - [Principles](#principles)
    - [Design rationales](#design-rationales)
    - [Miscellaneous decisions](#miscellaneous-decisions)
    - [Alternatives](#alternatives)
- [Prior art](#prior-art)
    - [Previous attempts](#previous-attempts)
    - [Haskell](#haskell)
- [Unresolved questions](#unresolved-questions)

<!-- /TOC -->

# Motivation
[motivation]: #motivation

## Stabilize the Pattern API

Pattern API v1.0 ([RFC 528] / [issue 27721]) has been implemented for nearly 3 years,
but we still haven't decided to stabilize. One of the blockers is attempting to generalize the API
to support `str`, `[T]` and `OsStr`, though it only exists as sketches and never finalized.

This RFC is raised as attempt to

1. Generalize the pattern API so that all built-in slice-like types `&str`, `&mut str`, `&[T]`,
    `&mut [T]`, `Vec<T>` and `&OsStr` can be searched.

2. Revise the API to address some performance and usability issues identified in
    the previous attempts.

We hope that this RFC could revitalize the Pattern API development and make its stabilization
foreseeable.

## Implement OMG-WTF-8

The OMG-WTF-8 encoding was introduced to allow slicing an `&OsStr`, and thus enable extending
the Pattern API to `&OsStr` without special-casing ([RFC 2295] / [issue 49802]). That RFC expects
a Pattern API working with `OsStr` to generalize some methods (e.g. `OsStr::ends_with()`).
This RFC would unblock the implementation of RFC 2295, as to decide whether to integrate with
a Pattern API, or just go with the non-generic version.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

You may check the prototype package [`pattern-3`] for API documentation and source code.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Key concepts:

* Searching is based on *trisection*, splitting a string into 3 parts: the substring before, being,
    and after the match. `"ab123cedf" == "ab" ++ "123" ++ "cdef"`.
* *Haystack* teaches the search algorithm how to perform splitting with proper ownership transfer.
* *Searcher* is responsible for finding the range of the match.
* Utilizing these together to safely construct many useful algorithms related to string matching.

## API

All items below should be placed in the `core::needle` module, re-exported as `std::needle`.

We renamed "Pattern API" into "Needle API" to avoid confusion with the language's pattern matching
i.e. the `match` expression.

### Hay

A `Hay` is the core type which the search algorithm will run on.
It is implemented on the unsized slice-like types like `str`, `OsStr` and `[T]`.

```rust
pub unsafe trait Hay {
    type Index: Copy + Debug + Eq;

    fn empty<'a>() -> &'a Self;

    fn start_index(&self) -> Self::Index;
    fn end_index(&self) -> Self::Index;

    unsafe fn next_index(&self, index: Self::Index) -> Self::Index;
    unsafe fn prev_index(&self, index: Self::Index) -> Self::Index;

    unsafe fn slice_unchecked(&self, range: Range<Self::Index>) -> &Self;
}
```

The trait is unsafe to implement because it needs to guarantee all methods (esp. `.start_index()`
and `.end_index()`) follow the documented requirements, which cannot be checked automatically.

We allow a hay to customize the `Index` type. While `str`, `[T]` and `OsStr` all  use `usize` as
the index, we do want the Needle API to support other linear structures like `LinkedList<T>`,
where a cursor/pointer would be more suitable for allowing sub-linear splitting.

```
start_index() = 0   next_index(2) = 6
        |         +-------------------+
        v         ^                   v
        0    1    2    3    4    5    6    7
        +----+----+----+----+----+----+----+
        | 48 | 69 | f0   9f   8c   8d | 21 |
        +----+----+----+----+----+----+----+
        0    1    2    3    4    5    6    7
             ^    v                        ^
             +----+                        |
        prev_index(2) = 1            end_index() = 7
```

### Haystack

A `Haystack` is any linear structure which we can do string/array matching on,
and can be sliced or split so they could be returned from the `matches()` and `split()` iterators.

Haystack is implemented on the reference or collection itself e.g. `&[T]`, `&mut [T]` and `Vec<T>`.
A hay can *borrowed* from a haystack.

```rust
pub unsafe trait Haystack: Deref<Target: Hay> + Sized {
    fn empty() -> Self;
    unsafe fn split_around(self, range: Range<Self::Target::Index>) -> [Self; 3];

    unsafe fn slice_unchecked(self, range: Range<Self::Target::Index>) -> Self {
        let [_, middle, _] = self.split_around(range);
        middle
    }

    fn restore_range(
        &self,
        original: Range<Self::Target::Index>,
        parent: Range<Self::Target::Index>,
    ) -> Range<Self::Target::Index>;
}
// we assume either RFC 2089 (issue #44491) or RFC 2289 is implemented.
// for simplicity we're ignoring issue #38078 which forces us to write `<Self::Target as Hay>::Index`.
```

> We assume either Implied Bounds ([RFC 2089] / [issue 44491]) is implemented (thus fixing
> [issue 20671]), or Associated Type Bounds ([RFC 2289]) has been accepted and implemented.
>
> For simplicity we are ignoring [issue 38078],
> which forces us to write `<Self::Target as Hay>::Index` instead of `Self::Target::Index`.

The `self.restore_range(original, parent)` method is implemented to solve:

* given haystacks `a` and `b`
* given `a = b[original]` and `self = a[parent]`
* find `range` such that `self == b[original][parent] == b[range]`

This method is used to recover the original range in functions like `find()` and `match_indices()`.
It is usually just implemented as `(original.start + parent.start)..(original.start + parent.end)`.

When an index is based on a pointer, splitting a haystack will *invalidate* those pointers.
However, a pointer is persisted with slicing, so they could implement this method simply as
`self.start_index()..self.end_index()`.

### Shared haystack

A `SharedHaystack` is a marker sub-trait which tells the compiler this haystack can cheaply be
cheaply cloned (i.e. shared), e.g. a `&H` or `Rc<H>`. Implementing this trait alters some behavior
of the `Span` structure discussed next section.

```rust
pub trait SharedHaystack: Haystack + Clone {}
```

`.restore_range()` will never be called with a shared haystack and should be implemented as
`unreachable!()`.

### Span

A `Span` is a haystack coupled with information where the original span is found.

```rust
pub struct Span<H: Haystack> { /* hidden */ }

impl<H: Haystack> Span<H> {
    pub fn original_range(&self) -> Range<H::Target::Index>;
    pub fn borrow(&self) -> Span<&H::Target>;
    pub fn is_empty(&self) -> bool;
    pub fn take(&mut self) -> Self;

    pub unsafe fn split_around(self, subrange: Range<H::Target::Index>) -> [Self; 3];
    pub unsafe fn slice_unchecked(self, subrange: Range<H::Target::Index>) -> Self;
}

impl<H: SharedHaystack> Span<H> {
    pub fn into_parts(self) -> (H, Range<H::Target::Index>);
    pub unsafe fn from_parts(haystack: H, range: Range<H::Target::Index>) -> Self;
}

impl<H: Haystack> From<H> for Span<H> { ... }
impl<H: Haystack> From<Span<H>> for H { ... }
```

The behavior of a span differs slightly between a shared haystack and unique haystack
(this is also the main reason why the `Span` structure is introduced).

```text
                     Span<&str>                         Span<&mut str>

               -+---+---+---+---+---+-               +---+---+---+---+---+
                | C | D | E | F | G |                | C | D | E | F | G |
               -+---+---+---+---+---+-               +---+---+---+---+---+
                ^___________________^                ^                   ^
                 3..8                                3                   8

split_around:           ^_______^                            ^_______^
                         5..7                                 2..4

               -+---+---+---+---+---+-               +---+---+
                | C | D | E | F | G |                | C | D |
               -+---+---+---+---+---+-               +---+---+
                ^_______^                            ^       ^
                 3..5                                3       5

               -+---+---+---+---+---+-                       +---+---+
                | C | D | E | F | G |                        | E | F |
               -+---+---+---+---+---+-                       +---+---+
                        ^_______^                            ^       ^
                         5..7                                5       7

               -+---+---+---+---+---+-                               +---+
                | C | D | E | F | G |                                | G |
               -+---+---+---+---+---+-                               +---+
                                ^___^                                ^   ^
                                 7..8                                7   8
```

A span of shared haystack will always store a copy of the original haystack when splitting,
because the haystack can be cheaply cloned. Splitting is thus just manipulation of the range only.
Slicing is only done when returning from an algorithm.

A non-shared haystack needs to maintain unique ownership for each haystack slice. Therefore,
a haystack will be split as soon as the span is split. The "original range" becomes a value
disconnected from the haystack, and this is where `.restore_range()` is needed:
to recover the indices in the middle (`5 == 3 + 2` and `7 == 3 + 4`).

### Searcher

A searcher only provides a single method: `.search()`. It takes a span as input,
and returns the first sub-range where the given needle is found.

```rust
pub unsafe trait Searcher<A: Hay + ?Sized> {
    fn search(&mut self, span: Span<&A>) -> Option<Range<A::Index>>;
}

pub unsafe trait ReverseSearcher<A: Hay + ?Sized>: Searcher<A> {
    fn rsearch(&mut self, span: Span<&A>) -> Option<Range<A::Index>>;
}

pub unsafe trait DoubleEndedSearcher<A: Hay + ?Sized>: ReverseSearcher<A> {}
```

The `.search()` function is safe because there is no safe ways to construct a `Span<&A>`
with invalid ranges. Implementations of `.search()` often start with:

```rust
    fn search(&mut self, span: Span<&A>) -> Option<Range<A::Index>> {
        let (hay, range) = span.into_parts();
        // search for needle from `hay` restricted to `range`.
    }
```

The trait is unsafe to implement because it needs to guarantee the returned range is valid.

There is a "reverse" version of the trait, which supports searching from the end
with the `.rsearch()` method besides from the start.

Furthermore, there is a "double-ended" version, which is a marker trait saying that
searching from both ends will give consistent results. The searcher of a substring needle is
an example which implements `ReverseSearcher` but not `DoubleEndedSearcher`, e.g.

* Forward searching the needle `xx` in the haystack `xxxxx` will yield `[xx][xx]x`
* Backward searching the needle `xx` in the haystack `xxxxx` will yield `x[xx][xx]`

### Consumer

A consumer provides the `.consume()` method to implement `starts_with()` and `trim_start()`. It
takes a span as input, and if the beginning matches the needle, returns the end index of the match.

```rust
pub unsafe trait Consumer<A: Hay + ?Sized> {
    fn consume(&mut self, span: Span<&A>) -> Option<A::Index>;
}

pub unsafe trait ReverseConsumer<A: Hay + ?Sized>: Consumer<A> {
    fn rconsume(&mut self, span: Span<&A>) -> Option<A::Index>;
}

pub unsafe trait DoubleEndedConsumer<A: Hay + ?Sized>: ReverseConsumer<A> {}
```

Comparing searcher and consumer, the `.search()` method will look for the first slice
matching the searcher's needle in the span,
and returns the range where the slice is found (relative to the hay's start index).
The `.consume()` method is similar, but anchored to the start of the span.

```rust
let span = unsafe { Span::from_parts("CDEFG", 3..8) };
// we can find "CD" at the start of the span.
assert_eq!("CD".into_searcher().search(span.clone()), Some(3..5));
assert_eq!("CD".into_consumer().consume(span.clone()), Some(5));
// we can only find "EF" in the middle of the span.
assert_eq!("EF".into_searcher().search(span.clone()), Some(5..7));
assert_eq!("EF".into_consumer().consume(span.clone()), None);
// we cannot find "GH" in the span.
assert_eq!("GH".into_searcher().search(span.clone()), None);
assert_eq!("GH".into_consumer().consume(span.clone()), None);
```

The trait also provides a `.trim_start()` method in case a faster specialization exists.

Similar to searchers, the consumers also have the "reverse" and "double-ended" variants.

### Needle

A needle is simply a "factory" of a searcher and consumer.

```rust
trait Needle<H: Haystack>: Sized {
    type Searcher: Searcher<H::Target>;
    type Consumer: Consumer<H::Target>;

    fn into_searcher(self) -> Self::Searcher;
    fn into_consumer(self) -> Self::Consumer;
}
```

Needles are the types where users used to supply into the algorithms.
Needles are usually immutable (stateless), while searchers sometimes require pre-computation and
mutable state when implementing some more sophisticated string searching algorithms.

The relation between `Needle` and `Searcher`/`Consumer` is thus like `IntoIterator` and `Iterator`.

There are two required methods `.into_searcher()` and `.into_consumer()`.
In some needles (e.g. substring search), checking if a prefix match will require much less
pre-computation than checking if any substring match.
Therefore, a consumer could use a more efficient structure with this specialized purpose.

```rust
impl<H: Haystack<Target = str>> Needle<H> for &'p str {
    type Searcher = SliceSearcher<'p, [u8]>;
    type Consumer = NaiveSearcher<'p, [u8]>;
    #[inline]
    fn into_searcher(self) -> Self::Searcher {
        // create a searcher based on Two-Way algorithm.
        SliceSearcher::new(self)
    }
    #[inline]
    fn into_consumer(self) -> Self::Consumer {
        // create a searcher based on naive search (which requires no pre-computation)
        NaiveSearcher::new(self)
    }
}
```

Note that, unlike `IntoIterator`, the standard library is unable to provide a blanket impl:

```rust
impl<H, S> Needle<H> for S
where
    H: Haystack,
    S: Searcher<H::Target> + Consumer<H::Target>,
{
    type Searcher = Self;
    type Consumer = Self;
    fn into_searcher(self) -> Self { self }
    fn into_consumer(self) -> Self { self }
}
```

This is because there is already an existing Needle impl:

```rust
impl<'h, F> Needle<&'h str> for F
where
    F: FnMut(char) -> bool,
{ ... }
```

and a type can implement all of `(FnMut(char) -> bool) + Searcher<str> + Consumer<str>`,
causing impl conflict.

### Algorithms

Standard algorithms are provided as *functions* in the `core::needle::ext` module.

<details><summary>List of algorithms</summary>

**Starts with, ends with**

```rust
pub fn starts_with<H, P>(haystack: H, needle: P) -> bool
where
    H: Haystack,
    P: Needle<H>;

pub fn ends_with<H, P>(haystack: H, needle: P) -> bool
where
    H: Haystack,
    P: Needle<H, Consumer: ReverseConsumer<H::Target>>;
```

**Trim**

```rust
pub fn trim_start<H, P>(haystack: H, needle: P) -> H
where
    H: Haystack,
    P: Needle<H>;

pub fn trim_end<H, P>(haystack: H, needle: P) -> H
where
    H: Haystack,
    P: Needle<H, Consumer: ReverseConsumer<H::Target>>;

pub fn trim<H, P>(haystack: H, needle: P) -> H
where
    H: Haystack,
    P: Needle<H, Consumer: DoubleEndedConsumer<H::Target>>;
```

**Matches**

(These function do return concrete iterators in the actual implementation.)

```rust
pub fn matches<H, P>(haystack: H, needle: P) -> impl Iterator<Item = H>
where
    H: Haystack,
    P: Needle<H>;

pub fn rmatches<H, P>(haystack: H, needle: P) -> impl Iterator<Item = H>
where
    H: Haystack,
    P: Needle<H, Searcher: ReverseSearcher<H::Target>>;

pub fn contains<H, P>(haystack: H, needle: P) -> bool
where
    H: Haystack,
    P: Needle<H>;

pub fn match_indices<H, P>(haystack: H, needle: P) -> impl Iterator<Item = (H::Target::Index, H)>
where
    H: Haystack,
    P: Needle<H>;

pub fn rmatch_indices<H, P>(haystack: H, needle: P) -> impl Iterator<Item = (H::Target::Index, H)>
where
    H: Haystack,
    P: Needle<H, Searcher: ReverseSearcher<H::Target>>;

pub fn find<H, P>(haystack: H, needle: P) -> Option<H::Target::Index>
where
    H: Haystack,
    P: Needle<H>;

pub fn rfind<H, P>(haystack: H, needle: P) -> Option<H::Target::Index>
where
    H: Haystack,
    P: Needle<H, Searcher: ReverseSearcher<H::Target>>;

pub fn match_ranges<H, P>(haystack: H, needle: P) -> impl Iterator<Item = (Range<H::Target::Index>, H)>
where
    H: Haystack,
    P: Needle<H>;

pub fn rmatch_ranges<H, P>(haystack: H, needle: P) -> impl Iterator<Item = (Range<H::Target::Index>, H)>
where
    H: Haystack,
    P: Needle<H, Searcher: ReverseSearcher<H::Target>>;

pub fn find_range<H, P>(haystack: H, needle: P) -> Option<Range<H::Target::Index>>
where
    H: Haystack,
    P: Needle<H>;

pub fn rfind_range<H, P>(haystack: H, needle: P) -> Option<Range<H::Target::Index>>
where
    H: Haystack,
    P: Needle<H, Searcher: ReverseSearcher<H::Target>>;
```

**Split**

```rust
pub fn split<H, P>(haystack: H, needle: P) -> impl Iterator<Item = H>
where
    H: Haystack,
    P: Needle<H>;

pub fn rsplit<H, P>(haystack: H, needle: P) -> impl Iterator<Item = H>
where
    H: Haystack,
    P: Needle<H, Searcher: ReverseSearcher<H::Target>>;

pub fn split_terminator<H, P>(haystack: H, needle: P) -> impl Iterator<Item = H>
where
    H: Haystack,
    P: Needle<H>;

pub fn rsplit_terminator<H, P>(haystack: H, needle: P) -> impl Iterator<Item = H>
where
    H: Haystack,
    P: Needle<H, Searcher: ReverseSearcher<H::Target>>;

pub fn splitn<H, P>(haystack: H, n: usize, needle: P) -> impl Iterator<Item = H>
where
    H: Haystack,
    P: Needle<H>;

pub fn rsplitn<H, P>(haystack: H, n: usize, needle: P) -> impl Iterator<Item = H>
where
    H: Haystack,
    P: Needle<H, Searcher: ReverseSearcher<H::Target>>;
```

**Replace**

```rust
pub fn replace_with<H, P, F, W>(src: H, from: P, replacer: F, writer: W)
where
    H: Haystack,
    P: Needle<H>,
    F: FnMut(H) -> H,
    W: FnMut(H);

pub fn replacen_with<H, P, F, W>(src: H, from: P, replacer: F, n: usize, writer: W)
where
    H: Haystack,
    P: Needle<H>,
    F: FnMut(H) -> H,
    W: FnMut(H);
```

</details>

Most algorithms are very simple to implement using trisection (`.split_around()`). For instance,
`split()` can be implemented as:

```rust
gen fn split<H, P>(haystack: H, needle: P) -> impl Iterator<Item = H>
where
    H: Haystack,
    P: Needle<H>,
{
    let mut searcher = needle.into_searcher();
    let mut rest = Span::from(haystack);
    while let Some(range) = searcher.search(rest.borrow()) {
        let [left, _, right] = unsafe { rest.split_around(range) };
        yield left.into();
        rest = right;
    }
    yield rest;
}
```

These functions are forwarded as *inherent methods* of the haystack type, e.g.

```rust
impl str {
    ...

    pub fn split_mut<'a>(
        &'a mut self,
        needle: impl Needle<&'a mut str>,
    ) -> impl Iterator<Item = &'a mut str> {
        core::needle::split(self, needle)
    }

    pub fn replace<'a>(
        &'a self,
        from: impl Needle<&'a str>,
        to: &str,
    ) -> String {
        let mut res = String::with_capacity(self.len());
        core::needle::replace_with(self, from, |_| to, |r| res.push_str(r));
        res
    }

    ...
}
```

## Standard library changes

* Remove the entire `core::str::pattern` module from public, as this is unstable.

* Add the `core::needle` module with traits and structs shown above.

* Implement `Hay` to `str`, `[T]` and `OsStr`.

* Implement `Haystack` to `∀H: Hay. &H`, `&mut str` and `&mut [T]`.

* Implement `Needle` as following:

    * `Needle<&{mut} str>` for `char`
    * `Needle<&{mut} str>` for `&[char]` and `FnMut(char)->bool`
    * `Needle<&{mut} str>` for `&str`, `&&str` and `&String`
    * `Needle<&{mut} [T]>` for `FnMut(&T)->bool`
    * `Needle<&{mut} [T]>` for `&[T]` where `T: PartialEq`
    * `Needle<&OsStr>` for `&OsStr` and `&str`

* Change the following methods of `str` to use the Needle API:

    * `.contains()`, `.starts_with()`, `.ends_with()`
    * `.find()`, `.rfind()`
    * `.split()`, `.rsplit()`
    * `.split_terminator()`, `.rsplit_terminator()`
    * `.splitn()`, `.rsplitn()`
    * `.matches()`, `.rmatches()`
    * `.match_indices()`, `.rmatch_indices()`
    * `.trim_matches()`, `.trim_left_matches()`, `.trim_right_matches()`
    * `.replace()`, `.replacen()`

    Note also [issue 30459] suggests deprecating `trim_{left, right}`
    and rename them to `trim_{start, end}`.

* Add the following range-returning methods to `str`:

    * `.find_range()`, `.rfind_range()`
    * `.match_ranges()`, `.rmatch_ranges()`

* Add the following mutable methods to `str`, they should all take `&mut self`:

    * `.split_mut()`, `.rsplit_mut()`
    * `.split_terminator_mut()`, `.rsplit_terminator_mut()`
    * `.splitn_mut()`, `.rsplitn_mut()`
    * `.matches_mut()`, `.rmatches_mut()`
    * `.match_indices_mut()`, `.rmatch_indices_mut()`
    * `.match_ranges_mut()`, `.rmatch_ranges_mut()`

* Modify the following iterators in `core::str` to type alias of the corresponding Needle API
    iterators, and mark them as deprecated:

    ```rust
    macro_rules! forward_to_needle_api {
        ($($name:ident)+) => {
            $(
                #[rustc_deprecated]
                pub type $name<'a, P> = needle::ext::$name<&'a str, <P as Pattern<&'a str>>::Searcher>;
            )+
        }
    }

    forward_to_needle_api! {
        MatchIndices Matches Split SplitN SplitTerminator
        RMatchIndices RMatches RSplit RSplitN RSplitTerminator
    }
    ```

    Rust allows the type alias to be stable while the underlying type be unstable.

* Generalize these methods of `[T]` to use the new Needle API:

    * `.split()`, `.split_mut()`, `.rsplit()`, `.rsplit_mut()`
    * `.splitn()`, `.splitn_mut()`, `.rsplitn()`, `rsplitn_mut()`
    * `.starts_with()`, `.ends_with()`

* Add the following methods to `[T]`:

    * `.contains_match()`
        (*note*: the existing `.contains()` method is incompatible with Needle API)
    * `.find()`, `.rfind()`, `.find_range()`, `.rfind_range()`
    * `.matches()`, `.matches_mut()`, `.rmatches()`, `.rmatches_mut()`
    * `.match_indices()`, `.match_indices_mut()`, `.rmatch_indices()`, `.rmatch_indices_mut()`
    * `.match_ranges()`, `.match_ranges_mut()`, `.rmatch_ranges()`, `.rmatch_ranges_mut()`
    * `.trim_matches()`, `.trim_start_matches()`, `.trim_end_matches()`
    * `.replace()`, `.replacen()` (produce a `Vec<T>`)

* Modify the following iterators in `core::slice` to type alias of the corresponding
    Needle API iterators, and mark them as deprecated:

    ```rust
    macro_rules! forward_to_needle_api {
        ($($name:ident $name_mut:ident)+) => {
            $(
                #[rustc_deprecated]
                pub type $name<'a, T, P> = needle::ext::$name<&'a [T], ElemSearcher<P>>;
                #[rustc_deprecated]
                pub type $name_mut<'a, T, P> = needle::ext::$name<&'a mut [T], ElemSearcher<P>>;
            )+
        }
    }

    forward_to_needle_api! {
        Split SplitMut
        SplitN SplitNMut
        RSplit RSplitMut
        RSplitN RSplitNMut
    }
    ```

* Add all immutable Needle API algorithms to `OsStr`. The `.replace()` and `.replacen()` methods
    should produce an `OsString`.

## Performance

The benchmark of the `pattern_3` package shows that algorithms using the Needle API ("v3.0 API")
is close to or much faster than the corresponding methods in libstd using v1.0.

The main performance improvement comes from `trim()`. In v1.0, `trim()` depends on
the `Searcher::next_reject()` method, which requires initializing a searcher and compute
the critical constants for the Two-Way search algorithm. Search algorithms mostly concern about
quickly skip through mismatches, but the purpose of `.next_reject()` is to find mismatches, so a
searcher would be a job mismatch for `trim()`. This justifies the `Consumer` trait in v3.0.

<details><summary>Summary of benchmark</summary>

(The lower the number, the better)

| Test case | v3.0 time change |
|-----------|-----------:|
| `contains('!')` | −75% |
| `contains("!")` | −26% |
| `ends_with('/')` | −31% |
| `ends_with('💤')` | +32% |
| `find('_')` | −80% |
| `find('💤')` | −74% |
| `find(_ == ' ')` | −30% |
| `match_indices("").count()` | −26% |
| `match_indices("a").count()` | −5% |
| `rfind('_')` | −18% |
| `rfind('💤')` | −18% |
| `rfind(_ == ' ')` | −8% |
| `split(" ").count()` | −4% |
| `split("a").count()` | −1% |
| `split("ad").count()` | −20% |
| `starts_with('/')` | −70% |
| `starts_with('💤')` | −56% |
| `starts_with("💩💩")` | −40% |
| `starts_with(_.is_ascii())` | −11% |
| `trim_end('!')` | −19% |
| `trim_end("m!")` | −97% |
| `trim_left(_.is_ascii())` | −57% |
| `trim_right(_.is_ascii())` | −54% |
| `trim_start('💩')` | −32% |
| `trim_start("💩💩")` | −97% |

</details>

# Drawbacks
[drawbacks]: #drawbacks

* This RFC suggests generalizing some stabilized methods of `str` and `[T]` to adapt
    the Needle API. This might cause inference breakage.

* Some parts of the Haystack trait (e.g. the `.restore_range()` method) may not be intuitive enough.

* This RFC does not address some problems raised in [issue 27721]:

    1. v3.0 still assumes strict left-to-right or right-to-left searching.
        Some niche data structures like [suffix table] as a haystack would return matches without
        any particular order, and thus cannot be supported.

        [suffix table]: https://docs.rs/suffix/1.0.0/suffix/struct.SuffixTable.html#method.positions

    2. Needles are still moved when converting to a Searcher or Consumer.
        Taking the entire ownership of the needle might prevent some use cases... ?

* Stabilization of this RFC is blocked by [RFC 1672] \(disjointness based on associated types)
    which is postponed.

    The default Needle implementation currently uses an impl that covers all haystacks
    (`impl<H: Haystack<Target = A>> Needle<H> for N`) for some types, and several impls for
    individual types for others (`impl<'h> Needle<&'h A> for N`). Ideally *every* such impl
    should use the blanket impl.
    Unfortunately, due to lack of RFC 1672, there would be conflict between these impls:

    ```rust
    // 1.
    impl<'p, H> Needle<H> for &'p [char]
    where
        H: Haystack<Target = str>,
    { ... }
    impl<'p, H> Needle<H> for &'p [T] // `T` can be `char`
    where
        H: Haystack<Target = [T]>,
        T: PartialEq + 'p,
    { ... }

    // 2.
    impl<H, F> Needle<H> for F
    where
        H: Haystack<Target = str>,
        F: FnMut(char) -> bool,
    { ... }
    impl<T, H, F> Needle<H> for F
    where
        H: Haystack<Target = [T]>,
        F: FnMut(&T) -> bool, // `F` can impl both `FnMut(char)->bool` and `FnMut(&T)->bool`.
        T: PartialEq,
    { ... }

    // 3.
    impl<'p, H> Needle<H> for &'p str
    where
        H: Haystack<Target = str>,
    { ... }
    impl<'p, H> Needle<H> for &'p str
    where
        H: Haystack<Target = OsStr>,
    { ... }
    ```

    We currently provide concrete impls like `impl<'h, 'p> Needle<&'h OsStr> for &'p str`
    as workaround, but if we stabilize the `Needle` trait before RFC 1672 is implemented,
    a third-party crate can sneak in an impl:

    ```rust
    struct MyOsString { ... };
    impl Deref for MyOsString {
        type Target = OsStr;
        ...
    }
    impl Haystack for MyOsString { ... }

    impl<'p> Needle<MyOsString> for &'p str { ... }
    ```

    and causes the standard library not able to further generalize (this is a breaking change).

    RFC 1672 is currently blocked by `chalk` integration before it could be reopened.

# Rationale and alternatives
[alternatives]: #alternatives

## Principles

These are some guiding principles v3.0 will adhere to.

### Generic algorithms

1. The Needle API should define an interface which can be used to easily implement
    all algorithms the standard library currently provides:

    * `starts_with()`, `ends_with()`
    * `trim_left_matches()`, `trim_right_matches()`, `trim_matches()`
    * `contains()`, `find()`, `rfind()`
    * `matches()`, `rmatches()`, `match_indices()`, `rmatch_indices()`
    * `split()`, `rsplit()`, `split_terminator()`, `rsplit_terminator()`
    * `splitn()`, `rsplitn()`
    * `replace()`, `replacen()`

2. We should not need "non-local unsafety" when writing these algorithms. Mainly, we should not need
    to do borrowck by hand (e.g. ensuring there is no overlapping mutable slices across functions).

### Haystack implementor

3. The standard slice types must be supported:
    `&str`, `&mut str`, `&[T]`, `&mut [T]`, `Vec<T>`, and `&OsStr`.

4. The API should be compatible with linked list and rope data structure as haystack,
    assuming we get either custom DST or GATs implemented.

### Needle/Searcher implementor

5. The existing needle for `&str` and `&mut str` should be supported:

    * `char`
    * `FnMut(char) -> bool`, `&[char]`
    * `&str`, `&&str`, `&String`

    Additionally, these re-implementations should not be slower than
    the existing ones in the standard library.

6. These needles for `&[T]`, `&mut [T]` and `Vec<T>` should be supported:

    * `FnMut(&T) -> bool`
    * `&[T]` where `T: PartialEq`

7. These needles for `&OsStr` should be supported:

    * `&str`
    * `&OsStr`

8. It should be possible to implement `Needle` for `&Regex` within the `regex` package.

9. One should not need to implement a `Searcher` three times to support `&[T]`, `&mut [T]` and
    `Vec<T>`. The searcher should rely on that these all can be borrowed as an `&[T]`.

## Design rationales

The section lists some important use cases which shape v3.0.

### No more `.next_reject()`

In v1.0 a searcher provides a `.next()` method which returns what is being seen ahead:
a match, no-match, or end-to-string, and then advance the cursor.

None of the generic algorithms besides `starts_with()`/`ends_with()`
uses the full power of `.next()`. The rest depend entirely on filtered versions of `.next()`:

* `.next_match()`, which produces ranges of matches, is used for `matches()` and `split()` etc.
* `.next_reject()`, which produces ranges of non-matches, is used for `trim()`.

Implementing `.next()` is sometimes not trivial. In v1.2 this method is entirely abolished
in favor of implementing `.next_match()` and `.next_reject()` directly.
The `starts_with()` methods are supported instead via a specialized method in the Needle trait.

However, we see that even `.next_reject()` is not something obvious. Given that `.next_reject()`
is only used in `trim()`, in v3.0 we decide to remove this method as well,
and instead make the Needle implement `trim()` directly.

### Searching in a `&mut str`

In all versions of Pattern APIs up to v2.0, the "haystack" is directly managed by the searcher.

```rust
// v2.0
trait Pattern<H: PatternHaystack> {
    type Searcher: Searcher<H>;
    fn into_searcher(self, haystack: H) -> Self::Searcher;
}
trait Searcher<H: PatternHaystack> {
    fn haystack(&self) -> H::Haystack; // e.g. returns (*mut u8, *mut u8) for H = &mut str
    fn next_match(&mut self) -> Option<(H::Cursor, H::Cursor)>;
    ...
}
```

The generic algorithms like `matches()` and `split()` would turn the cursor pair back into slices.
With mutable slices, this means logically both the searcher and the `matches()`/`split()` iterators
would hold a copy of the same mutable slice, which violates the "Aliasing XOR Mutability" rule.

This could be avoid by having the searcher carefully written to not look back into parts given out
via `next_match()`/`next_reject()`/`next_match_back()`/`next_reject_back()`,
however this kind of unsafety is very un-rustic (contradicts with "fearless concurrency").

A better way to avoid this is to ensure there is a unique owner to the haystack. Therefore, the
generic algorithm must now *borrow* the haystack for the searcher to work with:

```rust
// v3.0-alpha.1
trait Needle<H: Haystack> {
    type Searcher: Searcher<H>;
    fn into_searcher(self) -> Self::Searcher;
    //^ searcher no longer captures the haystack.
}
trait Searcher<H: Haystack> {
    // no more haystack() method.
    fn search(&mut self, haystack: &H) -> Option<Range<H::Index>>;
}
```

The `matches()` algorithm can then take the whole responsibility to split out
non-overlapping slices of the haystack it owns:

```rust
// v3.0-alpha.1
gen fn matches<H: Haystack, P: Needle<H>>(mut haystack: H, needle: P) -> impl Iterator<Item = H> {
    let mut searcher = needle.into_searcher();
    while let Some(range) = searcher.search(&haystack) {
        // split the haystack into 3 parts.
        let [_, matched, rest] = haystack.split_around(range);
        haystack = rest;
        yield matched;
    }
}
```

### Matching a `&Regex`

In the prototype above, we always feed the remaining haystack into `.search()`.
This works fine for built-in needle types like `char` and `&str`,
but is totally broken for more advanced regular expression needles.

The main issue is due to anchors and look-around.
Anchors like `^` and `$` depend on the actual position where the slice appears.
Look-around like `(?=foo)`, `(?<!foo)` and `\b` depend on parts which may have already matched.
These means to make regex work, we must pass the entire haystack (not just the remaining part),
and a range indicating what's the part should be matched.

In fact, this behavior is consistent with all regex libraries in the wild,
e.g. [`regex`], [`onig`] and [`pcre`].

```rust
// v3.0-alpha.2
trait Searcher<H: Haystack> {
    fn search(&mut self, full_haystack: &H, range: Range<H::Index>) -> Option<Range<H::Index>>;
}
```

This API completely conflicts with `&mut str` as a haystack though. This is fine as a `&mut str` is
incompatible with look-around anyway, but it is not OK for `matches()` which need to support both
"matching `&mut str` with `char`" and "matching `&str` with `&Regex`".

We fix this problem by treating the haystack and range as a single entity we call **span**:

```rust
// v3.0-alpha.3
trait Searcher<H: Haystack> {
    fn search(&mut self, span: (&H, Range<H::Index>)) -> Option<Range<H::Index>>;
}
gen fn matches<H: Haystack, P: Needle<H>>(haystack: H, needle: P) -> impl Iterator<Item = H> {
    let mut searcher = needle.into_searcher();
    let mut span = (haystack, haystack.start_index()..haystack.end_index());
    while let Some(range) = searcher.search((&span.0, span.1.clone())) {
        // split the span into 3 parts.
        let [_, matched, rest] = span.split_around(range);
        span = rest;
        yield matched.0.slice_unchecked(matched.1);
    }
}
```

For a span of `&str`, we will implement `.split_around()` to keep the original haystack,
and only split the ranges. While for `&mut str`, this method will split the haystack apart.

The call the these a *shared span* and *unique span* respectively. The split behavior of shared span
in fact is independent of haystack, and the operation is done entirely on the Range alone.
Thus we could reduce repetitive implementation by providing `Span<H>` in the standard library.
The Haystack implementation only needs to specify which flavor is chosen by a marker trait.

```rust
// v3.0-alpha.4
trait SharedHaystack: Haystack + Clone {}

struct Span<H: Haystack> {
    haystack: H,
    range: Range<H::Index>,
}

impl<H: Haystack> Span<H> {
    fn split_around(self, range: Range<H::Index>) -> [Self; 3];
    fn borrow(&self) -> (&H::Target, Range<H::Index>);
    ...
}

gen fn matches<H: Haystack, P: Needle<H>>(haystack: H, needle: P) -> impl Iterator<Item = H> {
    let mut searcher = needle.into_searcher();
    let mut span = H::Span::from(haystack);
    while let Some(range) = searcher.search(span.borrow()) {
        let [_, matched, rest] = span.split_around(range);
        span = rest;
        yield H::from(matched);
    }
}
```

### Hay: Don't repeat yourself

When we support searching both `&str` and `&mut str`, we'll often need to implement the same
algorithm to both types. v2.0 solves this by using macros, which works but is not elegant.

Since both `&str` and `&mut str` can be borrowed as a `str`, we could force every haystack
to implement `Borrow`. We call the borrowed type a **hay**. The searcher can then only work on
the hay, instead of haystack.

```rust
// v3.0-alpha.5
unsafe trait Haystack: Deref<Target: Hay> {
    ...
}
trait Searcher<A: Hay + ?Sized> {
    fn search(&mut self, span: Span<&A>) -> Option<Range<A::Index>>;
}
```

Unfortunately, a Needle must be associated with the Haystack,
because we must not allow "match `&mut str` with `&Regex`" to happen.
Thus macros would still be needed, though not surrounding the entire module.

```rust
// v3.0-alpha.5
trait Needle<H: Haystack> {
    type Searcher: Searcher<H::Target>;
    ...
}
```

### Consumer

In v2.0 and before, a pattern (needle) will need to specialize `starts_with()` and `ends_with()`.

```rust
// v2.0
trait Pattern<H: PatternHaystack> {
    ...
    fn is_prefix_of(self, haystack: H) -> bool;
    fn is_suffix_of(self, haystack: H) -> bool where Self::Searcher: ReverseSearcher<H>;
}
```

In v3.0, we have removed `.next_reject()` from Searcher, and thus Needle needs to provide
`.trim_start()` and `.trim_end()` as well, making the `Needle` trait quite large.

There are many disadvantages by putting these specialization methods directly inside `Needle`:

1. [Issue 20021] means the `Needle` impl for `&Regex` will still need to
    implement `.is_suffix_of()` and `.trim_end()` even if they are `unimplemented!()`
2. These two methods do not use the searcher directly, but is bounded by
    `where Self::Searcher: ReverseSearcher<H>` which feels strange.
3. More code needs to be repeated to delegate an implementation e.g. from `&str` to `&[u8]`.

A solution move `.is_prefix_of()` and `.trim_start()` directly into `Searcher`. However, a searcher
sometimes requires preprocessing unnecessary for these operations. Therefore, instead we put them
into a separate entity called a *consumer*.

```rust
// v3.0-alpha.6
trait Needle<H: Haystack> {
    type Consumer: Consumer<H::Target>;
    fn into_consumer(self) -> Self::Consumer;
    ...
}
trait Consumer<A: Hay + ?Sized> {
    fn is_prefix_of(&mut self, hay: &A) -> bool;
    fn trim_start(&mut self, hay: &A) -> A::Index;
}
```

We observed that `.is_prefix_of()` and `.trim_start()` have one thing in common: they both
only match the beginning of text. This allows us to require only a single method in
the `Consumer` trait.

```rust
// v3.0-alpha.7
trait Consumer<A: Hay + ?Sized> {
    fn consume(&mut self, hay: Span<&A>) -> Option<A::Index>;
    fn trim_start(&mut self, hay: &A) -> A::Index { /* default impl */ }
}
```

Both `starts_with()` and `trim()` can be efficiently implemented in terms of `.consume()`,
though for some needles a specialized `trim()` can be even faster, so we keep this default method.

## Miscellaneous decisions

### `usize` as index instead of pointers

Pattern API v1.3–v2.0 all used cursors (pointers) as the primary indexing method.
v3.0 still supports cursor-based indexing, but reverts to `usize` for the built-in slice types
(`str`, `[T]` and `OsStr`). There are two reasons for this:

1. **Zero-sized types**. All elements of a slice of ZSTs e.g. `[()]` have the same pointer.
    A proper haystack/searcher implementation would need to check `size_of::<T>()`
    and encode the index into (non-zero) pointers when the size is 0. This made the code very ugly
    and easy to get wrong (the v2.0 implementation does not consider ZSTs for instance).

2. **No performance advantage**. We have tested the performance and found that using integer index
    or cursor pointer have similar performance.

### DSTs instead of GATs

We share a searcher implementation by introducing the `Hay` trait, as the dereference target of the
`Haystack` trait, i.e. `&[T]`, `&mut [T]` and `Vec<T>` will all be delegated to `[T]`:

```rust
unsafe trait Haystack: Deref<Target: Hay> + Sized {
    ...
}
unsafe trait Searcher<A: Hay + ?Sized> {
    fn search(&mut self, span: Span<&A>) -> Option<Range<A::Index>>;
}
```

The problem is not every haystack can be dereferenced. Proper support of any types beyond slices
would require custom dynamic-sized types (DSTs).

An alternative formation is delegating to a shared haystack by generic associated types (GATs):

```rust
unsafe trait Haystack: Sized {
    type Shared<'a>: SharedHaystack;
    fn borrow(&self) -> Self::Shared<'_>;
    ...
}
unsafe trait Searcher<H: SharedHaystack> {
    fn search(&mut self, span: Span<H>) -> Option<Range<H::Index>>;
}
```

We have decided to go with the DSTs approach because:

1. **Non-slice haystacks are rare**. The built-in types that v3.0 aims to support all have
    corresponding built-in DSTs (`str`, `[T]` and `OsStr`), making the problem of custom DSTs
    irrelevant in the standard library.

2. **GATs is still unimplemented**. While the RFC for GATs has been accepted, the implementation
    has still not landed on the Rust compiler, making it impossible to create a test prototype.

### `Deref` instead of `Borrow`

The `Haystack` trait inherits `Deref` and requires its `Target` to implement `Hay`. An alternative
is extending `Borrow` instead:

```rust
unsafe trait Haystack: Borrow<Self::Hay> + Sized {
    type Hay: Hay + ?Sized;
    ...
}
```

The advantage of `Borrow` is that it does not force us to rely on custom DST because
`∀T. T: Borrow<T>`, but that is not the whole picture — the owned type `LinkedList<T>` cannot
implement `Hay`, because it cannot properly implement `slice_unchecked(&self, ...) -> &Self`
(we cannot magically make up a borrowed sub-list).

And thus the more general `Borrow` trait offers no advantage over `Deref`.

### Searcher makes Hay an input type instead of associated type

The `Searcher` and `Consumer` traits makes the hay as input type.
This makes any algorithm relying on a `ReverseSearcher` need to spell out the hay as well.

```rust
trait Searcher<A: Hay + ?Sized> {
    fn search(&mut self, span: Span<&A>) -> Option<Range<A::Index>>;
}

fn rfind<H, P>(haystack: H, needle: P) -> Option<H::Target::Index>
where
    H: Haystack,
    P: Needle<H>,
    P::Searcher: ReverseSearcher<H::Target>; // <---
```

An alternative is to make Hay an associated type:

```rust
trait Searcher {
    type Hay: Hay + ?Sized;
    fn search(&mut self, span: Span<&Self::Hay>) -> Option<Range<Self::Hay::Index>>;
}

fn rfind<H, P>(haystack: H, needle: P) -> Option<H::Target::Index>
where
    H: Haystack,
    P: Needle<H>,
    P::Searcher: ReverseSearcher;
```

This would mean a searcher type can only search on one haystack. It turns out a searcher is shared
quite frequently, e.g. the two-way search algorithm is shared among the needles of `&[T]`, `&str`
and `&OsStr`. Associated type would force creation of many wrapper types which is annoying.

Therefore we stay with having the hay as the input type, the same choice taken in v2.0 and before.

### Specialization of `contains()`

v3.0 removed the `Needle::is_contained_in()` method. The `contains()` algorithm simply returned
`searcher.search(span).is_some()`. The micro-benchmarks shows no performance decrease,
thus the method is removed to reduce the API surface.

### Needle for `&[T]` only requires `T: PartialEq`

Sub-slice searching nowadays uses the Two-Way search algorithm, which requires ordered alphabet
i.e. `T: Ord`. However, there are already two stabilized APIs only assuming `T: PartialEq`:

```rust
impl<T> [T] {
    pub fn starts_with(&self, needle: &[T]) -> bool
    where
        T: PartialEq;

    pub fn ends_with(&self, needle: &[T]) -> bool
    where
        T: PartialEq;
}
```

While we could allow only `starts_with`/`ends_with` to be bound on `PartialEq` and make the rest
of the array searching algorithm require `T: Ord`, it feels very inconsistent to do so.

With specialization, this dilemma can be easily fixed: we will fallback to an algorithm
which only requires `T: PartialEq` (e.g. [`galil-seiferas`] or even naive search),
and use the faster Two-Way algorithm when `T: Ord`.

### Not having default implementations for `search` and `consume`

In the `Searcher` and `Consumer` traits, `.search()` and `.consume()` can be implemented
in terms of each other:

```rust
impl<A, C> Searcher<A> for C
where
    A: Hay + ?Sized,
    C: Consumer<A>,
{
    fn search(&mut self, span: Span<&A>) -> Option<Range<A::Index>> {
        // we can implement `search` in terms of `consume`
        let (hay, range) = span.into_parts();
        loop {
            unsafe {
                if let Some(end) = self.consume(Span::from_span(hay, range.clone())) {
                    return Some(range.start..end);
                }
                if range.start == range.end {
                    return None;
                }
                range.start = hay.next_index(range.start);
            }
        }
    }
}

impl<A, S> Consumer<A> for S
where
    A: Hay + ?Sized,
    S: Searcher<A>,
{
    fn consume(&mut self, span: Span<&A>) -> Option<A::Index> {
        // we can implement `consume` in terms of `search`
        let start = span.original_range().start;
        let range = self.search(span)?;
        if range.start == start {
            Some(range.end)
        } else {
            None
        }
    }
}
```

These fallbacks should only be used when the needle does not allow more efficient implementations,
which is often not the case. To encourage needle implementations to support both primitives,
where they should have full control of the details, we keep them as required methods.

### Names of everything

* **Haystack**. Inherited from the v1.0 method `Searcher::haystack()`. v2.0 called it
    `PatternHaystack` since `Haystack` is an associated type referring to a range of cursors,
    but v3.0 does away the exclusive cursor-based design and thus can choose the shorter name
    for the trait.

* **Hay**. Chosen as a shorter but related name from "Haystack", similar to the relation in
    `String` → `str` and `PathBuf` → `Path`.

* **Needle**. Renamed from `Pattern` to clear confusion with the language's pattern matching.
    Calling it "needle" to pair up with "haystack".

* **Searcher::search()**. The name "Searcher" is the same as v1.0. The method is renamed from
    `.next_match()` since it needs to take a span as input and thus no longer iterator-like.
    It is renamed to `.search()` as a shorter verb and also consistent with the trait name.

* **Consumer::consume()**. The name is almost randomly chosen as there's no good name for
    this operation. This name is taken from the same function in the [`re2` library][re2-consume].

    * `Consumer` is totally different from `Searcher`. Calling it `PrefixSearcher` or
        `AnchoredSearcher` would imply a non-existing sub-classing relationship.

    * We would also like a name which is only a single word.

    * We want the name *not* start with the letter **S**
        so we could easily distinguish between this and `Searcher` when quick-scanning the code,
        in particular when `ReverseXxxer` is involved.

    * "Matcher" (using name from Python) is incompatible with the existing `.matches()` method.
        Besides, the meaning of "match" is very ambiguous among other libraries.

    <details><summary>Names from other languages and libraries</summary>

    | Library                   | Substring         | Start of text     | Entire string         |
    |---------------------------|-------------------|-------------------|-----------------------|
    | [C# (.NET)][cs-regex]     | `Match`           | -                 | -                     |
    | [C++][cpp-regex]          | `regex_search`    | -                 | `regex_match`         |
    | [D][d-regex]              | `matchFirst`      | -                 | -                     |
    | [Dart][dart-regex]        | `firstMatch`      | `matchAsPrefix`   | -                     |
    | [Erlang][erlang-regex]    | `run`             | (`anchored`)      | -                     |
    | [Go][go-regex]            | `Find`            | -                 | -                     |
    | [Haskell][haskell-regex]  | `match`           | -                 | -                     |
    | [ICU][icu-regex]          | `find`            | `lookingAt`       | `matches`             |
    | [Java (JVM)][java-regex]  | `find`            | `lookingAt`       | `matches`             |
    | [JavaScript][js-regex]    | `exec`/`match`    | -                 | -                     |
    | [Kotlin][kotlin-regex]    | `find`            | -                 | `matchEntire`         |
    | [Lua][lua-regex]          | `find`/`match`    | -                 | -                     |
    | [Nim][nim-regex]          | `find`            | -                 | `match`               |
    | [OCaml][ocaml-regex]      | `search_forward`  | `string_match`    | -                     |
    | [Oniguruma][onig-regex]   | `onig_search`     | -                 | `onig_match`          |
    | [PCRE2][pcre2-regex]      | `pcre2_match`    | (`PCRE2_ANCHORED`) | (`PCRE2_ENDANCHORED`) |
    | [POSIX][posix-regex]      | `regexec`         | -                 | -                     |
    | [Python][python-regex]    | `search`          | `match`           | `fullmatch`           |
    | [re2][re2-regex]          | `PartialMatch`    | `Consume`         | `FullMatch`           |
    | [Ruby][ruby-regex]        | `match`           | -                 | -                     |
    | [Rust][rust-regex]        | `find`            | -                 | -                     |
    | [Scala][scala-regex]      | `findFirstIn`     | `findPrefixOf`    | -                     |
    | [Swift][swift-regex]      | `firstMatch`      | -                 | -                     |

    [cs-regex]: https://docs.microsoft.com/en-us/dotnet/api/system.text.regularexpressions.regex
    [cpp-regex]: https://en.cppreference.com/w/cpp/regex
    [d-regex]: https://dlang.org/phobos/std_regex.html
    [dart-regex]: https://api.dartlang.org/stable/1.24.3/dart-core/Pattern-class.html
    [java-regex]: https://docs.oracle.com/javase/10/docs/api/java/util/regex/Matcher.html
    [js-regex]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_Expressions
    [pcre2-regex]: https://pcre.org/current/doc/html/pcre2api.html#SEC27
    [swift-regex]: https://developer.apple.com/documentation/foundation/nsregularexpression
    [icu-regex]: http://icu-project.org/apiref/icu4c/classRegexMatcher.html
    [ruby-regex]: https://ruby-doc.org/core-2.5.0/Regexp.html
    [ocaml-regex]: http://caml.inria.fr/pub/docs/manual-ocaml/libref/Str.html
    [go-regex]: https://golang.org/pkg/regexp/
    [kotlin-regex]: https://kotlinlang.org/api/latest/jvm/stdlib/kotlin.text/-regex/
    [scala-regex]: https://www.scala-lang.org/api/current/scala/util/matching/Regex.html
    [lua-regex]: https://www.lua.org/manual/5.3/manual.html#6.4
    [nim-regex]: https://nim-lang.org/docs/re.html
    [python-regex]: https://docs.python.org/3/library/re.html
    [erlang-regex]: http://erlang.org/doc/man/re.html
    [haskell-regex]: https://hackage.haskell.org/package/regex-base-0.93.2/docs/Text-Regex-Base-RegexLike.html
    [posix-regex]: http://pubs.opengroup.org/onlinepubs/9699919799//functions/regexec.html
    [rust-regex]: https://docs.rs/regex/1.0.1/regex/struct.Regex.html
    [onig-regex]: https://github.com/kkos/oniguruma/blob/master/doc/API
    [re2-regex]: https://github.com/google/re2/blob/master/re2/re2.h

    </details>

* **rsearch()**, **rconsume()**. The common naming convention of algorithms for reverse searching
    is adding an `r` prefix, so we do the same for the trait methods as well.

* **Span**. The name is taken from the rustc compiler.

## Alternatives

* The names of everything except `Searcher` and `Haystack` are not finalized.

# Prior art

## Previous attempts

### v1.0

The existing `Pattern` API was introduced in [RFC 528] to provide a common interface for several
search-related operations on a string. There were several minor revisions after the RFC was
accepted, but till nowadays is still an unstable API.

A `Pattern` is currently implemented for the following types:

* `char` — search for a single character in a string.
* `&[char]` — search for a character set in a string.
* `&str`, `&&str`, `&String` — search for a substring.
* `FnMut(char) -> bool` — search by property of a character.
* `&regex::Regex` — search by regular expression (provided through the `regex` package).

```rust
trait Pattern<'a> {
    type Searcher: Searcher<'a>;
    fn into_searcher(self, haystack: &'a str) -> Self::Searcher;

    fn is_contained_in(self, haystack: &'a str) -> bool { ... }
    fn is_prefix_of(self, haystack: &'a str) -> bool { ... }
    fn is_suffix_of(self, haystack: &'a str) -> bool where Self::Searcher: ReverseSearcher<'a> { ... }
}
```

The `Pattern` trait is a builder object. To perform searching, implementations will convert itself
into a `Searcher` object. This conversion serves two purposes:

1. Preprocess the pattern to allow for faster algorithm, e.g. the `Pattern::into_search` for
    substring search will calculate critical information to perform the Two-Way search algorithm.
2. Store the mutable search states.

```rust
unsafe trait Searcher<'a> {
    fn haystack(&self) -> &'a str;
    fn next_match(&mut self) -> Option<(usize, usize)> { ... }
    fn next_reject(&mut self) -> Option<(usize, usize)> { ... }
    fn next(&mut self) -> SearchStep;
}
```

Calling `next_match()` or `next_reject()` will yield a range:

* `next_match()` — returns the next substring range of the haystack which matches a single instance
    of the pattern.
* `next_reject()` — returns the next longest substring range of the haystack which contains no
    pattern at all.

(The `next()` call interleaves both methods above.)

```rust
// for simplicity, `where` clauses involving ReverseSearcher and DoubleEndedSearcher are omitted.
impl str {
    fn contains(&'a self, pat: impl Pattern<'a>) -> bool;
    fn starts_with(&'a self, pat: impl Pattern<'a>) -> bool;
    fn find(&'a self, pat: impl Pattern<'a>) -> Option<usize>;
    fn split(&'a self, pat: impl Pattern<'a>) -> impl Iterator<Item = &'a str>;
    fn split_terminator(&'a self, pat: impl Pattern<'a>) -> impl Iterator<Item = &'a str>;
    fn splitn(&'a self, n: usize, pat: impl Pattern<'a>) -> impl Iterator<Item = &'a str>;
    fn matches(&'a self, pat: impl Pattern<'a>) -> impl Iterator<Item = &'a str>;
    fn match_indices(&'a self, pat: impl Pattern<'a>) -> impl Iterator<Item = (usize, &'a str)>;
    fn trim_left_matches(&'a self, pat: impl Pattern<'a>) -> &'a str;
    fn replace(&'a self, from: impl Pattern<'a>, to: &str) -> String;
    fn replacen(&'a self, from: impl Pattern<'a>, to: &str, count usize) -> String;

    // The following requires the Pattern's Searcher to additionally be constrained by `ReverseSearcher`
    fn ends_with(&'a self, pat: impl Pattern<'a>) -> bool;
    fn rfind(&'a self, pat: impl Pattern<'a>) -> Option<usize>;
    fn rsplit(&'a self, pat: impl Pattern<'a>) -> impl Iterator<Item = &'a str>;
    fn rsplit_terminator(&'a self, pat: impl Pattern<'a>) -> impl Iterator<Item = &'a str>;
    fn rsplitn(&'a self, n: usize, pat: impl Pattern<'a>) -> impl Iterator<Item = &'a str>;
    fn rmatches(&'a self, pat: impl Pattern<'a>) -> impl Iterator<Item = &'a str>;
    fn rmatch_indices(&'a self, pat: impl Pattern<'a>) -> impl Iterator<Item = (usize, &'a str)>;
    fn trim_right_matches(&'a self, pat: impl Pattern<'a>) -> &'a str;

    // The following requires the Pattern's Searcher to additionally be constrained by `DoubleEndedSearcher`
    fn trim_matches(&'a self, pat: impl Pattern<'a>) -> &'a str;
}
```

Using the result from the `SearchStep` stream, the `Pattern` API can be used to implement the above
string methods.

While the pattern-to-searcher conversion is beneficial when searching the entire haystack, it is
often wasteful in simple functions like `starts_with` and `ends_with` (a sub-slice equality check is
optimal). Therefore, the specialized methods like `Pattern::is_prefix_of` are provided.

### v1.2–v1.5

The `Pattern` API in Rust only supports searching a string. An [attempt][v1.5-comment] to
evolve this to arbitrary haystack type can be found in the repository [Kimundi/pattern_api_sketch].

```rust
trait Pattern<H: SearchPtrs>: Sized {
    type Searcher: Searcher<H>;
    fn into_searcher(self, haystack: H) -> Self::Searcher;
    ...
}

unsafe trait Searcher<H: SearchPtrs> {
    fn haystack(&self) -> H::Haystack;
    fn next_match(&mut self) -> Option<(H::Cursor, H::Cursor)>;
    fn next_reject(&mut self) -> Option<(H::Cursor, H::Cursor)>;
}
```

The most obvious change is to replace all `&'a str` by an arbitrary type `H`. The type still needs
to "behave like a string" though, thus the `SearchPtrs` bound, which will be used to turn a pair of
cursors (equivalent to byte offsets) into a "substring" of the haystack for the `split` and `match`
methods.

```rust
trait SearchPtrs { // e.g. implemented for &str
    type Haystack: Copy; // e.g. (*const u8, *const u8)
    type Cursor: Copy; // e.g. *const u8

    unsafe fn offset_from_start(hs: Self::Haystack, begin: Self::Cursor) -> usize;
    unsafe fn range_to_self(hs: Self::Haystack, start: Self::Cursor, end: Self::Cursor) -> Self;
    unsafe fn cursor_at_front(hs: Self::Haystack) -> Self::Cursor;
    unsafe fn cursor_at_back(hs: Self::Haystack) -> Self::Cursor;
}
```

### v2.0

The [v2.0 API][Kimundi/rust_pattern_api_v2] was introduced due to [RFC 1309],
trying to cover `OsStr` as well. But other than `OsStr` support
the v2.0 API is essentially the same as the v1.5 API.

```rust
trait Pattern<H: PatternHaystack>: Sized {
    type Searcher: Searcher<H>;
    fn into_searcher(self, haystack: H) -> Self::Searcher;
    ...
}

unsafe trait Searcher<H: PatternHaystack> {
    fn haystack(&self) -> H::Haystack;
    fn next_match(&mut self) -> Option<(H::Cursor, H::Cursor)>;
    fn next_reject(&mut self) -> Option<(H::Cursor, H::Cursor)>;
}

trait PatternHaystack: Sized { // same as SearchPtrs in v1.5
    type Haystack: Copy;
    type Cursor: Copy + Ord;
    type MatchType; // yielded item types from `matches()` and `split()`

    fn into_haystack(self) -> Self::Haystack;
    fn offset_from_front(hs: Self::Haystack, begin: Self::Cursor) -> usize;
    fn cursor_at_front(hs: Self::Haystack) -> Self::Cursor;
    fn cursor_at_back(hs: Self::Haystack) -> Self::Cursor;
    unsafe fn range_to_self(hs: Self::Haystack, start: Self::Cursor, end: Self::Cursor) -> Self::MatchType;
    fn match_type_len(mt: &Self::MatchType) -> usize;
}
```

## Haskell

Haskell is perhaps one of the few languages where a generic string matching API is found,
since it also has so many string types like Rust 😝, and there isn't an official regex
implementation (unlike C++ which won't give insight how a `Searcher` interface should be designed).

Haskell's [`regex-base`] is the base package which provides the type classes for regex matching.

The type class `Extract` is corresponding to `Haystack` in this RFC.

```haskell
class Extract source where
    empty :: source
    before :: Int -> source -> source
    after :: Int -> source -> source
```

```rust
// equivalent meaning in terms of Rust.
trait Extract: Sized {
    fn empty() -> Self;
    fn before(self, index: usize) -> Self;
    fn after(self, index: usize) -> Self;
}
```

The type class `RegexLike` is corresponding to `Searcher` in this RFC.

```haskell
class (Extract source) => RegexLike regex source where
    matchOnceText :: regex -> source -> Maybe (source, MatchText source, source)
    matchAllText :: regex -> source -> [MatchText source]
    -- the rest are default implementations depending on these two functions.
```

```rust
// equivalent meaning in terms of Rust.
trait RegexLike<Source: Extract>: Sized {
    fn match_once_text(self, source: Source) -> Option<(Source, MatchText<Source>, Source)>;
    fn match_all_text(self, source: Source) -> impl IntoIterator<Item = MatchText<Source>>;
    // ...
}
```

Similar to this RFC, the primary search method `matchOnceText` is trisection-based.

Unlike this RFC, the `Extract` class is much simpler.

1. Haskell doesn't have the shared/mutable/owned variant of the same type of string.
    Therefore it does not need the `Hay`/`Haystack` trait separation, and also does not need
    a dedicated `split :: Int -> source -> (source, source)` method.
2. Haskell's strings do not enforce a particular encoding on its string types, thus `next_index`
    and `prev_index` become simply `(+ 1)` and `(− 1)`.
3. The `Extract` class only supports indexing using an integer, so `start_index` must be `0`.
    `end_index` is also not needed since `before` and `after` (the slicing operations) will
    automatically clamp the index.

# Unresolved questions
[unresolved]: #unresolved-questions

* Currently, due to RFC 2089 and/or 2289 not being implemented, using a `Haystack` in any algorithm
    would need to a redundant where clause:

    ```rust
    fn starts_with<H, P>(haystack: H, needle: P) -> bool
    where
        H: Haystack,
        P: Needle<H>,
        H::Target: Hay, // <-- this line
    { ... }
    ```

    This RFC assumes that before stabilizing, either RFC should have been implemented.

* For simplicity the prototype implementation fallbacks to the "naive search algorithm"
    when `T: !Ord` by always factorizing the needle `arr` into `arr[..1] ++ arr[1..]`.
    It is not proven that this is equivalent to the "naive search",
    though unit testing does suggest this works.

    As mentioned in the RFC, there are faster algorithms for searching a `T: !Ord` slice.
    It is not decided if we should complicate the standard library to support this though.

* We could represent `SharedHaystack` using a more general concept of "cheaply cloneable":

    ```rust
    pub trait ShallowClone: Clone {}
    impl<'a, T: ?Sized + 'a> ShallowClone for &'a T {}
    impl<T: ?Sized> ShallowClone for Rc<T> {}
    impl<T: ?Sized> ShallowClone for Arc<T> {}
    ```

    and all `H: SharedHaystack` bound can be replaced by `H: Haystack + ShallowClone`.
    But this generalization brings more questions e.g. should `[u32; N]: ShallowClone`.
    This should be better left to a new RFC, and since `SharedHaystack` is mainly used for
    the core type `&A` only, we could keep `SharedHaystack` unstable longer
    (a separate track from the main Needle API) until this question is resolved.

* With a benefit of simplified API,
    we may want to merge `Consumer` and `Searcher` into a single trait.

[RFC 528]: https://github.com/rust-lang/rfcs/pull/528
[RFC 1309]: https://github.com/rust-lang/rfcs/pull/1309
[RFC 1672]: https://github.com/rust-lang/rfcs/pull/1672
[RFC 2089]: https://github.com/rust-lang/rfcs/pull/2089
[RFC 2289]: https://github.com/rust-lang/rfcs/pull/2289
[RFC 2295]: https://github.com/rust-lang/rfcs/pull/2295
[Issue 20021]: https://github.com/rust-lang/rust/issues/20021
[issue 27721]: https://github.com/rust-lang/rust/issues/27721
[issue 30459]: https://github.com/rust-lang/rust/issues/30459
[issue 38078]: https://github.com/rust-lang/rust/issues/38078
[issue 44491]: https://github.com/rust-lang/rust/issues/44491
[issue 49802]: https://github.com/rust-lang/rust/issues/49802
[`pattern-3`]: https://crates.io/crates/pattern-3
[`regex`]: https://crates.io/crates/regex
[`onig`]: https://crates.io/crates/onig
[`pcre`]: https://crates.io/crates/pcre
[`regex-base`]: https://hackage.haskell.org/package/regex-base
[`galil-seiferas`]: https://crates.io/crates/galil-seiferas
[Kimundi/pattern_api_sketch]: https://github.com/Kimundi/pattern_api_sketch
[Kimundi/rust_pattern_api_v2]: https://github.com/Kimundi/rust_pattern_api_v2
[v1.5-comment]: https://github.com/rust-lang/rust/issues/27721#issuecomment-185405392
[re2-consume]: https://github.com/google/re2/blob/2018-07-01/re2/re2.h#L330-L334
