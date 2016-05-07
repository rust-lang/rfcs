- Feature Name: tuple_cons_cells
- Start Date: 2016-05-17
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a syntax for expressing tuples as a head and tail pair, similar to a Lisp
cons cell.

# Motivation
[motivation]: #motivation

Currently, rust doesn't give the user any way to talk about generically-sized
tuples. This means that it's not possible to, for example, implement a trait
for all tuples. Traits like `fmt::Debug` are only implemented for tuples of up
to some arbitrary number of elements. These impls are generated using some
hacky macro magic and they end up polluting the rust-doc documentation. This
RFC would alleviate these problems.

This RFC is also a step towards more general variadic generics as proposed in
draft RFC #376. While that RFC discusses using variadic lists of types in
positions such as function argument lists, this RFC only covers the specific
case of tuples of generic arity. Whatever "full" variadic generics eventually
look like, when and if they get implemented, it is unlikely we would want the
design for tuples to work any differently to what's proposed here.

# Detailed design
[design]: #detailed-design

This RFC proposes introducing two new syntactic forms: one for tuple types and
one for tuple terms. With this new syntax a tuple can be expressed as `(head_0,
head_1, ... head_n; tail)` where `head_x` are the first `n + 1` elements of the
tuple and `tail` is a tuple containing the remainder of the tuple. The table
below shows some of the different ways of expressing the same tuple combining
current rust syntax and the new syntax.

    0 elements
        ()
        (; ())
        (; (; ()))

    1 element

        (a,)
        (a; ())
        (a,; ())
        (; (a,))
        (; (a; ()))
        (; (a,; ()))

    2 elements
    
        (a, b)
        (a, b,)
        (a, b; ())
        (a, b,; ())
        (a; (b,))
        (a; (b; ()))
        (a; (b,; ()))

    3 elements

        (a, b, c)
        (a, b, c,)
        (a, b, c; ())
        (a, b, c,; ())
        (a, b; (c,))
        (a, b,; (c,))
        (a, b; (c; ()))
        (a, b; (c,; ()))
        (a, b,; (c; ()))
        (a, b,; (c,; ()))
        (a; (b, c))
        (a; (b, c,))
        (a,; (b, c))
        (a,; (b, c,))
        (a; (b, c; ()))
        (a; (b, c,; ()))
        (a,; (b, c; ()))
        (a; (b, c,; ()))
        (a,; (b, c,; ()))
        (a; (b; (c,)))
        (a; (b,; (c,)))
        (a,; (b; (c,)))
        (a,; (b,; (c,)))
        (a; (b; (c; ())))
        (a; (b; (c,; ())))
        (a; (b,; (c; ())))
        (a; (b,; (c,; ())))
        (a,; (b; (c; ())))
        (a,; (b; (c,; ())))
        (a,; (b,; (c; ())))
        (a,; (b,; (c,; ())))

    and so forth...

This RFC proposes equivalent syntax for tuple types. Formally, the syntax for
tuple types could be described with the following grammar fragment:

```
ty_tuple
: "(" ")"
| "(" ty "," ty_tuple_inner ")"
| "(" ty_tuple_inner ";" ty ")"

ty_tuple_inner
: %empty
| ty
| ty "," ty_tuple_inner
```

With this syntax, any tuple can be expressed as either `()` or `(head; tail)`.
This makes it possible for generic code to handle all tuples by covering just
these two cases.

In addition to syntax for expressions and types, this RFC also proposes syntax
for destructuring tuples into a head and tail. Here, the obvious syntax is
used, ie. `let (head; tail) = (a; b);` results in `head == a` and
`tail == b`.

This RFC also proposes a new marker trait be added to the language. `Tuple` is
a trait which is implemented for `()` and `(H; T) where T: Tuple`. In general,
the type `(H; T)` is only valid when `T: Tuple`.

### Representation

The main problem with implementing this RFC is the question of representation.
At the memory level, in current Rust, there is no guarantee that the
representation of an `(a; b)` contains the representation of a `b`. The
solution proposed here is two-fold. First, we allow types to have separate
stride and size as per RFC issue #1397. Secondly, we layout tuples in reverse
order. Under this scheme, the tuple `(A, B, C) : (u16, u16, u32)` would be
represented as

```
------------------------------------------------
| Byte | 0  | 1  | 2  | 3  | 4  | 5  | 6  | 7  |
|------|-------------------|---------|---------|
| Data |      C (u32)      | B (u16) | A (u16) |
------------------------------------------------
```

And it's tail `(B, C) : (u16, u32)` would be represented as

```
------------------------------------------------
| Byte | 0  | 1  | 2  | 3  | 4  | 5  | 6  | 7  |
|------|-------------------|---------|---------|
| Data |      C (u32)      | B (u16) | padding |
------------------------------------------------
```

Crucially, the stride of this type is only 6 bytes. This means that a
`&mut (u16, u32)` can not be used to modify the tuple's two trailing "padding"
bytes as any tuple (accessed through a reference) may be the tail of a larger
tuple.

## Example

This code shows how we could use the proposed syntax to `impl Debug` for all
tuples.

```
// libcore/fmt/mod.rs

// Private trait used to help impl Debug.
trait TupleExt: Tuple + Debug {
    fn debug_tail(&self, f: &mut Formatter) -> Result;
}

impl TupleExt for () {
    fn debug_tail(&self, f: &mut Formatter) -> Result {
        write!(f, ")")
    }
}

impl<H: Debug, T: TupleExt> TupleExt for (H; T) {
    fn debug_tail(&self, f: &mut Formatter) -> Result {
        let (ref head; ref tail) = *self;
        try!(write!(f, ", {:?}", *head));
        tail.debug_tail(f)
    }
}

// impl Debug for 0 elements
impl Debug for () {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "()")
    }
}

// impl Debug for 1 element
impl<H: Debug> Debug for (H,) {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let (ref head,) = *self;
        write!(f, "({:?},)", *head)
    }
}

// impl Debug for 2 or more elements
impl<H0: Debug, H1: Debug, T: TupleExt> Debug for (H0, H1; T) {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let (ref head; ref tail) = *self;
        try!(write!(f, "({:?}", *head));
        tail.debug_tail(f)
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

* Adds more syntax and another concept that people will need to learn.
* Code that modifies the head element of a tuple through a mutable reference -
  where the compiler cannot guarantee that the tuple is not part of a larger
  tuple - will sometimes be less efficient as the compiler will no longer be
  able to assume that it's safe to overwrite trailing padding bytes.

# Alternatives
[alternatives]: #alternatives

* Not do this.
* Consider using a different syntax. The `(H; T)` syntax was chosen to resemble
  the `[T; N]` syntax for arrays (on the theory that a tuple type is defined by
  the type of its element and the type of its tail whereas an array type is
  defined by the type of its elements and its length). Another possible syntax
  would be `(a, b, ...more_elems)` although this would conflict with the
  inclusive ranges RFC. Another is `(a, b, more_elems...)` although this looks
  very similar to range syntax and may be confusing.
* Consider a different layout. By packing tuples less efficiently we could
  obviate the need for the stride/size distinction and make updating the head
  elements of tuples more efficient. Overall though I'm not sure this
  would be a win. The efficiency hit associated with the proposed design only
  happens when modifying a tuple through a mutable reference. Also the reference
  must be to the tuple itself, not to an element in the tuple like what one
  would obtain by writing `(mut ref a, ...) = some_tuple`. Also, the update
  must happen to the head element of the tuple and the head element must be
  small. Conversely, packing tuples less efficiently would often result in
  significantly less efficient layout (eg. `(u16, u16, u32)` taking 12 bytes
  instead of 8). [More knowledgeable people than me disagree though](https://github.com/rust-lang/rfcs/issues/1397#issuecomment-213311508),
  so it would be worth discussing this further and trying to obtain data to
  inform a decision with.
* Sidestep the representation issue by disallowing references to the tail of a
  tuple.  This would largely defeat the purpose of the RFC as, for example, the
  `Debug` implementation above would be impossible to write.
* Sidestep the representation issue by making references to the tail of a tuple
  expand into a tuple of references. Here, `let (; ref x) = (0, 1, 2);` would
  yield `x == (&0, &1, &2)`. This would get extremely messy. Consider the
  `Debug` implementation above which recursively formats its tuple argument. On
  the first iteration it would be handling a tuple of values. On the second, a
  tuple of references-to-values. On the third, a tuple of
  references-to-references-to-values. And so forth. It would also be surprising
  and unintuitive that `let (; x) = (0, 1, 2)` gives `x == (0, 1, 2)` but
  `let (; ref x) = (0, 1, 2)` doesn't give `x == &(0, 1, 2)`.

# Unresolved questions
[unresolved]: #unresolved-questions

The representation issue warrants further discussion.

