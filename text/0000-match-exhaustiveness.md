- Feature Name: match_exhaustiveness
- Start Date: 2017-25-01
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Fix the handling of uninhabited types in pattern matches so that it is
consistent with the handling of all other types.

# Motivation
[motivation]: #motivation

The motivations for this RFC are:
 (a) to make Rust more logically consistent and
 (b) to enable users to do pattern matching on expressions involving empty types
     in a way that's more concise and doesn't force them to write dead code.

To explain why Rust's current pattern-matching behaviour is inconsistent we'll
need the following types:

```rust
enum Zero {
}

enum One {
    V0,
}

enum Two {
    V0,
    V1,
}
```

All the examples that follow which apply to `One` and `Two` would also apply to
`Three`, `Four` etc. but not necessarily to `Zero`. This is because the match
syntax surrounding `Zero` is arbitrarily different to the syntax for the other
types.

Suppose we match on a value of type `Two`:

```rust
match two {
    Two::V0 => ...
    Two::V1 => ...
}
```

Here, we have two match arms because `Two` has two variants. So there is one
match arm for each.

We can do the same thing for `One`:

```rust
match one {
    One::V0 => ...
}
```

`One` only has one variant, so we need one match arm to match that one variant.

We can also do the same thing with `Zero`:

```rust
match zero {
}
```

`Zero` has zero variants, so there are zero match arms, one for each variant.

All of the above code works today. So far, so good. But what happens if we put
the value inside a 1-tuple?

```rust
match (two,) {
    (Two::V0,) => ...
    (Two::V1,) => ...
}
```

Like `Two`, a `(Two,)` still has two values: `(Two::V0,)` and `(Two::V1,)`.
Rust allows us to do *deep* matches into types and match on the `Two` inside
the tuple without having to nest match expressions. The concept of "depth" is
important and we'll return to it in a moment.

We can also do the same thing with `(One,)`:

```rust
match (one,) {
    (One::V0,) => ...
}
```

Again, we have a single possible value of this type and our single match arm is
checking precisely for this single value.

But look what happens we try this with `(zero,)`:

```rust
match (zero,) {
}
```

**``error: non-exhaustive patterns: `(_,)` not covered``**

This is clearly nonsense. The pattern *is* exhaustive. `(Zero,)` has just as
many values as `Zero`, ie. none, so we should be able to match on all of them
exhaustively by matching with an empty set of arms.

This doesn't just happen with tuples either though. It happens with references,
structs, enums, arrays - anything that can be matched on to reveal a `Zero`
inside. For example this will not compile either:

```rust
let zero_array: [Zero; 1] = ... ;
match zero_array {
}
```

To better understand the semantics of pattern-nesting, and why we should expect
these examples to work, I want to return to the concept of "depth" in matches.

The example given above of matching on `(Two,)` mentioned that this is a *deep*
match. It is "deep" in the sense that it matches all the way to the bottom of the
type, splitting every possible value into a separate arm.

Let's step back and look at all the possible ways we could match on this type.
Firstly, we could simply bind the matched value to a variable:

```rust
match (two,) {
    two_tup => ...
}
```

This is a *0-deep* match in that it recurses 0 levels into the type.
Hypothetically, the tuple might not even need to be an initialized value
because all this code does is move the value and assign it to another variable.

However, we could also just match on the tuple but not on the `Two`.

```rust
match (two,) {
    (two,) => ...
}
```

This is a *1-deep* match. It recurses 1 level into the type by splitting open
the tuple but doesn't recurse any further. The `Two` contained within is simply
assigned to a variable, its enum discriminant does not need to be read.

Lastly, we can match on the `Two` aswell. As `Two` does not contain any types
that are exposed for matching, this is as deep as we can go.

```rust
match (two,) {
    (Two::V0,) => ...
    (Two::V1,) => ...
}
```

This is a *2-deep* match. It recurses 2 levels in to the type: first, by
splitting open the tuple to access the contained `Two`, then by splitting the
`Two` into its two variants.

In order to further illustrate, let's consider another example: the type `(One,
One)`.

A *0-deep* match on this type looks like this:

```rust
let tup: (One, One) = ...
match tup {
    tup => ... 
}
```

A *1-deep* match on this type looks like this:

```rust
let tup: (One, One) = ...
match tup {
    (x, y) => ... 
}
```

The following match is *2-deep* although it only recurses to 1 level at the
second tuple position:

```rust
let tup: (One, One) = ...
match tup {
    (One::V0, y) => ...
}
```

Whereas this match is fully *2-deep* and matches on all the state contained in
the type:

```rust
let tup: (One, One) = ...
match tup {
    (One::V0, One::V0) => ...
}
```

We can, of course, extend this notion to 3-deep, 4-deep matches etc.
depending on how deeply nested our types are.

Another way to look at these example is by imagining we're using a simplified
version of Rust where all matches are required to be 1-deep. Returning to
our `(Two,)` example we can look at how this would require us to rewrite our
matches of different depths:

The *0-deep* example given before was:

```rust
match (two,) {
    two_tup => ... ,
}
```

Here, the match doesn't actually do anything. This code is equivalent to code
which has no match expressions in it:

```rust
let two_tup = (two,);
...
```

The *1-deep* example is exactly the same, it contains one match expression,
peeling away only the outer layer of the type:

```rust
match (two,) {
    (two,) => ...
}
```

The *2-deep* example requires us to match on the tuple, then the `Two`, in two
separate match statements:

```rust
match (two,) {
    (two,) => {
        match two {
            Two::V0 => ...
            Two::V1 => ...
        }
    },
}
```

This de-nesting of patterns into nested matches is more-or-less what happens
when pattern matching code is compiled down to MIR. It's relevant for
understanding when a value is being "used", a concept we'll explore later.

For further illustration, let's also repeat the `(One, One)` example in this
style.

```rust
let tup: (One, One) = ... ;

// 0-deep example
...

// 1-deep example
match tup {
    (x, y) => ...
}

// 2-deep (in the first position) example
match tup {
    (x, y) => {
        match x {
            One::V0 => ...
        }
    }
}

// 2-deep everywhere example
match tup {
    (x, y) => {
        match x {
            One::V0 => {
                match y {
                    One::V0 => ...
                }
            }
        }
    }
}
```

Now that we've got this way of thinking about matches down-pat, let's consider
again the case of `Zero`. Specifically, let's consider a `Zero` behind several
layers of nesting and, for variety, we'll use references instead of 1-tuples.

Consider the type: `&&Zero`. What do different depths of matches look like
applied to this type? Well, a 0-deep match simply binds the outer-most
reference to a variable:

```rust
let zero_ref_ref: &&Zero = ... ;
match zero_ref_ref {
    zero_ref_ref => ...
}
```

Because all this code does is re-assign `zero_ref_ref` to another variable
called `zero_ref_ref`, it doesn't even need to dereference the pointer.
Implementation-wise, this is a no-op, as we can see if we write it in the
second, de-nested, style:

```rust
let zero_ref_ref: &&Zero = ... ;
let zero_ref_ref = zero_ref_ref;
...
```

We can also consider a 1-deep match. This match recurses into the first
reference and binds a variable there:

```rust
let zero_ref_ref: &&Zero = ... ;
match zero_ref_ref {
    &zero_ref => ...
}
```

This match, presumably, *does* require the outer-most reference to be a
non-dangling pointer so that it has something to bind `zero_ref` to.

A 2-deep match dereferences the outer 2 layers of references:

```rust
let zero_ref_ref: &&Zero = ... ;
match zero_ref_ref {
    &&zero => ...
}
```

Or, written in the de-nested style:

```rust
let zero_ref_ref: &&Zero = ... ;
match zero_ref_ref {
    &zero_ref => {
        match zero_ref {
            &zero => ...
        }
    }
}
```

Finally, a 3-deep match recurses through both layers of references and matches
on the `Zero`. Let's look at the de-nested version of this first:

```rust
let zero_ref_ref: &&Zero = ... ;
match zero_ref_ref {
    &zero_ref => {
        match zero_ref {
            &zero => {
                match zero {
                }
            }
        }
    }
}
```

The important thing to note here is *we end up in an empty match pattern*.
There is no `...` anywhere inside the matches. There is no code that could end
up being run. This code acknowledges the fact that `Zero` is uninhabited and
relies on the assumption that both the references and the `Zero` consists of
valid data.

If we re-nest these matches back into a single match statement with a nested
pattern, we get this:

```rust
let x: &&Zero = ... ;
match x {
}
```

This is simply the shorthand for the previous code example. And it is a
**3-deep** match. Semantically, this code dereferences the outer pointer,
dereferences the inner pointer, examines the enum descriminant of `Zero` then
branches into one of zero possible branches based on the zero possible values
of that discriminant. The equivalent code for `&&Two` looks like this:

```rust
let x: &&Two = ... ;
match x {
    &&Two::V0 => ...
    &&Two::V1 => ...
}
```

This, like the previous example, is a 3-deep match. It requires that the
matched value is deeply valid (all the way to the bottom of the type),
dereferences through all the layers of nesting, and splits into one branch for
every possible value of the overall type.

However the pattern-match on `&&Zero` given above, like the `(Zero,)`  and
`[Zero; 1]` patterns given earlier, will not (currently) compile. This is
because Rust only respects empty pattern-matches at the outer-most layer of a
match. This is in contrast to enums of any other number of variants where you
can nest patterns as deeply as you like.

So why does this matter? Firstly, it's just inconsistent. If I can match on a
`Two` or a `One` behind a reference why can't I do the same thing with a
`Zero`? Disallowing this could potentially break macro code which expands an
enum behind a reference or a tuple or something. Secondly, it makes code that
handles `Zero`-like types weird to write. For example, we want to roll-out the
`!` type at some point and one of the main use-cases for this type is as an
error type for trait methods that return a `Result` with a generic error type
of the implementer's choice. Consider a method `never_fails` which returns
`Result<T, !>`. With the current rules, we'd have to call such a method like
this:

```rust
let t = match never_fails() {
    Ok(t) => t,
    Err(e) => e,
}
```

This relies in an unintuitive way on the `!`-type's ability to transmogrify
into other types. Alternatively we could also write it like this:

```rust
let t = match never_fails() {
    Ok(t) => t,
    Err(e) => match e {},
}
```

But it would be nicer if we could simply write this:

```rust
let t = match never_fails() {
    Ok(t) => t,
}
```

Or even this:

```rust
let Ok(t) = never_fails();
```

Not only are these ways or writing the above code more concise, they don't
force the user to write dead code to explicitly handle a value that can never
occur. These ways of writing patterns aren't a new extra feature, they're just
what you get if you naturally extend to current syntax to cover uninhabited
types the same way it handles all other types. As such, they should be legal.

# Detailed design
[design]: #detailed-design

This has already been implemented, and merged, but has now been hidden behind
the `never_type` feature gate due to the changes being controversial.

Since we're now blocked on resolving that controversy, we need to explore what
it is.

Questions about how to handle uninhabited types tie-in to broader, unanswered,
questions about the semantics of unsafe code and uninitialized data.
This is because uninhabited types are special in one regard - they have no
valid representation. A `bool`, for instance is represented in memory as a
single `u8`. However only the `0b00000000u8` and `0b00000001u8` bit-patterns
are valid. A `bool` that contains the value `42u8` is not really a `bool` at all.
Such values are called "trap representations" and if you try to match on such a
value with code like `match b { true => ..., false => ... }` then the compiler
is allowed to invoke undefined behaviour. If this weren't the case, such a match statement
would simply have to be illegal and users would be forced to treat `bool` as
the same type as `u8`. Similarly, `!` is represented as a zero-sized-type, but
unlike with `()` the empty array of bits is an invalid bit-pattern. In fact, `!`
does not any have valid bit-patterns at all! This means that if we have a `!`
which was produced through `mem::uninitialized` or some other `unsafe` means,
we don't even have to "read" the value (in any sense) to determine that the value is
invalid. The compiler can determine, logically, that this piece of code is
definitely holding a trap representation. This reasoning is what allows us to
treat functions which return `!` as being diverging - we don't need to handle
the possibility of the function returning because it could only possibly return
trap representations which we're free to invoke undefined behaviour on (in this
case the undefined behaviour of what happens next if the function actually does
return). However this reasoning creates all kinds of fun when it comes into
contact with unsafe code where uninitialized values abound. This has been the
topic of [an internals
thread](https://internals.rust-lang.org/t/mem-uninitialized-and-trap-representations/)
and some IRC discussions. The main question raised in these discussion was:
What counts as "reading" or "using" a value? ie. When, exactly, are we allowed
to invoke undefined behaviour when handling invalid, uninitialized data? Should
we consider it safe to:
 * Return an uninitialized value?
 * Return an uninitialized value hidden inside a data structure that can only
   be accessed through a safe API?
 * Move/copy uninitialized data?
 * Pass uninitialized data to a function so long as the function never uses it?
 * Pattern-match on data that contains exposed, uninitialized sub-data so long
   as we don't recursively pattern-match into that sub data?

These are important questions that will need to be answered in the course of
nailing-down the unsafe code guidelines.

*However none of them are relevant here.*

I only bring them up because they were raised in objection to the
pattern-matching changes implemented by this RFC. But accepting this RFC only
forces us to commit to a bare-minimum of completely uncontroversial design
choices regarding unsafe semantics, it does not force our hand on any of the
thornier issues.

I'll explain: For the sake of argument, let's take the most liberal position
and say that all the answers to the above questions are "Yes". We can move
uninitialized data around, return it, pass it, match on larger data structures
containing it, whatever. All safely. The only thing we *can't* do is match on
the uninitialized data itself and encounter a trap representation. This is the
commitment-minimising position. If we can't even commit to this, then we
shouldn't be allowing code like this:

```rust
match b {
    true => ...
    false => ...
}
```

Whatever "reading a value" means, running that code absolutely requires us to
"read" that `bool`. And the fact that it compiles without requiring branches for
the other 254 possible `bool` bit-patterns means that we're already committed to
the notion that matching on a value asserts (in the sense of [Niko's recent
blog post](http://smallcultfollowing.com/babysteps/blog/2017/01/22/assigning-blame-to-unsafe-code/))
that the data is valid.

Note however, this position means that this is safe:

```rust
let b: bool = unsafe { mem::transmute(42u8) };
match b {
    c => ...
}
```

This is only a 0-deep match, it does not (necessarily) read the `bool`. This is
also safe:

```rust
let b: bool = unsafe { mem::transmute(42u8) };
match (b,) {
    (c,) => ...
}
```

This is only a 1-deep match. It matches on the (shallowly-valid) tuple but
does not recurse into the (invalid) `bool`.

*(Note: I'm not saying that these are the unsafe semantics we &ast;should&ast;
have, just that we could have them and they wouldn't conflict with this RFC. I
personally would prefer much stricter semantics which allow us to trust the
type system)*

Let's also consider a `&&bool`:

```rust
let b: bool = unsafe { mem::transmute(42u8) };
let b_ref = &b;
let b_ref_ref = &b_ref;

// 0-deep match. Safe.
match b_ref_ref {
    b_ref_ref => ...
}

// 1-deep match. Safe.
match b_ref_ref {
    &b_ref => ...
}

// 2-deep match. Safe.
match b_ref_ref {
    &&b => ...
}

// 3-deep match.
// Recurse all the way down to the bool and split into 2 arms.
// UNSAFE.
match b_ref_ref {
    &&true  => ...
    &&false => ...
}
```

A more precise way to look at when values are being "read" (under the
hypothetical interpretation of "read" we're assuming here) is by de-nesting the
patterns, in which case we get this:

```rust
let b: bool = unsafe { mem::transmute(42u8) };
let b_ref = &b;
let b_ref_ref = &b_ref;

// 0-deep match. Safe.
...

// 1-deep match. Safe.
match b_ref_ref {
    &b_ref => ...
}

// 2-deep match. Safe.
match b_ref_ref {
    &b_ref => {
        match b_ref {
            &b => ...
        }
    }
}

// 3-deep match.
// Recurse all the way down to the bool and split into 2 arms.
// UNSAFE.
match b_ref_ref {
    &b_ref => {
        match b_ref {
            &b => {
                match b {
                    true => ...
                    false => ...
                }
            }
        }
    }
}
```

Now let's apply the same logic to `!` instead of `bool`. If we instead start with the
expanded, de-nested version we get this:

```rust
let n: ! = unsafe { mem::transmute(()) };
let n_ref = &n;
let n_ref_ref = &n_ref;

// 0-deep match. Safe.
...

// 1-deep match. Safe.
match n_ref_ref {
    &n_ref => ...
}

// 2-deep match. Safe.
match n_ref_ref {
    &n_ref => {
        match n_ref {
            &n => ...
        }
    }
}

// 3-deep match.
// Recurse all the way down to the never and split into 0 arms.
// UNSAFE.
match n_ref_ref {
    &n_ref => {
        match n_ref {
            &n => {
                match n {
                }
            }
        }
    }
}

```

The last match invokes undefined behaviour. And that's fine. We created an
impossible value and in the last match we inspected that value directly. What
else were we expecting to happen?

If we write this the sugary way - without nesting match expression but with
nested patterns instead - we get this:

```rust
let n = unsafe { mem::transmute(42u8) };
let n_ref = &n;
let n_ref_ref = &n_ref;

// 0-deep match. Safe.
match n_ref_ref {
    n_ref_ref => ...
}

// 1-deep match. Safe.
match n_ref_ref {
    &n_ref => ...
}

// 2-deep match. Safe.
match n_ref_ref {
    &&n => ...
}

// 3-deep match.
// Recurse all the way down to the never and split in 0 arms.
// UNSAFE.
match n_ref_ref {
}
```

And, like with `bool`, the last match invokes undefined behaviour.

To summarise the explanation/justification for this RFC with in one sentence:
**When you omit an arm in a match statement due to a nested uninhabited type,
you are pattern-matching all the way down to that type, dereferencing
everything along the way, asserting that everything along the path is valid,
and applying an empty pattern-match to that type**.

We don't need to commit to any extra rules about unsafety in order to allow
this kind of pattern-matching. We just need to agree that when we apply an
empty match to the `Void` inside a `&Void` we are asserting that that `Void` is
valid. Same as if we dereferenced into and applied a match to the `bool` behind a
`&bool`.

As such, this RFC should be considered completely future-safe with regard to
whatever the unsafe code guidelines end up looking like and there should be no
problem with de-gating the proposed changes.

Having argued all this, there are some other concerns regarding this change,
but they're fairly minor.  Firstly, it's been described as weird that you can
add extra assertions and matches to your code's logic by deleting code. For
example, under the weak-validity semantics I've described, this code does not
assert that a `!` inside an `Err` is valid:

```rust
let r: Result<T, !> = ... ;
match r {
    Ok(x) => ...
    Err(_) => ... 
}
```

However this code does, despite having one less line of code!

```rust
let r: Result<T, !> = ... ;
match r {
    Ok(x) => ...
}
```

I don't think this is really that weird. It's just what you get when you
combine uninhabited types with the ability to nest patterns. Once that's
understood, it becomes intuitive. Also the second example stands out somewhat
by the conspicuous omission of the `Err` variant. This hints at the idea that
something is happening here.

Secondly, even though the {0, 1, 2}-deep matches on `&&!` (as shown a page up)
are safe and valid, they will still raise unreachable pattern warnings. This is
because the lint *does* assume that the entire value we're matching on is
deeply-valid, even the sub-data that we're not applying a sub-pattern to. But
obeying the lint (by deleteing the unreachable arm) may change the semantics of
the code.  I don't think this is a big issue. The only time you'll encounter it
is if you're working explicitly with a type like `!`/`Void` and not with some
type parameter `T` where `T = !`. If a user is explicitly handling a `!`, which
they know is invalid, in code which is intended to be live, then they're
already doing something very unsafe and wacky and it's up to them to know to
ignore or disable the lint. Also, this issue is contingent on us never
strengthening the rules around data validity and unsafety beyond the
bare-minimum that I've described (for example, by ruling that any data exposed
to safe code must be valid). We probably should strengthen these rules further,
in which case this issue will become even less relevant or altogether moot (as
the lint will always be correct).

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

These semantics don't need to be explained explicitly in the book chapter on
pattern-matching as they follow directly from the semantics already explained
therein. In fact, those explanations are currently wrong if they don't mention
that we currently treat uninhabited types as a special case. However, these
sorts of empty patterns should be explained in the chapter on uninhabited types
using examples for things like unpacking a `Result<T, !>`.

# Drawbacks
[drawbacks]: #drawbacks

None that I can think of not already mentioned.

# Alternatives
[alternatives]: #alternatives

The status quo.

# Unresolved questions
[unresolved]: #unresolved-questions

None.

