- Start Date: 2014-09-27
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)


# Summary and motivation

 * Rename `Option` and its variants to bring them into closer alignment with
   `bool` and `Result`;

 * Generalize assertions, by use of a trait, from `bool`s to also (née)
   `Option`s and `Result`s, allowing them to evaluate to a value;

 * Make the `assert!` macro also use this trait.

The result is a more compositional, pleasant, and informative style of
assertions.


# Design and discussion

## Rename `Option`

Rust uses the `Option` type to represent "nullable" or "optional" values:

    enum Option<T> {
        Some(T),
        None
    }

There is precedent for at least four different names for this type: `Option`
(ML), `Optional` (Java, C++), `Maybe` (Haskell), and `Nullable` (C#). The
`Option` name is the only one of these which doesn't quite say what it means.
The word "option" brings to mind program flags, configuration settings, or open
possibilities - not an optional value. One might ask whether it is worth
sacrificing clarity for two characters. The name we select, however, is not
`Optional`, but `Maybe`: partly because it is the shortest intuitive name, but
mostly because, if one thinks upon common responses to the word "maybe", then a
fantastic choice of variant names also presents itself:

    enum Maybe<T> {
        Yes(T),
        No
    }

Here we can see it in action:

    match my_map.find(name) {
        Yes(entry) => display(entry),
        No => println!("Not found!")
    }

> "So did you find the entry?"
>
> "Well, maybe."
>
> "Yes or no?"
>
> "Yes, and here it is."; or "Sorry, no."

This nicely aligns the variant names for our three major two-variant types:

`bool`  | `Maybe<T>` | `Result<T, E>`
------- | ---------- | -------------
`true`  | `Yes(T)`   | `Ok(T)`
`false` | `No`       | `Err(E)`

Now all of these reflect a similar dichotomy between a positive/affirmative and
a negative result. This makes sense, as they are all used for similar purposes.
The only difference, that determines which is appropriate to use in a particular
case, is whether there is additional "data" (evidence) associated with success
and/or failure, or if there is only a single possible success, respectively
failure condition. If both success and failure can only mean one thing: `bool`;
if success has additional information associated with it: `Maybe<T>`; if failure
also does: `Result<T, E>`.


## `Assert` trait

Building upon this, we generalize the concept of asserting a successful result
when we believe that failure should be impossible. Let us define the traits:

    // for use by `assert!`
    trait AssertMsg {
        type Value;
        fn assert_msg(self, msg: &'static str, file_line: &(&'static str, uint)) -> Value;
        fn assert_fmt(self, fmt: &fmt::Arguments, file_line: &(&'static str, uint)) -> Value;
    }

    // for use directly
    trait Assert: AssertMsg {
        fn assert(self) -> Value {
            self.assert_msg("assertion failed", &(file!(), line!()))
        }
    }

(The reason why there are two traits rather than one will be explained below.)

And the corresponding implementations:

    impl AssertMsg for bool {
        type Value = ();
        fn assert_msg(self, msg: &'static str, file_line: &(&'static str, uint)) {
            if !self { rt::begin_unwind(msg, file_line) }
        }
        fn assert_fmt(self, fmt: &fmt::Arguments, file_line: &(&'static str, uint)) {
            if !self { rt::begin_unwind_fmt(fmt, file_line) }
        }
    }

    impl Assert for bool { }

    impl<T> AssertMsg for Maybe<T> {
        type Value = T;
        fn assert_msg(self, msg: &'static str, file_line: &(&'static str, uint)) -> T {
            match self {
                Yes(val) => val,
                No       => rt::begin_unwind(msg, file_line)
            }
        }
        fn assert_fmt(self, fmt: &fmt::Arguments, file_line: &(&'static str, uint)) -> T {
            match self {
                Yes(val) => val,
                No       => rt::begin_unwind_fmt(fmt, file_line)
            }
        }
    }

    impl<T> Assert for Maybe<T> { }

    impl<T, E> AssertMsg for Result<T, E> {
        type Value = T;
        fn assert_msg(self, msg: &'static str, file_line: &(&'static str, uint)) -> T {
            match self {
                Ok(val) => val,
                Err(_)  => rt::begin_unwind(msg, file_line)
            }
        }
        fn assert_fmt(self, fmt: &fmt::Arguments, file_line: &(&'static str, uint)) -> T {
            match self {
                Ok(val) => val,
                Err(_)  => rt::begin_unwind_fmt(fmt, file_line)
            }
        }
    }

    impl<T, E: Any + Send> Assert for Result<T, E> {
        fn assert(self) -> T {
            match self {
                Ok(val)      => val,
                Err(err_val) => rt::begin_unwind(err_val, &(file!(), line!()))
            }
        }
    }

We also redefine the `assert!` macro in terms of the above trait:

    macro_rules! assert(
        ($cond:expr) => ({
            static _FILE_LINE: (&'static str, uint) = (file!(), line!());
            $cond.assert_msg(concat!("assertion failed: ", stringify!($cond)), &_FILE_LINE)
        });
        ($cond:expr, $($arg:expr),+) => ({
            static _FILE_LINE: (&'static str, uint) = (file!(), line!());
            format_args!(|fmt| $cond.assert_fmt(fmt, &_FILE_LINE), $($arg),+)
        });
    )

(`debug_assert!` remains unchanged: as it may be compiled out, it must have type
`()`.)

This is a somewhat maximalist design arising from the following constraints:

 * We do not wish to incur any regressions relative to the existing `assert!`:

 * Failing `assert!`s should have accurate source location information, and

 * They should print the condition which failed, or

 * It should be possible to specify a custom formatted message instead, and

 * It should not allocate.

Additionally, as a nice touch, if the `assert()` method (rather than the macro)
is used on a `Result`, we would like to pass the value from the `Err` variant
directly on to `fail!()` (or `begin_unwind`, as the case may be, and is). The
very few cases in the existing `rust` codebase where an additional *non-string*
argument is passed to `assert!` to invoke `fail!` with nearly all have the form
`assert!(res.is_ok(), res.unwrap_err())`: this use case is now directly
supported as simply `res.assert()`. This is why we need to have separate
`AssertMsg` and `Assert` traits, instead of `assert()` being an additional
method of the same trait: for passing it on to task failure we must require
`E: Send + Any`, but we do *not* wish to incur this restriction on uses of the
`assert!` macro.

Satisfying these requirements involved duplicating significant swathes from
[the implementation of `fail!()`](http://doc.rust-lang.org/std/macro.fail!.html).
This could be accepted as-is, or it could potentially be resolved by instead
implementing `fail!()` in terms of `assert!(false)` (with some likely
complications around making it be of type `!`). The design could also
potentially be simplified by dropping one or more of the listed requirements,
and of course, it is also possible that a more ingenious design exists than the
one which the author has managed to come up with.

As a result of these changes, messages for failing asserts actually become *more
useful*. Instead of:

    let entry = map.find(key);
    assert!(entry.is_some());
    forbinate(entry);

One would write:

    forbinate(assert!(map.find(key)));

The assert message will now directly mention what operation actually failed
(`map.find(key)`), rather than a disconnected test about the state of a
temporary variable (`entry.is_some()`).

The original design by @aturon also proposed an `assert_err()` method on
`Result` which could be used to assert that an error *had* occurred. Instead, we
would suggest adding a method such as:

    // Other possible names: inverse, flipped, reversed, swapped, ...
    fn negated(self: Result<T, E>) -> Result<E, T> {
        match self {
            Ok(x)  => Err(x),
            Err(x) => Ok(x)
        }
    }

Instead of `res.assert_err()`, one would write `res.negated().assert()`. We feel
this is a more general and orthogonal way of satisfying this uncommon use case.

Many people feel that `assert` returning a value would be strange and
disorienting for practitioners of existing mainstream languages, where it does
not. While this concern is a noble one, we feel that this line of thinking is
fundamentally misguided. On this basis, we should also complain that constructs
such as `if`..`else`, `match`, and indeed plain old blocks of code `{ ... }`
evaluating to a value would be too confusing for those coming from other
languages. Instead, Rust takes the approach of generalizing these constructs,
allowing them to be used in the accustomed imperative manner, but also building
out the path to a more ergonomic and compositional style of writing code. We
feel that this approach is the correct one, and follow it here.

(Note: The variants for each datatype should of course be in ascending order:
first `No`, then `Yes`; but the reverse was felt to be more advantageous for the
purpose of exposition.)


## Alternatives

Instead of a named `assert` method and `assert!` macro, a postfix `!` operator
has also been proposed. The author considers this to be neither wise nor
worthwhile.

The motivation for doing this would be to be able to adopt a uniform convention
that functions should always push their preconditions out as an error value in
the result type, with the assertion that preconditions were satisfied also being
moved out to the caller, and the lightweight `!` operator reducing the syntactic
burden of doing so. This, once again, is a noble endeavour, but the price is
high:

 * By making it so lightweight, we would also encourage people to use it
   frequently. Back when Rust had single-character sigils `~` and `@` for
   allocating boxes, the tendency was that people would sprinkle these in their
   code without much thought until the compiler ceased its complaints. The same
   phenomenon would inevitably manifest in this case. Just as `~` is not only
   more convenient, but also *less self-explanatory* than `box`, likewise `!` is
   more convenient *and* less self-explanatory than `assert!`, and people would
   use it without being fully aware of its connotations. Instead of `!` being
   the assertion operator, it would be the "make the compiler shut up" operator.
   Where we had sought to encourage functions to propagate error conditions to
   their callers, we will have accomplished the opposite: adding an `!` to
   silence the error will be easier and more obvious than propagating it, and so
   that is what people will tend to do. (This is the case *even if* we also have
   an `?` operator for propagating: the latter also requires changing the
   function's result type and all other code paths which return a value, and is
   thus far more invasive.)

 * To use the `!` syntax for asserting, we must take it away from macros.
   However, macros are very common in Rust code, and the current `foo!()`
   syntax is the most appealing syntax which they could possibly have while
   still clearly distinguishing them from normal code. Any alternative use of
   the `!` symbol should have benefits large enough to outweigh this drawback.
   As suggested above, that is not the case.

In sum, an `!` operator would have considerable costs both syntactically and
practically, in exchange for modest benefits, and may even end up being
counterproductive to its stated goal. We would much rather live with the fact
that API authors should use their best judgment to decide whether to assert
their preconditions, to propagate them to the caller, or to provide both
options. And if one wishes to make an assertion, she should write `assert`.
