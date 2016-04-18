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
hacky macro magic and they end up polluting the rust-doc documentation.

This RFC would alleviate these problems.

# Detailed design
[design]: #detailed-design

This RFC proposes introducing two new syntactic forms: one for tuple types and
one for tuple terms. With this new syntax a tuple can be expressed as `(head_0,
head_1, ... head_n; tail)` where `head_x` are the first `n` elements of the
tuple and `tail` is a tuple containing the remainder of the tuple. The table
below shows all the different ways of expressing the same tuple combining
current rust syntax and the new syntax.

    0 elements
        ()

    1 element

        (a,)
        (a; ())

    2 elements
    
        (a, b)
        (a, b,)
        (a, b; ())
        (a; (b,))
        (a; (b; ()))

    3 elements

        (a, b, c)
        (a, b, c,)
        (a, b, c; ())
        (a, b; (c,))
        (a, b; (c; ()))
        (a; (b, c))
        (a; (b, c,))
        (a; (b, c; ()))
        (a; (b; (c,)))
        (a; (b; (c; ())))

    and so forth...

This RFC proposes similar syntax for tuple types. With this syntax, any tuple
can be expressed as either `()` or `(head; tail)`. This makes it possible for
generic code to handle all tuples by covering just these two cases.

This RFC also proposes a new marker trait be added to the language. `Tuple` is
a trait which is implemented for `()` and `(H; T) where T: Tuple`. In general,
the type `(H; T)` is only valid when `T: Tuple`.

## Example

This code shows how we could use this syntax to `impl Debug` for all tuples.

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

Adds more syntax and another concept that people will need to learn.

# Alternatives
[alternatives]: #alternatives

* Not do this.
* Consider using a different syntax. The `(H; T)` syntax was chosen to resemble
  the `[T; N]` syntax for arrays (on the theory that a tuple type is defined by
  the type of its element and the type of its tail whereas an array type is
  defined by the type of its elements and its length). Another possible syntax
  would be `(a, b, ...more_elems)` although this would conflict with the
  inclusive ranges RFC.

# Unresolved questions
[unresolved]: #unresolved-questions

None that I can see.

