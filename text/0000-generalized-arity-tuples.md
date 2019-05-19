- Feature Name: `generalized_arity_tuples`
- Start Date: 2019-05-22
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Currently, it is not possible to write functions that generalize over tuples with an arbitrary arity. Rust could be able to support this feature if tuples had an alternative type-level representation. This RFC proposes a simple and straight-forward solution that does not break any existing code and that does not include any changes to the syntax or the way Rust reasons about types.

# Motivation
[motivation]: #motivation

Many crucial API functions can intuitively be generalized over tuples of any arity. Examples for such functions are spread over the entire ecosystem:

- `core::ops::Eq::eq`
- `core::iter::Iterator::zip`
- `futures::future::join{,3,4,5}`
- `serde::Serialize::serialize`
- `specs::join::Join::join`
- etc.

Unfortunately, it is not possible to express the generalization strategy in Rust's type system. Instead, a common practice is to generalize code using the macro system. This has two major drawbacks:

- The code is not really general since it can only support a limited number of arities. This is the same restriction as if it had been written down by hand. To make things worse, each library has its own understanding about what is cosidered a good limit.
- A lot of `fn`s or `impl`s are created and sent to tools like `racer` or `cargo doc`. As a result, these tools yield too many items and, hence, obfuscate the generalizing nature of the code.

Despite everything, it is possible to _emulate_ generalized arity tuples in Rust _right now_ by using recursive types. If the compiler were to create those types automatically for each tuple, it would be easily possible to implement, for example, the following generalized function:

- `future::future::join` which consumes a tuple of any arity `(Future<Output=A>, Future<Output=B>, ..., Future<Output=Z>)` and returns a tuple with the same arity `Future<Output=<(A, B, ..., Z)>`

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The following guide illustrates how a `join` function can be implemented that consumes a nested tuple of `Option`s and returns `Some` with all values unwrapped if possilbe and `None` otherwise, s.t.:

- `(Option<i32>, (Option<bool>, Option<&str>))` is mapped to `Option<(i32, (bool, &str))>`
- `(Some(99), (Some(true), Some("text"))).join()` evaluates to `Some((99, (true, "text")))`
- `(Some(99), (None::<bool>, Some("text"))).join()` evaluates to `None`

In order to generalize over the tuples, we first need to define an appropriate abstraction for the `join` function:

```rust
trait Join {
    type Joined;

    fn join(self) -> Option<Self::Joined>;
}
```

Next, we trivially implement `join` on the `Option` type:

```rust
impl<T> Join for Option<T> {
    type Joined = T;

    fn join(self) -> Option<Self::Joined> { self }
}
```

And on the empty tuple:

```rust
impl Join for () {
    type Joined = ();

    fn join(self) -> Option<Self::Joined> { Some(()) }
}
```

The only step left to make the `join` function work on any tuple of `Option`s is to implement a recursion:

```rust
impl<ELEM: Join, TAIL: Join> Join for Tuple<ELEM, TAIL> {
    type Joined = Tuple<ELEM::Joined, TAIL::Joined>;

    fn join(self) -> Option<Self::Joined> {
        if let (Some(elem), Some(tail)) = (self.elem.join(), self.tail.join()) {
            Some(Tuple::new(elem, tail))
        } else {
            None
        }
    }
}
```

Note that `Tuple<ELEM, TAIL>` is a desugared representation of the tuple `(ELEM, TAIL.0, ..., TAIL.n-1)` with arity `n + 1` where `TAIL` is a tuple with arity `n`.

## Why are we already done here?

Consider the type `(Option<i32>, (Option<bool>, Option<&str>))` from the requirements above. This type is just syntactic sugar for the type `Tuple<Option<i32>, Tuple<Tuple<Option<bool>, Tuple<Option<&str>, ()>>, ()>>`.

Now, apply the provided trait implementations step by step on the desugared type. The resulting associated type `Join::Joined` evaluates to `Tuple<i32, Tuple<Tuple<bool, Tuple<&str, ()>>, ()>>`. But this is just the desugared version of `(i32, (bool, &str))`.

This illustrates how the mechanims works on the type level.

## More examples / Advanced type mappings

Generalized implementations on `Tuple<HEAD, TAIL>` are not restricted to mappings between tuples of the same shape. The following example demonstrates how a `last` function can be realized that works with any tuple:

```rust
trait Last {
    type Last;

    fn last(self) -> Self::Last;
}

impl<ELEM> Last for (ELEM,) {
    type Last = ELEM;

    fn last(self) -> Self::Last {
        self.elem
    }
}

impl<ELEM, TAIL: Last> Last for Tuple<ELEM, TAIL> {
    type Last = TAIL::Last;

    fn last(self) -> Self::Last {
        self.tail.last()
    }
}
```

Correcty, the compiler rejects empty tuples while tuples of other sizes are accepted:

```rust
().last();       // does not compile: no method named `last` found for type `()` in the current scope
(1,).last();     // returns 1
(1, "two").last(); // returns "two"
etc.
```

The last example demonstrates how every second element of a tuple can be removed:

```rust
trait Halve {
    type Output;

    fn halve(self) -> Self::Output;
}

impl Halve for () {
    type Output = ();

    fn halve(self) {}
}

impl<ELEM1, ELEM2, TAIL: Halve> Halve for Tuple<ELEM1, Tuple<ELEM2, TAIL>> {
    type Output = Tuple<ELEM1, TAIL>;

    fn halve(self) -> Self::Output {
        Tuple::new(self.elem, self.tail.tail)
    }
}
```

Results:

```rust
().halve();                   // returns ()
(1,).halve()                  // does not compile: no method named `halve` found for type `(i32,)` in the current scope
(1, "two").halve();           // returns (1,)
(1, "two", 3.0, '4').halve(); // returns (1, 3.0)
etc.
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The idea of this RFC to leave the tuple notation as is but to treat tuple type expressions as aliases according to the following pattern:

```rust
type () = (); // Not really an alias. Written down for completeness.
type (A,) = Tuple<A, ()>;
type (A, B) = Tuple<A, Tuple<B, ()>>;
type (A, B, C) = Tuple<A, Tuple<B, Tuple<C, ()>>>;
type (A, (B, C)) = Tuple<A, Tuple<Tuple<B, Tuple<C, ()>>, ()>>
etc.
```

where `Tuple` is a new struct located in `std::ops` with the following definition:

```rust
struct Tuple<ELEM, TAIL> {
    pub elem: ELEM,
    pub tail: TAIL,
}
```

This is everything needed to make the `Join` or `Last` traits from the guide section above work for tuples of any arity.

## Required compiler changes

- The compiler needs to treat any type `(ELEM, TAIL.0, ..., TAIL.n-1)` to be equivalent to `Tuple<ELEM, (TAIL.0, ..., TAIL.n-1)>`. This could work in the same way as `std::io::Result<T>` is considered equivalent to `core::result::Result<T, std::io::Error>`.
- Equivalently, every tuple value `(elem, tail.0, ..., tail.n-1)` must be considered structurally equal to `Tuple { elem: elem, tail: (tail.0, ..., tail.n-1) }`.
- Every tuple index access `tuple.n` must evaluate to `tuple{{.tail}^n}.elem`. In other words, `.tail` must be called `n` times before calling `.elem`.
- `Tuple<_,_>` types must be mapped back to their user-friendly representation when used in compiler messages or documentation.

# Drawbacks
[drawbacks]: #drawbacks

- People might not understand how or why the `Tuple<ELEM, TAIL>` representation works. Probably, library users do not need to fully understand the details but library maintainers, on the other hand, should.
- Although the intention is to make code more understandable, depending on how the documentation is rendered, it could become _less_ understandable.
- Someone might find a better, more general or simpler solution after this RFC has been implemented. In this case, it will be hard to remove the current solution.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The selling point of the proposed solution is that it is completely based on existing concepts. The syntax and type system remain unaffected. Hence, the implementation effort should be predictable and the risk of compromising the overall quality of the language should be low. A second benefit is the possibility to define more advanced type mappings, e.g. `(A, B, C, ..., Z)` &rarr; `(B, A, C, ..., Z)`.

An alternative approach to the tuple generalization problem could be to add some kind of type-level iterator to the language. Although the idea seems simple and straight-forward at first sight, it comes with some big drawbacks:

- New syntax for the iteration must be introduced. This, usually, is a very hard problem.
- The compiler needs a new type-level iterator machinery.
- It is hard to imagine how more advanced type mappings can be realized without introducing even more syntax.

# Prior art
[prior-art]: #prior-art

Similar solutions have been proposed in earlier RFCs. The drawbacks compared to this RFC are summarized here:

- [#1582](https://github.com/rust-lang/rfcs/pull/1582):
  - Introduces new syntax
  - Introduces new traits
  - Deals with memory layout issues
- [#1921](https://github.com/rust-lang/rfcs/pull/1921):
  - Focussed on variadic function arguments
  - Introduces new attributes
  - Changes the way the language works by introducing function overloading
- [#1935](https://github.com/rust-lang/rfcs/pull/1935)
  - Focussed on variadic generics
  - Introduces new syntax
  - Includes new traits
  - Uses special handling for references

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Those points are mainly decisions to be made before implementing the RFC:

- How should compiler messages or the documentation be rendered? The printed output for `Tuple<A, Tuple<B, Tuple<C, ()>>>` must probably be mapped back to `(A, B, C)` for readability. But what if this reverse mapping is impossible as is the case for the generalized tuple `impl`s?
- What should the compiler do with nonsensical tuples? A nonsensical tuple is a `Tuple` whose `TAIL` parameter is not a tuple (e.g. `Tuple<String, String>`). It feels like the easiest and most idiomatic answer is that the compiler should not care and let the code run into a type error as soon as the tuple is used. Nevertheless, nonsensical tuples could be discovered and reported by `clippy`.
- How should the `Tuple` struct look like precisely? Should it export globally visible symbols like `tuple.elem` or `tuple.elem()` or should they be hidden behind a namespace, e.g. `Tuple::elem(tuple)`?

# Future possibilities
[future-possibilities]: #future-possibilities

- Generalized tuples could be an enabler or a replacent for variadic generic type parameters.
- Generalized tuples could be an enabler or a replacent for variadic function arguments.
- It might be possible to use the type aliasing strategy on `structs` or `enums`. Using const generics the struct

  ```rust
  struct MyStruct {
      first: String,
      second: usize,
  }
  ```

  could become

  ```rust
  NamedFieldsStruct<
      "MyStruct",
      Field<
          "first",
          String,
          Field<
              "second"
              usize,
              End,
          >
      >
  >
  ```

- With trait specialization, any list operation should be possible on the type level.