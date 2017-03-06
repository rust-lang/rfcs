- Feature Name: autoref_match
- Start Date: 2016-08-12
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Better ergonomics for match (and similar expressions).

Prevent some of the boilerplate required for match expressions by auto-
dereferencing the target expression, and 'auto-referencing' pattern variables.
The rules for the latter are based closely on those for captured variables in
closures.

For example, current:

```
match *x {
    Foo(ref x) => { ... }
    Bar(ref mut y, z) => { ... }
}
```

proposed:

```
match x {
    Foo(x) => { ... }
    Bar(y, z) => { ... }
}
```

# Motivation
[motivation]: #motivation

Rust is usually strict when distinguishing between value and reference types. In
particular, distinguishing borrowed and owned data. However, there is often a
trade-off between [explicit-ness and ergonomics](https://blog.rust-lang.org/2017/03/02/lang-ergonomics.html),
and Rust errs on the side of ergonomics in some carefully selected places.
Notably when using the dot operator to call methods and access fields, and when
declaring closures.

The match expression is an extremely common expression and arguably, the most
important control flow mechanism in Rust. Borrowed data is probably the most
common form in the language. However, using match expressions and borrowed data
together can be frustrating: getting the correct combination of `*`, `&`, and
`ref` to satisfy the type and borrow checkers is a common problem, and one which
is often encountered early by Rust beginners. It is especially frustrating since
it seems that the compiler can guess what is needed but gives you error messages
instead of helping.

For example, consider the following program:

```
enum E { Foo(...), Bar }

fn f(e: &E) {
    match e { ... }
}

```

It is clear what we want to do here - we want to check which variant `e` is a
reference to. Annoyingly, we have two valid choices:

```
match e {
    &E::Foo(...) => { ... }
    &E::Bar => { ... }
}
```

and

```
match *e {
    E::Foo(...) => { ... }
    E::Bar => { ... }
}
```

The former is more obvious, but requires more noisey syntax (an `&` on every
arm). The latter can appear a bit magical to newcomers - the type checker treats
`*e` as a value, but the borrow checker treats the data as borrowed for the
duration of the match. It also does not work with nested types, `match (*e,)
...` for example is not allowed.

In either case if we further bind variables, we must ensure that we do not
attempt to move data, e.g.,

```
match *e {
    E::Foo(x) => { ... }
    E::Bar => { ... }
}
```

If the type of `x` does not have the `Copy` bound, then this will give a borrow
check error. We must use the `ref` keyword to take a reference: `E::Foo(ref x)`
(or `&E::Foo(ref x)` if we match `e` rather than `*e`).

The `ref` keyword is a pain for Rust beginners, and a bit of a wart for everyone
else. It violates the rule of patterns matching declarations, it is not found
anywhere outside of patterns, and it is often confused with `&`. (See for
example, https://github.com/rust-lang/rust-by-example/issues/390).

Match expressions are an area where programmers often end up playing 'type
Tetris': adding operators until the compiler stops complaining, without
understanding the underlying issues. This serves little benefit - we can make
match expressions much more ergonomic without sacrificing safety or readability.

Match ergonomics has been highlighted as an area for improvement in 2017:
[internals thread](https://internals.rust-lang.org/t/roadmap-2017-productivity-learning-curve-and-expressiveness/4097)
and [Rustconf keynote](https://www.youtube.com/watch?v=pTQxHIzGqFI&list=PLE7tQUdRKcybLShxegjn0xyTTDJeYwEkI&index=1).


# Detailed design
[design]: #detailed-design

I propose two mostly orthogonal improvements: match target auto-deref removes
the need to write `*e` in the above example; match arm 'auto-ref' infers use
of `ref` in patterns in match arms, allowing them to be elided. Both features
aim to make `match` expressions easier to read and write by reducing
boilerplate.

## Match target auto-deref

The target expression (aka discriminant) in a match expression is `expr` in
`match expr { ... }`.

If the target expression is borrowed data, then the compiler will implicitly
dereference the target expression as many times as necessary to make every
pattern in every arm type check. Dereference here is equivalent to using the `*`
operator: either a primitive pointer dereference or calling the `deref` method,
if a type implements the `Deref` trait.

Auto-deref allows the programmer to access borrowed data within a limited scope
without having to explicitly dereference the data. This is analogous to the
implicit dereferencing performed by the dot operator, however, the scope of
dereference is inline, rather than defined by a function.

More formally, let `T0...Tn` be the types of the patterns of each arm in the
match expression. If the pattern is a wildcard or variable, we ignore its type.
Let `expr` be the target expression.

I will use `A <: B` to mean `A` is a subtype of `B`, and `e: T` to mean that
expression `e` has type `T`. I use `*^m` to mean applying the `*` operator `m`
times where `m >= 0`.

```
forall i in [0..n] Ti <: U
*^m expr: U
...
--------------------------
match expr { ... }: ...
```

If this rule can be satisfied with different values of `m` (e.g., if none of the
arms constrain the type), then the compiler will choose the smallest value.

Note that this dereferencing allows the compiler to insert calls to the `deref`
method (if there is a `Deref` implementation) for any intermediate type found
whilst dereferencing.

We do not permit auto-referencing. Since `deref` is implemented for `&self`, to
apply the auto-deref, `expr` must be a borrowed reference. E.g., if `expr` has
type `Rc<Foo>` we will not implicitly dereference to `Foo`, but we will if the
type is `&Rc<Foo>`. This means that we never implicitly borrow owned data, which
should make this conversion easier to reason about. It also matches the
behaviour of deref coercions (although it is weaker than the dot operator
conversions).

A less conservative alternative would be to allow auto-referencing. I believe
there are no technical obstacles. That would be more flexible and match the
behaviour of the dot operator. However, I believe constraining to borrowed data
makes reasoning easier. We can always adopt auto-referencing later.

The above implies that the patterns in each arm must have the same type. For
example, the following match would not type check:

```
match x: &Option<_> {
    Some(_) => {}
    &None => {}
}
```


### Type inference and backwards compatibility

Currently, we might infer the type of the target expression from the type of the
patterns. In order to be backwards compatible, we must be able to fall back to
the type known from the patterns if type inference is stuck without that
information. I think this will only require choosing `m = 0` if type inference
can't make progress.


## Match arm 'auto-ref'

The programmer should no longer have to annotate variables in
patterns with `ref` or `ref mut`; the compiler will infer that information from
the arm body following the same rules as for captured variables in closures (see
RFCs [114](https://github.com/rust-lang/rfcs/blob/master/text/0114-closures.md)
and [231](https://github.com/rust-lang/rfcs/blob/master/text/0231-upvar-capture-inference.md)).

For each variable declared in a pattern, the compiler will inspect how that
variable is used. If the variable is required to be treated by value, then the
value is moved or copied. If the value is mutated (or a mutable reference to the
variable is taken), then the variable is mutably borrowed. Otherwise, the
variable is immutably borrowed. Similarly to closures, the type of the variable
is not affected, only its move/borrow semantics at runtime and in the borrow
checker.

In designing this feature we must be careful to ensure backwards compatibility
where the user does write `ref`, `ref mut`, or `mut`.

If there is a `ref` or `ref mut` annotation on a variable, then that annotation
is obeyed and no further auto-referencing is performed. The compiler will not
infer `mut` for a by-value variable even if it is mutated.

Example with mutability:

```
let mut i: i32 = 0;
match i {
    j => j += 1,  // `j` is treated as `ref mut j` since `+=` takes `j` as `&mut i32`.
}
println!("{}", i); // prints `1`.

match i {
    j => j = j + 1,  // ERROR: j is not mutable (since `j` is used by value here,
                     // the compiler will not treat `j` as `mut`).
}
```

To opt out of all implicit dereferencing in a pattern, we could allow the
programmer to prefix the pattern with the `move` keyword. This would follow
closures and indicates that all variables are to be treated strictly as values.

I don't think that the `move` opt-out is necessary and I don't think we should
support it unless it is required. It is necessary for closures where the
closure escapes the stack frame and therefore inference cannot be complete. An
analogous situation for match arms seems impossible.


### Type checking

Currently, to type check a match expression we:
  1. check the target expression,
  2. check the patterns on each arm (and check that the type matches
     the target expression (subtyping is kind of complex here, but it is close enough
     to assume that we do not take subtyping into account and that types must match
     exactly)),
  3. check each arm (and check that all arms have a common super-
     type and that that type is a sub-type of any expected type for the whole
     match).

To accommodate match arm auto-ref, step 3 is extended with a step to determine
the move/borrow semantics of variables. After type checking the arm body, uses
of variables are inspected if auto-ref should take place.

To see how a variable is treated, the compiler follows this algorithm:

* any variable declared `ref` is borrowed and no further adjustment is made.
* any variable declared `mut` is by-value (and mutable) and no further adjustment
  is made.
* if the type of a variable is `Copy`, it is not borrowed and is copied.
* for any remaining variables, the match arm is inspected:
  - if the target of the `match` expression is mutated or a mutable
    reference is taken in the body, all variables in the pattern have move
    semantics (we could also issue a lint warning in this case).
  - If the variable is ever mutated or found to have mutable reference type, then
    it is treated as borrowed and mutable.
  - If the variable is ever used by value (and is not `Copy`), then it has move
    semantics.
  - Otherwise it is treated as borrowed.


'Borrowed' here means that the variable is compiled to a pointer and the borrow
checker treats the variable as a reference. One can imagine a desugaring where
the variable declaration (`x`) is desugared to `ref x` and uses of the variable
to `*x`.

In every case, the type of the variable does not change (it has the type as
expected when treating the pattern as written). The behaviour of the variable
changes only with respect to *borrow* checking, not *type* checking.


## More details

### Extension to similar expressions

`if let`, `while let`, and `for` expressions all have patterns similar to match
arms and suffer from the same ergonomic friction around `ref`. In fact,
internally, the compiler treats all these expressions in the same way (they are
desugared early in compilation). It would therefore be trivial to extend both
proposed features to these other expressions. I believe it also makes intuitive
sense - `if let` in particular is usually thought of like a single arm match
expression.

Alternatively, we could just support `match` expressions. This would be more
conservative, but the only advantage is that `if let` remains similar to `let`
and patterns in function declarations.


### Backwards compatibility

Since there is no opt-in to these features, we must be sure that there are no
backwards compatibility concerns.

Target auto-deref is backwards compatible, since it will only apply to programs
that do not type check today.

My reasoning that arm 'auto-ref' is backwards compatible is as follows:

* We are only concerned with programs that type check today - they must continue
  to do so, and give the same results.
* For any variable annotated with `ref`, there is no change to type checking, so
  there cannot be issues.
* The concern is thus for variables declared without `ref`, where they were
  previously treated as by-value, but would be treated by-reference. If a
  variable has copy semantics, then it is not referenced, so we only need
  concern ourselves with moved variables. This means that in the worrying cases,
  the matched value cannot be accessed after the match (since we currently move
  the variable out of it).

Given these steps, we have considerably reduced the scope for differing
semantics.

First, let us consider only immutable data. Following the rules for inferring
ref-ness, we know that every use of a variable must be a reference. What if a
variable use could be either a move or a reference? I believe the only ways this
is possible is as the receiver of a method call or used in a match expression
(as proposed in this RFC).

In the first case, the variable will be treated as by-value rather than by-
reference due to the method lookup rules and we won't infer a `ref`. For
example, consider `x: T` and `m` where `m` is declared for `self: T` and `self:
&T`:

```
fn foo(x: T) {
    match x {
        x => x.m(),
    }
}
```

Where `x.m()` is used in an arm body we type check this with `x: T` (we don't
consider any auto-referencing at this stage); when we type check the method
call, we use the implementation where `self: T` because it is found before the
other implementation. That means that we consider `x` to be used by value and we
do not infer any auto-borrowing.

In the second case, consider the following code where there are two different
implementations of `foo` for `T` and `&T`. Current code will call `T::foo`, we
must ensure that we do not infer a `ref` for `x` and thus call `&T::foo`. In any
case where the compiler can type check without auto-deref'ing the target
expression it will do so (see the 'smallest `m`' rule), therefore we will be
backwards compatible here.

```
match a: T {
    x => {
        match x {
            y => y.foo(),
        }
    }
}
```

We might cause backwards incompatibility if we can observe the ref-ness via
mutation. By the same reasoning as above, such a situation would involve a
variable being moved, and so a difference could only be observed inside the match
arm. Mutation could occur by mutating the target expression or a pattern
variable. In the former case, we are protected by the special case about not
inferring `ref` if the target expression is mutated directly or indirectly. In
the latter case, the variable must have been marked `mut`, so again the ref-ness
will not be changed.

For example,

```
struct S(i32);

fn foo(x: S) {
    match x {
        y => {
            x.0 += 1; // ERROR - x has been moved (because cannot infer `ref y`)
            y.0 += 1; // ERROR - y is not mutable
        }
    }
    // s is moved and cannot be accessed.
}
```

Finally, we might cause backwards incompatibility if a program that currently
compiles will not do so with the proposed changes. We can apply much of the same
reasoning as above. I believe the only way this could happen is where we
reference a variable which was previously moved and this means that it (and any
structure it belongs to) is borrowed. This could change the outcome of borrow
checking. However, I believe the only way this could be observed is if the
target expression is mutated, and this is covered by a special case rule.

To see why that rule is necessary, consider the following code:

```
let mut i = String::new();

match i {
    j => {
        g(&j);
        i = "Hello".to_owned();
    }
}
```

Under this RFC, `&j` in `g(&j)` would mean that we infer that `j` is used by-
reference. However, this would mean that `i` is borrowed for the duration of the
match, and thus the assignment to `i` would be a borrow-check error. The special
case rule about the discriminant being mutated or mutably borrowed means we take
`j` by-value.


### Safety

Whenever considering relaxing Rust's rules around borrowing, we must consider
safety. Could either of the proposed rules make Rust code less safe?

Target auto-dereference introduces implicit dereferencing. However, there is no
safety concern in the strict sense because the compiler still treats the data as
borrowed and therefore, for example, won't let data move out of the `match`. We
do not permit implicit dereferencing of owned data (e.g., `Box<_>`).

With match arm auto-ref, there is no hard threat to safety - the compiler is
merely inferring some annotations, the borrow checker remains unchanged and
cannot be compromised.

### Mental model

One risk is that errors become even more confusing - the compiler might complain
about borrowing violations without there being explicit dereferencing of the
target expression. I believe that these errors are less likely due to arm auto-
ref, and that when they do happen, there must be an explicit dereference in the
arm body to indicate an error. Furthermore, since the target must always be
borrowed, either its type or an explicit borrow (`&`) should indicate that the
data is borrowed.

It is reasonable to argue that arm auto-ref makes `match` more confusing
(because there is an additional subtle rule to understand). I don't think this
has proven to be a problem with closures, and I don't see why it should be with
match arms, but some experience with the feature when implemented will help us
decide.

Note that explicit borrows where pattern variables are used are still required,
as they would be if this feature did not exist. I.e., there is still a clear
indication in the source code that owned data becomes borrowed.


# Drawbacks
[drawbacks]: #drawbacks

The major downside of this proposal is that it introduces implicit referencing
and dereferencing, and in particular implicit borrowing of otherwise un-borrowed
data (in the arm auto-ref case). This is generally something we avoid, however,
there is some precedent in the dot operator and closures.

This is a fairly major change to the semantics of a common feature with no opt-
in. Whilst I believe the proposal is backwards compatible (see above) and mostly
only affects code which would not compile today, it is a bit unnerving.

This RFC introduces differences in the semantics of patterns in `match` compared
with `let`. On the other hand, there are differences already - allowing multiple
patterns separated with `|` and `if` guards in match arms.

As an alternative to this proposal, we could apply the same rules to `let` as to
match arms. I think this should not be done because `ref` in `let` statements is
much rarer than in matches, and there is no explicit scope to limit checking of
uses to. These reasons make the facility less necessary and less appealing.


# Alternatives
[alternatives]: #alternatives


* We could support either target auto-deref or arm auto-ref without the other.
* We could opt-in to arm auto-ref, for example by using closure syntax:

```
match x {
    Foo(x) => { ... }        // No auto-ref
    |Bar(x)| => { ... }      // Auto-ref of x
    move |Baz(x)| => { ... } // No auto-ref, equivalent to the first arm
                             // and so probably unnecessary.
}
```

  There might be parsing issues since we use `|` to separate patterns in match
  arms. Note that this is a bit misleading - this RFC follows the rules for
  variables implicitly captured by closures, not closure parameters.

* Target auto-deref could be done in a slightly different way: intuitively,
  rather than inferring `*`s on the target, we infer `&`s on the patterns in
  match arms. This is more flexible in that it allows different arms to be
  deref'ed a different number of times, and could be made to work with nested
  references. However, it cannot work with smart pointers such as `Rc`, at least
  not in any obvious way. It is also likely to be more complex to implement.
* Ubiquitous auto-ref/deref. Rather than restricting auto-ref/deref to a few
  places in Rust, one could consider allowing it anywhere (or nearly anywhere).
  This might work, however, it would be a huge change, difficult to design, and
  close to impossible to ensure backwards compatibility. It therefore seems
  impractical.

# Unresolved questions
[unresolved]: #unresolved-questions

The biggest open question is if the implementation will actually work. Some of
the details in this RFC, particularly around backwards compatibility are subtle,
and today's [implementation of type checking for match](https://github.com/rust-lang/rust/blob/master/src/librustc_typeck/check/_match.rs)
is already complex.

The interaction of target auto-deref and closures (described above) or the
analogous interaction with arm auto-ref (i.e., nested matches) is one area that
makes me nervous.

Although I believe this proposal is backwards compatible, the fact that we are
making big changes to how match arms are type-checked means there is scope for
problems.

Can we extend auto-deref of the target expression to nested types?
