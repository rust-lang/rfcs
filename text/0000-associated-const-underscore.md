- Feature Name: `associated_const_underscore`
- Start Date: 2023-11-12
- RFC PR: [rust-lang/rfcs#3527](https://github.com/rust-lang/rfcs/pull/3527)
- Rust Issue:

# Summary
[summary]: #summary

Allow `_` for the name of associated constants. This RFC builds on [RFC 2526]
which added support for free `const` items with the name `_`, but not associated
consts.

```rust
// RFC 2526 (stable in Rust 1.37)
const _: () = { /* ... */ };

impl Thing {
    // this RFC
    const _: () = { /* ... */ };
}
```

Constants named `_` are not nameable by other code and do not appear in
documentation, but are useful when macro-generated code must typecheck some
expression in the context of a specific choice of `Self`.

[RFC 2526]: https://github.com/rust-lang/rfcs/pull/2526

# Motivation
[motivation]: #motivation

The motivation is long, because understanding why this feature is worth having
requires understanding a fair bit of context about procedural macro techniques
and limitations. I have opted to provide this context in substantial depth.

Consider the standard library's `derive(Eq)` macro. The `core::cmp::Eq` trait
notionally contains no functions, but the following simple expansion would be
_wrong_ for its derive macro:

```rust
// input:
#[derive(Eq)]
pub struct Thing {
    field: Field,
}

// an incorrect expansion:
impl ::core::cmp::Eq for Thing {}
```

This expansion is incorrect because we want `derive(Eq)` to be responsible for
enforcing that all fields of the type have an `Eq` impl. If the type `Field`
above happens to be `f32` (which implements `PartialEq` but  not `Eq`), spitting
out a compilable `Eq` impl for `Thing` would be incorrect.

Here is what `derive(Eq)` expands to today, as of Rust 1.74:

```rust
impl ::core::cmp::Eq for Thing {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_receiver_is_total_eq(&self) -> () {
        let _: ::core::cmp::AssertParamIsEq<Field>;  // AssertParamIsEq<T: Eq + ?Sized>
    }
}
```

The `Eq` trait has secretly come with a `doc(hidden)` associated function for
the sole purpose that `derive(Eq)` can stick code in there to typecheck it.

This RFC proposes that `derive(Eq)` should generate its output as follows
instead, and the nonpublic `assert_receiver_is_total_eq` can be removed from the
trait.

```rust
impl ::core::cmp::Eq for Thing {}

impl Thing {
    const _: () = {
        let _: ::core::cmp::AssertParamIsEq<Field>;
    };
}
```

A number of alternative expansions come to mind using only existing syntax, none
of which are adequate to this use case.

1. **Just keeping the hidden function doesn't seem so bad.**

    From the perspective of the standard library's own `derive(Eq)`, sure. The
    trait and the derive macro are both defined by the same library. It's fair
    for the macro to be written against nonpublic internals of the trait. This
    is standard practice.

    But in a situation where the trait and macro are defined in independent
    crates, a nonpublic function for dumping typechecking code into is not a
    workable solution. This even affects `Eq`, because crates other than the
    standard library want to be able to provide custom derive macros for it.

    Consider what the [derive\_more] crate would need to do to support its own
    `derive(derive_more::Eq)`.

    [derive\_more]: https://github.com/JelteF/derive_more/issues/311

    ```rust
    #[derive(derive_more::Eq)]
    struct Thing {
        foo: Foo,
        #[derive_more(skip)]
        bar: Bar,
    }
    ```

    Code needs to go somewhere to check the `Foo: Eq` requirement. Reaching into
    private standard library internals is definitely not an intended way to
    accomplish this.

2. **So just make the dummy function public and stable?**

    My personal guess is that doing this to work around a language limitation
    would not be appealing to the standard library API team.

    Beyond aesthetic sensibility, here are some downsides to the dummy function
    approach.

    While `Eq` is not an auto-trait, the function approach is impossible to
    apply to auto-traits. Auto-traits (formerly known as opt-in builtin traits)
    are not allowed to contain trait functions. If we want derive macros such as
    in derive\_more to be able to produce implementations of `Unpin` or
    `UnwindSafe`, a different approach is required.

    Trait functions also have implications on dyn-safety. `Eq` is not dyn-safe
    already, but other marker traits are. In order to keep dummy functions from
    adding bloat to vtables, we'd want them bounded with `where Self: Sized`.
    This poses a footgun for the macro implementation which would need to know
    to _omit_ `where Self: Sized` on dummy functions within generated trait
    impls ([overconstraining]/[refining]) or risk getting false negatives.

    [overconstraining]: https://rust-lang.github.io/rfcs/2316-safe-unsafe-trait-methods.html
    [refining]: https://rust-lang.github.io/rfcs/3245-refined-impls.html

    ```rust
    trait DynSafeTrait {
        fn dummy_function_for_typechecking() where Self: Sized {}
    }

    // macro-generated impl
    impl<T: ?Sized> DynSafeTrait for Thing<T> {
        fn dummy_function_for_typechecking() {
            // We want to check this in a context where Thing<T> is not
            // necessarily Sized.
            let _: WhateverCheck<Thing<T>>;
        }
    }
    ```

    Finally, while the dummy function workaround has been discussed as applying
    to the case of marker traits like `Eq` which otherwise contain no functions
    that a macro could stick typechecking code into, consider that this RFC can
    be valuable more generally than that. In traits that contain a large,
    consistent set of signatures that a macro might want to implement all using
    the same codepath (think of [syn::visit::Visit] with a macro that forwards
    every visit function to a nested visitor), singling out a single one of
    those for the macro to stick its extra typechecking code into can be
    awkward. Would such traits also be expected to supply a `fn
    dummy_function_for_typechecking`?

    [syn::visit::Visit]: https://docs.rs/syn/latest/syn/visit/trait.Visit.html

3. **Just do everything through where-clauses.**

    This is a surprisingly feasible outside-the-box alternative.

    A suggestion frequently made is that macros like `derive(Eq)` on a struct
    like the following:

    ```rust
    pub struct Thing {
        field: Field,
    }
    ```

    should not expand to this kind of thing:

    ```rust
    impl ::core::cmp::Eq for Thing {
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<Field>;  // AssertParamIsEq<T: Eq + ?Sized>
        }
    }
    ```

    but rather to this:

    ```rust
    impl ::core::cmp::Eq for Thing
    where
        Field: Eq,
    {}
    ```

    In both cases, those generated trait impls compile successfully if `Field`
    implements `Eq`, and fail to compile if `Field` does not implement `Eq`.

    In the past this has been more problematic than today. Namely, until Rust
    1.59, this was liable to fail with _"private type in public interface"_
    errors.

    Remaining reasons this approach is not generally applicable are: _"overflow
    evaluating the requirement"_ errors in the case of co-recursive data
    structures, and _"type annotation needed"_ errors in certain cases involving
    lifetimes due to a longstanding compiler bug. See [dtolnay/syn#370].

    [dtolnay/syn#370]: https://github.com/dtolnay/syn/issues/370

    There's this less successful alternative using a where-clause with 0 trait
    bounds on a wacky array type:

    ```rust
    impl ::core::cmp::Eq for Thing
    where
        [(); {
            let _: ::core::cmp::AssertParamIsEq<Field>;
            0
        }]:
    {}
    ```

    This does not work when the type has generic parameters, even with
    `feature(generic_const_exprs)` enabled. The diagnostic pushes us toward
    using a const function. For this use case, if a const function were
    sufficient, there wouldn't be any use for a where-clause.

    ```console
    error: overly complex generic constant
      --> src/lib.rs:13:10
       |
    13 |       [(); {
       |  __________^
    14 | |         let _: ::core::cmp::AssertParamIsEq<Field<'a, T>>;
    15 | |         0
    16 | |     }]:
       | |_____^ blocks are not supported in generic constants
       |
       = help: consider moving this anonymous constant into a `const` function
       = note: this operation may be supported in the future
   ```

4. **Is free const underscore not sufficient?**

    Let's go through a series of decreasingly naïve ways that one might try to
    implement a correct `derive(Eq)` using free const underscore, without
    associated const underscore. If "implied bounds" are already on your mind at
    this point, you have predicted where this is heading.

    With this as the macro input:

    ```rust
    pub struct Thing {
        field: Field,
    }
    ```

    One might expect that we can emit:

    ```rust
    impl ::core::cmp::Eq for Thing {}

    const _: () = {
        let _: ::core::cmp::AssertParamIsEq<Field>;
    };
    ```

    and indeed this works. But only because generic parameters are not involved.
    Let's try it with generics:

    ```rust
    pub struct Thing<T> {
        field: Field<T>,
    }
    ```

    Today in stable Rust, `const` cannot be generic (there is an experimental
    implementation in the compiler, but no RFC yet; see [rust#113521]). Instead
    we'll use a function to introduce appropriately bounded generic parameters.
    But we also keep a surrounding underscore constant to avoid needing to pick
    a unique function name that won't conflict with other uses of `derive(Eq)`
    in the same scope.

    [rust#113521]: https://github.com/rust-lang/rust/issues/113521

    ```rust
    const _: () = {
        fn assert_fields_are_total_eq<T: ::core::cmp::Eq>() {
            let _: ::core::cmp::AssertParamIsEq<Field<T>>;
        }
    };
    ```

    So far so good, but let's try the same thing with lifetimes in the picture.

    ```rust
    type Field<'a, T> = &'a mut T;

    // #[derive(Eq)]
    pub struct Thing<'a, T> {
        field: Field<'a, T>,
    }

    const _: () = {
        fn assert_fields_are_total_eq<'a, T: ::core::cmp::Eq>() {
            let _: ::core::cmp::AssertParamIsEq<Field<'a, T>>;
        }
    };
    ```

    This fails to compile because of a missing `T: 'a` implied bound. The
    implied bound originates from code that is not visible to the macro
    implementation, so it is hopeless for the macro to produce a correct
    explicit bound in this situation.

    ```console
    error[E0309]: the parameter type `T` may not live long enough
     --> src/lib.rs:9:16
      |
    8 |     fn assert_fields_are_total_eq<'a, T: ::core::cmp::Eq>() {
      |                                   -- the parameter type `T` must be valid for the lifetime `'a` as defined here...
    9 |         let _: ::core::cmp::AssertParamIsEq<Field<'a, T>>;
      |                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ ...so that the type `T` will meet its required lifetime bounds
      |
    help: consider adding an explicit lifetime bound
      |
    8 |     fn assert_fields_are_total_eq<'a, T: ::core::cmp::Eq + 'a>() {
      |                                                          ++++
    ```

    Instead of an explicit bound, we can try to arrange for a suitable implied
    bound to get put in, by making an unused argument of type `Self` appear in
    scope.

    ```rust
    const _: () = {
        fn assert_fields_are_total_eq<'a, T: ::core::cmp::Eq>(_: &Thing<'a, T>) {
            let _: ::core::cmp::AssertParamIsEq<Field<'a, T>>;
        }
    };
    ```

    This works. Though notice we can't exactly use `Self`; the type needs to be
    spelled out. Also if `Self` appears in the type of one of the fields, that
    would also need to be substituted with the right spelled-out type name.

    ```rust
    pub struct Thing<T> {
        buf: <Self as Buffered<T>>::Buf,
    }

    const _: () = {
        fn assert_fields_are_total_eq<T: ::core::cmp::Eq>(_: &Thing<T>) {
            let _: ::core::cmp::AssertParamIsEq<<Thing<T> as Buffered<T>>::Buf>;
         }                                       ^^^^^^^^
    };
    ```

    "Replacing `Self`" like this looks simple but is fiendish to handle
    correctly. It cannot be done correctly on the token level because different
    appearances of `Self` in a type can refer to different types. In the
    following example, `Self` is used twice within the definition of `Struct`
    and substituting both with `Struct` would break the meaning of the program.

    ```rust
    pub struct Struct {
        pub header: [u8; {
            struct Nested(Option<Box<Self>>);
            Self::K + mem::size_of::<Nested>()
        }],
        pub rest: [u8],
    }

    impl Struct {
        const K: usize = 1;
    }

    fn main() {
        let _: fn(&Struct) -> &[u8; 9] = |s| &s.header;
    }
    ```

    The `async-trait` crate has [172 lines of logic][async-trait] dedicated to
    "replacing Self". The `serde_derive` crate has [292 lines][serde_derive].
    Async-trait has had at least 13 bugs involving the replacement of `Self`,
    affecting real-world non-contrived code. This is not a thing that typical
    procedural macros should be expected to implement.

    <!--
    https://github.com/dtolnay/async-trait/issues/9
        https://github.com/dtolnay/async-trait/pull/12
    https://github.com/dtolnay/async-trait/issues/31
        https://github.com/dtolnay/async-trait/pull/32
    https://github.com/dtolnay/async-trait/pull/44
    https://github.com/dtolnay/async-trait/issues/53
        https://github.com/dtolnay/async-trait/pull/54
        https://github.com/dtolnay/async-trait/pull/55
    https://github.com/dtolnay/async-trait/issues/61
    https://github.com/dtolnay/async-trait/issues/73
        https://github.com/dtolnay/async-trait/pull/74
    https://github.com/dtolnay/async-trait/issues/81
        https://github.com/dtolnay/async-trait/pull/82
    https://github.com/dtolnay/async-trait/issues/87
        https://github.com/dtolnay/async-trait/pull/88
    https://github.com/dtolnay/async-trait/issues/92
        https://github.com/dtolnay/async-trait/pull/100
        https://github.com/dtolnay/async-trait/pull/124
    https://github.com/dtolnay/async-trait/pull/102
    https://github.com/dtolnay/async-trait/pull/103
    -->

    [async-trait]: https://github.com/dtolnay/async-trait/blob/0.1.74/src/receiver.rs
    [serde_derive]: https://github.com/serde-rs/serde/blob/v1.0.192/serde_derive/src/internals/receiver.rs

    Let's try avoiding needing to handle `Self` replacement by moving the
    typechecking code into an `impl` block.

    ```rust
    // #[derive(Eq)]
    pub struct Thing {
        field: Field,
    }

    impl Thing {
        #[doc(hidden)]
        #[allow(dead_code)]
        #[coverage(off)]
        fn __assert_fields_are_total_eq() {
            let _: ::core::cmp::AssertParamIsEq<Field>;
        }
    }
    ```

    For the library ecosystem, this isn't terrible, though needing to pick a
    name for the hidden function that won't conflict with other macro-generated
    code is annoying. Consider the case where a macro might be applied multiple
    times to the same data structure, such as to generate `AsRef<First>` and
    `AsRef<Second>`.

    For the standard library's derive macros I think this expansion is not
    viable. The reason is we'd have no way to mark that generated associated
    function as being a standard library implementation detail (`#[unstable]`)
    as we would ordinarily want to do.

    Here is a way to work around both issues: eliminating conflicts between
    different expansions, and avoiding inserting junk APIs into the caller's
    code.

    ```rust
    impl ::core::cmp::Eq for Thing {}

    const _: () = {
        trait __AssertFieldsAreTotalEq {
            fn assert_fields_are_total_eq();
        }
        impl __AssertFieldsAreTotalEq for Thing {
            fn assert_fields_are_total_eq() {
                let _: ::core::cmp::AssertParamIsEq<Field>;
            }
        }
    };
    ```

    For a library containing `pub struct Thing { field: i32 }` and the above
    `const _`, this produces an rlib that is 7.2 KB, containing a symbol for that
    `assert_fields_are_total_eq` function.

    ```console
    $ llvm-dwarfdump target/debug/librepro.rlib

    DW_TAG_namespace
      DW_AT_name  ("repro")

      DW_TAG_namespace
        DW_AT_name  ("_")

        DW_TAG_namespace
          DW_AT_name  ("{impl#0}")

          DW_TAG_subprogram
            DW_AT_low_pc  (0x0000000000000000)
            DW_AT_high_pc  (0x0000000000000001)
            DW_AT_frame_base  (DW_OP_reg7 RSP)
            DW_AT_linkage_name  ("_ZN67_$LT$repro..Thing$u20$as$u20$repro.._..__AssertFieldsAreTotalEq$GT$26assert_fields_are_total_eq17hc74c403364f7baa6E")
            DW_AT_name  ("assert_fields_are_total_eq")
            DW_AT_decl_file  ("src/lib.rs")
            DW_AT_decl_line  (12)
            DW_AT_external  (true)
    ```

    We can make an approach that is cheaper to compile by changing the
    `__AssertFieldsAreTotalEq` trait's contents from a fn to a const. This way
    there is no longer a need to compile the function's body to machine code;
    just type-check it. This reduces the size of librepro.rlib by 35% to 4.7 KB.

    ```rust
    impl ::core::cmp::Eq for Thing {}

    const _: () = {
        trait __AssertFieldsAreTotalEq {
            const ASSERT_FIELDS_ARE_TOTAL_EQ: ();
        }
        impl __AssertFieldsAreTotalEq for Thing {
            const ASSERT_FIELDS_ARE_TOTAL_EQ: () = {
                let _: ::core::cmp::AssertParamIsEq<Field>;
            };
        }
    };
    ```

    As far as I know, this final expansion is able to accomplish all technical
    objectives. I considered making a PR to make `derive(Eq)` take this
    approach, but if possible, going straight to the associated const underscore
    proposed by this RFC would be preferable.

    ```rust
    impl ::core::cmp::Eq for Thing {}

    impl Thing {
        const _: () = {
            let _: ::core::cmp::AssertParamIsEq<Field>;
        };
    }
    ```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

https://doc.rust-lang.org/1.73.0/reference/items/constant-items.html#unnamed-constant

```diff
- Unlike an associated constant, a free constant may be unnamed by using an
+ A free constant or associated constant may be unnamed by using an
  underscore instead of the name. For example:
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The implementation pretty much follows the implementation of free const
underscore, which has been working well.

The following details are called out as being worth testing:

1. Unlike ordinary associated constants, multiple associated const underscore
    are permitted to co-exist on the same Self type.

    ```rust
    struct Struct<T>(T);

    impl<T> Struct<T> {
        const _: () = ();

        const _: i16 = 0;  // not a conflict
    }

    impl Struct<i16> {
        const _: () = ();  // not a conflict
    }
    ```

2. Although associated const underscore does not add any externally accessible
    API to a type, a visibility specification is still allowed on it. As with
    any other associated constant, of the 3 visibilities {receiver's visibility,
    constant's visibility, constant's type's visibility}, you get a warning if
    the constant's type's visibility is the strictly lowest one.

    ```rust
    pub struct Public;

    struct Private;

    impl Public {
        pub const _: Private = Private;  // warn(private_interfaces)
    }

    impl Public {
        const _: Private = Private;  // no warning
    }

    impl Private {
        pub const _: Private = Private;  // no warning
    }
    ```

3. The `Self` type of the impl must be local to the crate containing the impl.

    ```rust
    impl std::thread::Thread {
        const _: () = {};  // not allowed
    }

    struct Local;
    impl &Local {
        const _: () = {};  // although &T is #[fundamental], this is not allowed
    }
    ```

4. This RFC does not propose const underscore for inclusion as a trait item.

    ```rust
    trait Trait {
        const _: ();  // not allowed
    }
    ```

5. This RFC does not propose const underscore inside trait impls.

    ```rust
    trait Trait {}

    impl Trait for Type {
        const _: () = {};  // not allowed
    }
    ```

6. They are allowed syntactically but not semantically.

    ```rust
    struct Struct;

    trait Trait {
        #[cfg(any())]
        const _: () = {};
    }

    impl Trait for Struct {
        #[cfg(any())]
        const _: () = {};
    }
    ```

    This code already works on stable since Rust 1.43 (https://github.com/rust-lang/rust/pull/69194).

7. The underscore const's value is evaluated in exactly the situations that an
    ordinary named associated constant would be evaluated. Named associated
    constants are evaluated when accessed. Underscore associated constants
    cannot be accessed, so are never evaluated &mdash; only typechecked.

    ```rust
    pub struct Unit;

    impl Unit {
        const K: () = assert!(false);  // no error
        const _: () = assert!(false);  // no error
    }

    pub struct Generic<T>(T);

    impl<T> Generic<T> {
        const K: () = assert!(mem::size_of::<T>() % 2 == 0);  // no error
        const _: () = assert!(mem::size_of::<T>() % 2 == 0);  // no error
    }

    fn main() {
        let _ = Unit;  // no error
        let _ = Generic([0u8; 3]);  // no error

        let _ = Unit::K;  // error
        let _ = Generic::<[u8; 3]>::K;  // error
    }
    ```

8. Underscore constants are not dead code, despite not being referenced.

    ```rust
    #![deny(dead_code)]

    pub struct Struct;

    const _: () = {
        let _ = Struct;
    };

    impl Struct {
        const _: () = {
            let _ = Struct;
        };

        const _: () = {
            struct Unused;  // error: dead code
        };
    }
    ```

# Drawbacks
[drawbacks]: #drawbacks

None identified. This is a logical combination of 2 language features that the
Rust Reference needs to go out of its way to identify as being disallowed.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The do-nothing alternative is worth examining for the following reason: **unlike
RFC 2526 (free const underscore), this RFC does not add expressiveness.**

That previous RFC was exceedingly well motivated by use cases that were
impossible to solve prior to the language change. Some examples include
[inventory#8] and [static\_assertions].

[inventory#8]: https://github.com/dtolnay/inventory/issues/8
[static\_assertions]: https://github.com/rust-lang/rust/issues/54912#issuecomment-480594120

Meanwhile this RFC only makes a use case easier to express than it was before,
by removing a spurious limitation of 2 language features not working together
(associated constants and const underscore). As demonstrated near the bottom of
the Motivation, the following proposed use of associated const underscore:

```rust
impl<Generics> SelfType {
    const _: Something = {/* ... */};
}
```

is substantially equivalent to the following already legal syntax:

```rust
const _: () = {
    trait __SomeUniqueEnoughName {
        const K: Something;
    }
    impl<Generics> __SomeUniqueEnoughName for SelfType {
        const K: Something = {/* ... */};
    }
};
```

The former is something that I think would be great to convert the standard
library's `derive(Eq)` to as soon as available. The latter is something that
would be a hard sell despite advantages over the current less-verbose expansion
of `derive(Eq)`.

### Alternative: eagerly evaluate all possible associated underscore constants when a type is instantiated

As described by **@programmerjake** in https://github.com/rust-lang/rfcs/pull/3527#issuecomment-1807591083.

In discussing the do-nothing alternative, I wrote that my RFC as currently
written does not add expressiveness. Jacob's alternative _does_ add
expressiveness. It gives a way to express invariants on the instantiations of a
generic type, with those invariants being eagerly checked any time the type is
mentioned with enough generic parameters provided. This kind of checking cannot
be implemented in Rust today.

```rust
pub struct Struct<T, U>(T, U);

impl<T, U> Struct<T, U> {
    const _: () = assert!(mem::size_of::<T>() == 8, "invariant A");

    const _: () = assert!(mem::size_of::<T>() == mem::size_of::<U>(), "invariant B");
}

pub fn f<T, U>(s: Struct<T, U>) {}  // no error

pub fn g<U>(s: Struct<u8, U>) {}  // ERROR (invariant A)

pub fn h(s: Struct<usize, i32>) {}  // ERROR (invariant B)
```

This alternative remains compatible with what the `derive(Eq)` macro needs.
`derive(Eq)` would never need to generate a constant that fails to evaluate. It
would only generate constants that potentially fail to type-check. Evaluating
the constants makes no difference.

Eager evaluation of associated underscore constants would have some limitations
of what constants it's able to trigger evaluation of. One interesting example in
the ecosystem I know about through `trybuild` is [`objc2::Encode`] which
contains the following arrangement.

[`objc2::Encode`]: https://github.com/madsmtm/objc2/blob/objc-0.4.1/crates/objc2/src/encode/mod.rs

```rust
pub unsafe trait Encode {
    const ENCODING: Encoding;
}

// SAFETY: requires T has same layout as Option<T>
pub unsafe trait OptionEncode {}

unsafe impl<T: Encode + OptionEncode> Encode for Option<T> {
    const ENCODING: Encoding = {
        if mem::size_of::<T>() != mem::size_of::<Option<T>>() {
            panic!("invalid OptionEncode + Encode implementation");
        }
        T::ENCODING
    };
}
```

When I last thought about this crate for about an hour some months ago, I was
not able to come up with any way of rewriting this impl whereby `cargo check`
would report incorrect impls of `OptionEncode`, not even in the cases where
`<Option<BadT> as Encoding>::ENCODING` is mentioned somewhere in the program.
Only `cargo build` would catch that (refer to [RFC 3477]). I don't see a way
that eagerly evaluated associated underscore constant would help, either. The
use case isn't something that falls obviously in scope for this RFC to address,
but it's mentioned here only to convey that the underscore associated const
eager evaluation alternative is still not an associated constant
eager-evaluation panacea in general.

[RFC 3477]: https://github.com/rust-lang/rfcs/pull/3477

### Alternative: const blocks as where-clauses

As described by **@JulianKnodt** in https://github.com/rust-lang/lang-team/issues/163.

The previous alternative's "invariants on the instantiations of a generic type"
does not sound like a job for an associated constant. It sounds like a job for a
where-clause.

```rust
pub struct Struct<T, U>(T, U)
where
    const { mem::size_of::<T>() == 8 };
```

Adapting this to `derive(Eq)` might look as follows.

```rust
// input:
#[derive(Eq)]
pub struct Thing<'a, T> {
    field: Field<'a, T>,
}

// expansion:
impl<'a, T> ::core::cmp::Eq for Thing<'a, T>
where
    T: ::core::cmp::Eq,
    const {
        let _: ::core::cmp::AssertParamIsEq<Field<'a, T>>;
        true
    },
{}
```

### Alternative: private functions inside trait impls, not declared by the trait

Brianstormed by **@scottmcm** in https://github.com/rust-lang/rfcs/pull/3527#issuecomment-1817352170.

```rust
// input:
pub struct Thing<'a, T> {
    field: Field<'a, T>,
}

// expansion:
impl<'a, T> ::core::cmp::Eq for Thing<'a, T>
where
    T: ::core::cmp::Eq,
{
    // even though the following function is not a member of core::cmp::Eq
    #[coverage(off)]
    priv fn _assert_fields_are_total_eq() {
        let _: ::core::cmp::AssertParamIsEq<Field<'a, T>>;
    }
}
```

The priv function is visible only inside that one impl block. There is no
conflict with other impl blocks, which might contain their own priv function
with the same name, similar to how there is no conflict between multiple
associated `const _` on the same type in this RFC.

No need for `doc(hidden)` because priv functions would be treated the same as
other non-pub things and only included in docs when `--document-private-items`
is passed to rustdoc.

Being a function means the fact that it's not ever evaluated seem more expected,
compared against accomplishing the same thing via associated const.

Scott proposes that allowing trait impls to hold helper functions that don't
have to go in a different inherent impl block would be a useful feature in its
own right, beyond those use cases which overlap with associated underscore
constant. (But also a bigger one, because anything that deals in visibility is
complicated.)

### Alternative: anonymous modules

Brainstormed by **@nikomatsakis** in https://github.com/rust-lang/rfcs/pull/3527#issuecomment-1817497844.

TODO: flesh this out. I can see how modules would supplant some uses of free
underscore const, but not the `derive(Eq)` use case.

# Prior art
[prior-art]: #prior-art

None identified.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- [ ] When does associated const code get run? Eagerly at type definition?
    [When substituting concrete types into generic arguments?][substituting]
    Never?

[substituting]: https://github.com/rust-lang/rfcs/pull/3527#issuecomment-1807591083

# Future possibilities
[future-possibilities]: #future-possibilities

1. Consider lifting the restriction that the `Self` type of the impl must be
    local.

    Associated const underscore does not add any externally accessible API to a
    type, so I wonder whether there is a strong rationale for limiting it to
    local types. I believe I have had cases that would have benefited from
    having associated const underscore on an arbitrary type, but I have not
    aggregated the justification for supporting this. I will consider RFC-ing
    this separately with a strong justification.

2. Consider allowing the expression part of underscore const to be omitted,
    resulting in a way to type-check only the type.

    ```rust
    impl ::core::cmp::Eq for Thing {}

    impl Thing {
        const _: ::core::cmp::AssertParamIsEq<Field>;
    }
    ```

Separately, refer to the "Possible future work" section of the stabilization
proposal for the original const underscore, of which this RFC is one part.
https://github.com/rust-lang/rust/pull/61347#issuecomment-497533585
