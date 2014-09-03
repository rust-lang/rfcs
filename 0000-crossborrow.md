- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Allow implicit coercion from any type which implements `Deref<T>` to a borrowed
reference, `&T` (sometimes called cross-borrowing).

# Motivation

Rust code is littered with `&*expr` expressions. These are ugly, off-putting to new-comers,
and provide little benefit (see footnote, below). In the past, we allowed ad-hoc
cross-borrowing from Gc<T> and Box<T> to &T. Whilst the previous system
privileged built in pointer types over user-defined pointer types, it was both
usable and ergonomic. I propose using the `Deref` trait to bring back the
ergonomic benefits of cross- borrowing without the ad-hoc-ness of the old
system.

The expected outcome if this RFC is implemented is that Rust code is easier to
read and write, and is less off-putting to new-comers.

Data: there are 1557 occurrences of `&*` (and 3257 uses of `as_slice`) in the
Rust compiler and standard libraries (excluding tests) and 565 uses in Servo
(and 540 uses of `as_slice`). Note that this proposal would remove most, but not
all of these occurrences (because they occur where implicit coercions are not
allowed).

The fact that the coercion is limited to deref has several consequences for
reasoning about code [attrib: aturon]:

* You're generally quite aware of when you're using smart pointers and hence when
this coercion might come into play.
* The Deref trait will ultimately make the target type an associated -- hence,
*output* -- type. So this coercion would apply in only a single way to a given
value. That also protects somewhat against the "abuse" of Deref to implement
non-smart-pointer coercions.
* We already run code on the side for Deref in receiver position, so this doesn't
seem to open significant new cans of worms on that front.

Footnote: To expound on the benefit or otherwise of explicit borrows (`&`) and cross-
borrows (`&*`): there is benefit in being explicit about referencing and
dereferencing (i.e., not having auto-borrowing (other than for receivers) in
Rust) - it means that the programmer may reason (using only local knowledge)
about the performance characteristics of the call, potential aliasing, and (in
the case of mutable references) how objects can be mutated by function calls.
Furthermore, it helps the programmer keep in mind the distinction between
pointer and value types, which is essential in a systems language.

By contrast, requiring explicit `&*` (i.e., not implementing this RFC) has none
of these benefits - since we are simply converting from one kind of pointer to
another, the performance, aliasing, or mutability characteristics cannot change.
Nor does this blur the distinction between pointers and values. Focusing on the
mutation argument, eliding `&mut` at a call site does remove some information
about how an argument may be affected by a function call. However, this can only
happen when the argument is already some kind of mutable pointer and so this is
analogous to the case where the argument has `&mut` type before coercion (where
there is no indication at the call site of being `&mut`).

# Detailed design

If `U` is `&V` and `T` implements the `Deref<V>` trait, or `U` is `&mut V` and
`T` implements `DerefMut<V>`, then `T` may be implicitly coerced to `U` (for
example where a function's formal parameter has type `U` and the corresponding
actual parameter has type `T`). If the expression with type `T` is `e`, then the
dynamic semantics of the conversion are that `e` (in a coercible position) is
reduced to `e.deref()` (or equivalently, `&*e`).

Only a single dereference can be elided and only if there is a matching
reference. For example (using `*` as today),

Write today | Write with this proposal
------------|-------------------------
 *e         | *e
 &e         | &e
 &*e        | e
 &**e       | *e
 &&**e      | &*e

It would still be legal to write out the references and dereferences explicitly
(i.e., the proposed change is backwards compatible).

I believe `Box` does not actually implement `Deref`, either we implement `Deref`
for `Box` or we special case it.

Implicit re-borrows of borrowed references already occur (e.g., to convert `&mut
T` to `&T`).


# Drawbacks

Makes the ownership metaphor somewhat fuzzier [attrib: aturon]:

* `Box<T>` would coerce to `&T`
* `T` would not coerce to `&T`

You can make some sense of this by thinking of `Box` as a pointer. But, even so,
it's a coercion that silently introduces borrows of owned data. It seems
inconsistent to do data that happens to live on the heap via `Box`, but not
owned data on the stack. It means that when you write

    foo(some_data)

you may or may not be transferring ownership of `some_data` -- you have to know
all the types involved. By contrast, in today's Rust, you're always transferring
ownership. With `foo(&some_data)` you're passing ownership of a borrow. Since
ownership and borrowing are so central to Rust, I think consistency here is very
important; I think we should either auto-borrow *and* cross-borrow, or do
neither.

[Response] The difference is not between heap and stack allocation - a field
stored by value on the heap has the same behaviour as a value on the stack. So
the distinction is purely on whether an object is a value or a pointer, and
this is a distinction that must be at the forefront of any system programmer's
mind in any case.

I see two issues - one is that we treat T and Box<T> types differently, in
particular that an argument with no explicit ref/deref has different semantics
depending on the type. Two is that for Box<T> you need to know the type of the
formal argument to know if the actual is moved or borrowed. Both are certainly
disadvantages, but neither, I think, outweighs the benefit here.

For the first issue, the programmer must know this difference in any case -
today we use different syntax for calls (`&` vs `&*`), also for performance
reasons. For the second issue, I don't think it matters if you get it wrong - it
does not affect performance (unlike auto-borrowing, we never change the calling
semantics). Nor can it introduce bugs - if you assume a borrow happened, but
really a move did, the compiler will prevent you using the value after the call,
if you make the opposite assumption you just end up writing code which is too
conservative. Likewise, when reading code, it only matters if ownership is
transferred if the argument is used after the call, and if it is, then you can
assume that ownership was not transferred.

I feel that both the argument that we should not implicitly execute arbitrary
code and the ownership model argument are both placing principal too high
against practicality. I think this is a case where we can be a little less
principled for the sake of ergonomics.


## Other drawbacks

This change makes it less clear where referencing and dereferencing occur.

This change means treating `Box<T>` a bit more like a pointer, rather than a
value.

Allows arbitrary code to be executed implicitly.


# Alternatives

Allow cross-borrowing only in the immutable case. That is, only coerce if there
is a `Deref` implementation to an `&` pointer, not in the the case of `DerefMut`
to an `&mut` pointer. This has the advantage that any calls where an argument
may be mutated by the callee are indicated at the call site (although this is
not such a great advantage because of the `&mut self` case for receivers, and
the case where the argument has `&mut` type).

More drastic auto-borrowing - such as from any type to a borrowed reference. In
the case of smart-pointer types, this would not actually do what the programmer
expects, unless `Deref` was taken into account (i.e., the rules would be
somewhat confusing to account for doing either an `&e` or a `&*e` coercion,
depending on whether `e` implements `Deref` and on the type being coerced to).

Stick with the status quo.

# Unresolved questions

Whether to extend this mechanism to allow conversions from `String` to `&str`
and `Vec<T>` to `&[T]`. Since that really hinges on whether these types should
implement `Deref`, I think it is a separate issue. If we did want to take this
approach for solving the 'as_slice' problem, then this RFC is a necessary step.

There are some issues with exactly where and how coercions and other type
conversions happen in Rust. That is out of scope for this RFC and will be
addressed in a separate one, coming soon.
