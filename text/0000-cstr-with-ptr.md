- Feature Name: `cstr_with_ptr`
- Start Date: 2016-06-06
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Deprecate `CStr::as_ptr`, which returns a raw pointer and thereby invites unsound code, in favor of
a new method, `CStr::with_ptr`, which passes the pointer to a user-provided closure.

# Motivation
[motivation]: #motivation

`CString` wraps a C string and provides easy, safe access to the raw pointer to the string, for
FFI interop. The safety is correct, as there's nothing unsound about having a raw pointer around
until you dereference it. However, raw pointers are not tracked by the borrow checker, and once you
have one it's as easy in Rust as it is in C to keep it around too long (namely, after the backing
`CString` is dropped) and have a dangling pointer, which leads to unsound code.

There are [hundreds of projects](https://users.rust-lang.org/t/you-should-stop-telling-people-that-safe-rust-is-always-safe/6094/7)
including a call like `CString::new(...).unwrap().as_ptr()`, which is evidence that this UB
mistake is widespread.

By changing the API from a method _returning_ a raw pointer to one that runs a user-provided
closure, we make it easier to write sound code than to write unsound code. Sure, you can still
have the pointer escape the closure (see [Drawbacks](#drawbacks)), but it's a clear choice. The
API change tips the scales towards soundness.

# Detailed design
[design]: #detailed-design

1. Deprecate `CStr::as_ptr`, with the following message: "too easy to misuse because it returns a
raw pointer; use `with_ptr` instead". The method is stable, so it will not be removed in any 1.x
release.

2. Add the following method to `CStr`:

    ```rust
        /// Calls the provided closure, passing the inner pointer to this C string.
        ///
        /// The pointer will be valid for as long as `self` is and points to a contiguous region of
        /// memory terminated with a 0 byte to represent the end of the string.
        #[allow(deprecated)] // for the as_ptr call
        fn with_ptr<F: FnOnce(*const c_char)>(&self, f: F) {
            f(self.as_ptr());
        }
    ```

    For example usage see [this playpen](https://play.rust-lang.org/?gist=b6b1495ebee03fea679e95acb6b51ed6).

3. Modify the `CStr` and `CString` examples that use `as_ptr` to use `with_ptr` instead.

    `CString::new` has the following example code:

    ```rust
    use std::ffi::CString;
    use std::os::raw::c_char;

    extern { fn puts(s: *const c_char); }

    fn main() {
        let to_print = CString::new("Hello!").unwrap();
        unsafe {
            puts(to_print.as_ptr());
        }
    }
    ```

    Under this proposal, it would change to:

    ```rust
    use std::ffi::CString;
    use std::os::raw::c_char;

    extern { fn puts(s: *const c_char); }

    fn main() {
        let to_print = CString::new("Hello!").unwrap();
        to_print.with_ptr(|p| unsafe {
            puts(p);
        });
    }
    ```

    There are nearly identical examples on `CString` and `CStr` themselves which would change in the
    same way.

# Drawbacks
[drawbacks]: #drawbacks

- It deprecates another stable method, which contributes to perception of API churn.
- It adds surface area to the API of `CStr`.
- It's still rather easy to circumvent the help and write unsound code by "leaking" the pointer out
of the closure:

    ```rust
    let mut ptr = ptr::null();
    {
        let s = CString::new("foo").unwrap();
        s.with_ptr(|p| { ptr = p; });
    }
    /* ptr is now dangling */
    ```

# Alternatives
[alternatives]: #alternatives

- Do nothing. It remains very easy to write unsound code using `CString` and `CStr::as_ptr`.
- Just make the warnings louder and the font larger in the docs for `CStr::as_ptr`. Continue to assume that people read this documentation despite the fact that the documentation examples show correct usage, but incorrect usage abounds in the ecosystem.
- Move the `temporary_cstring_as_ptr` lint, which warns on the most common way to write said unsound
code (calling `as_ptr` on a temporary `CString`) from Clippy to rustc.
- Deprecate `as_ptr` and introduce a new function that does the same thing (we would have to bikeshed
the name), but mark it `unsafe` due to the potential for unsoundness in combination with unsafe code.

# Unresolved questions
[unresolved]: #unresolved-questions

- Should `with_ptr` allow the closure to return a value? It could be

    ```rust
        fn with_ptr<F: FnOnce(*const c_char) -> O, O>(&self, f: F) -> O {
            f(self.as_ptr())
        }
    ```

    which might be convenient but would make it easier to "leak" the pointer (as easy as
    `let ptr = s.with_ptr(|p| p);`).

- Does `f(CString::new(...).unwrap().as_ptr())` actually invoke undefined behavior, if `f` doesn't store the pointer? The author's reading of the Rust reference implies that the `CString` temporary is kept alive for the entire expression, so it's fine. However, some commenters in the RFC thread have opined that the behavior of this code is unspecified at best.

