- Feature Name: `static_in_pattern`
- Start Date: 2022-08-17
- RFC PR: [rust-lang/rfcs#3305](https://github.com/rust-lang/rfcs/pull/3305)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

Allow referencing non-`mut` `static`s in pattern matches wherever referencing a `const` of the same type would be allowed.

# Motivation

[motivation]: #motivation

Rust pattern matches compare a scrutinee against compile-time information. Rust generally doesn't allow patterns to depend on runtime information; that is relegated to match guards. However, there is a category between "compile-time", when `rustc` runs, and "runtime", when Rust code runs. Some information a Rust program relies on may be determined at link-time, or by the target operating system, or before `main()` by the C runtime. Rust currently prevents patterns from depending on such information. Specifically, Rust patterns cannot reference statics from `extern` blocks.

I encountered this restriction while trying to port the Rust standard library to [cosmopolitan libc](https://justine.lol/cosmopolitan/index.html). Cosmopolitan provides an API that mostly matches POSIX, with one major exception: constants like `ENOSYS` and `EINVAL`, which on most platforms are defined as C `#define`s (equivalent to Rust `const`s), are instead provided as C `const`s (equivalent to Rust non-`mut` `static`s).

```rust
// libc crate

cfg_if! {
    if #[cfg(target_env = "cosmopolitan")] {
        extern "C" {
            pub static EINVAL: i32;
            pub static ENOSYS: i32;
            pub static ENOENT: i32;
        }
    } else {
        pub const EINVAL: i32 = 42;
        pub const ENOSYS: i32 = 43;
        pub const ENOENT: i32 = 44;
    }
}

// stdlib code

use libc::*;

fn process_error(error_code: i32) {
    match error_code {
        // Compiler throws error EO530 on Cosmopolitan,
        // because `static`s can't be used in patterns, only `const`s
        EINVAL => do_stuff(),
        ENOSYS => panic!("oh noes"),
        ENOENT => make_it_work(),
        _ => do_different_stuff(),
    }
}
```

Because Rust patterns don't support statics, all the `match` expressions in the standard library that refer to POSIX constants would currently need to be rewritten to accommodate Cosmopolitan.

```rust
// stdlib code adapted for cosmopolitan

use libc::*;

fn process_error(error_code: i32) {
    if error_code == EINVAL {
        do_stuff();
    } else if error_code == ENOSYS {
        panic!("oh noes");
    } else if error_code == ENOENT {
        make_it_work();
    } else {
        do_different_stuff();
    }
}
```

Needless to say, this is unlikely to ever be upstreamed. Allowing statics in patterns would solve this use-case much more cleanly.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

Rust patterns can refer to constants.

```rust
const EVERYTHING: i32 = 42;

fn foo(scrutinee: i32) {
    match scrutinee {
        EVERYTHING => println!("have all of it"),
        _ => println!("need moar"),
    }
}
```

With this feature, they can refer to statics as well.

```rust
static EVERYTHING: i32 = 42;

fn foo(scrutinee: i32) {
    match scrutinee {
        EVERYTHING => println!("have all of it"),
        _ => println!("need moar"),
    }
}
```

Mutable statics are not allowed, however. Patterns can't reference information that can change at runtime, and also can't be `unsafe`.

```rust

static mut EVERYTHING: i32 = 42;

fn foo(scrutinee: i32) {
    match scrutinee {
        // ERROR can't refer to mutable statics in patterns
        /* EVERYTHING => println!("have all of it"), */
        _ => println!("need moar"),
    }
}
```

Statics from `extern` blocks are allowed, but they must be marked as trusted using the (not-yet-implemented) [trusted external statics](https://github.com/rust-lang/lang-team/issues/149) feature.

```rust
extern "C" {
    #[unsafe(trusted_extern)]
    static EVERYTHING: i32;
}

fn foo(scrutinee: i32) {
    match scrutinee {
        EVERYTHING => println!("have all of it"),
        _ => println!("need moar"),
    }
}
```

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

For a static to be eligible for use in a pattern, it must:

- not be marked `mut`
- not be marked `#[thread_local]`
- not come from an extern block, unless it is marked as safe to use with the [trusted external statics](https://github.com/rust-lang/lang-team/issues/149) feature
- have a type that satisfies the structural match rules, as described in [RFC 1445](1445-restrict-constants-in-patterns.md), but without any allowances for backward compatibility like there are for consts (e.g., floating point numbers in patterns) . These rules exclude all statics with interior mutability.

Static patterns match exactly when a const pattern with a const of the same type and value would match.

The values of statics are treated as opaque for reachability and exhaustiveness analysis.

```rust
static TRUE: bool = true;
static FALSE: bool = false;

fn foo(scrutinee: bool) {
    match scrutinee {
        TRUE | FALSE => println!("bar"),

        // The compiler will throw an error if you remove this branch;
        // it is not allowed to look into the values of the statics
        // to determine that it is unreachable.
        _ => println!("baz"),
    }
}
```

As an exception, when all safe values of a type are structurally equal, the compiler is allowed to see that the match will always succeed.

```rust
// Not all `&()` are bitwise equal,
// but they are structurally equal,
// that is what matters.
static ONE_TRUE_VALUE: &() = &();

fn foo(scrutinee: &()) {
    match scrutinee {
        ONE_TRUE_VALUE => println!("only one branch"),
        // No need for a wildcard.
        // The above match always succeeds.
    }
}
```

Visibility and `#[non_exhaustive]` can affect whether the compiler can tell that all values of the type are structurally equal.

```rust
mod stuff {
    #[derive(PartialEq, Eq)]
    pub(super) struct PrivateZst(());

    pub(super) static PRIVATE_ZST: PrivateZst = PrivateZst(());
}

fn foo(scrutinee: stuff::PrivateZst) {
    match scrutinee {
        stuff::PRIVATE_ZST => println!("secrets abound"),
        // `stuff::PrivateZst` has a field that's not visible in this scope,
        // so we can't tell that all values are equivalent.
        // The wildcard branch is required.
        _ => println!("incorrect password"),
    }
}
```

```rust
// crate `stuff`
#[derive(PartialEq, Eq)]
#[non_exhaustive]
pub struct PrivateZst();

pub static PRIVATE_ZST: PrivateZst = PrivateZst();

// main crate
extern crate stuff;

fn foo(scrutinee: stuff::PrivateZst) {
    match scrutinee {
        stuff::PRIVATE_ZST => println!("secrets abound"),
        // `stuff::PrivateZst` is marked `#[non_exhaustive]`
        // and comes from an external crate,
        // so we can't tell that all values are equivalent.
        // The wildcard branch is required.
        _ => println!("incorrect password"),
    }
}
```

Static patterns can be nested in other patterns:

```rust
static ONE: i32 = 1;

fn foo(scrutinee: i32) {
    match scrutinee {
        ONE | 2 => println!("a"),
        _ => (),
    }

    match (scrutinee, scrutinee) {
        (ONE, ONE) =>  println!("a"),
        _ => (),
    }
}
```

The examples above all use `match`, but statics would be allowed in all other language constructs that use patterns, including `let`, `if let`, and function parameters. However, as statics cannot be used in const contexts, static patterns are be unavailable there as well.

# Drawbacks

[drawbacks]: #drawbacks

This change slightly weakens the rule that patterns can only rely on compile-time information. In addition, static patterns may have slightly worse performance than the equivalent constant patterns.

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

The proposed rules around reachability and exhaustiveness checking are designed to ensure that changing the value of a static, or changing from a static defined in Rust to a trusted extern static, is never a breaking change. The special dispensations for types with a single value could be considered unnecessary, as matching on such a type is a pointless operation. However, the rules are not difficult to implement (I managed to do it myself, despite near-zero experience contributing to the compiler), and are arguably the most correct and least surprising semantics.

Allowing unsafe-to-access statics in patterns (`static mut`s, untrusted `extern` statics, `#[thread_local]` statics) is another possibility. However, I believe this option to be unwise:

- Rust generally has not allowed unsafe operations (like union field accesses) in pattern matches
- It's not clear where the `unsafe` keyword would go (within the pattern? around the whole `match` or `let`? what about patterns in function parameters?)
- it requires Rust to commit to and document, and users to understand, when exactly it is allowed to dereference the static when performing a pattern match

As for not making this change at all, I believe this would be a loss for the language as it would lock out the use-cases described above. This is a very simple feature, it doesn't conflict with any other potential extensions, the behavior and syntax fit well with the rest of the language, and it is immediately understandable to anyone who is already familiar with matching on `const`s.

# Prior art

[prior-art]: #prior-art

As far as I am aware, no other language has an analogous feature. C's `switch` statement does not allow referring to C `const`s.

# Unresolved questions

[unresolved-questions]: #unresolved-questions

 - The motivation for this RFC assumes that [trusted external statics](https://github.com/rust-lang/lang-team/issues/149) will eventually be implemented and stabilized.
 - Should statics be accepted in range patterns (`LOW_STATIC..=HIGH_STATIC`)? One wrinkle is that the compiler currently checks at compile time that ranges are non-empty, but the values of statics aren't known at compile time. Such patterns could be either always accepted, accepted only when known to be non-empty (because the lower or upper bound is set to the minimum or maximum value of the type, respectively), or always rejected.

# Future possibilities

[future-possibilities]: #future-possibilities

None; this is a very simple and self-contained feature. I've argued against some possible extensions in the [rationale and alternatives](#rationale-and-alternatives) section. Future changes to the structural equality rules might affect this feature, but that is anther discussion and out of scope for this RFC.
