- Feature Name: ptr-to-int-truncate-warning
- Start Date: 2016-11-1
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Warn by default when casting a pointer to an integer smaller than usize.

# Motivation
[motivation]: #motivation

Recently someone [posted in rust-users](https://users.rust-lang.org/t/can-mprotect-succeed-in-rust/7704) asking for help with mprotect.  It turned out that they were trying to align a pointer by casting it to an integer, doing arithmetic on it, and casting back to a pointer, which is fine, but they were inadvertently using `i32` on a 64-bit platform, causing the pointer value to be truncated.

I was surprised that rustc didn't emit a warning for this, because casting a pointer directly to a smaller integer is almost never desirable.  It seems someone already filed [an issue report](https://github.com/rust-lang/rust/issues/31012) on the rustc repo asking for this, but it was closed with the justification that adding lints requires a RFC.  So here's an RFC.

There are a few cases I can think of where such casts could be present in correct code, but they're all niche:

- Most likely: the "pointer" did not really point to anything, but was just an integer stashed in a pointer field for some reason.  Semi-common in C with `void *` arguments; should be uncommon in Rust.

- The pointer is guaranteed to be at the beginning of the address space (e.g. the first 4GB on a 64-bit system, when casting to a 32-bit integer).  There are [some](http://timetobleed.com/digging-out-the-craziest-bug-you-never-heard-about-from-2008-a-linux-threading-regression/) legacy [environments](https://blogs.msdn.microsoft.com/oldnewthing/20150709-00/?p=45181) where this guarantee holds, but new code is unlikely to care about this.

- The pointer is being split into multiple parts, e.g.

    ```rust
    let a = ptr as u32;
    let b = ((ptr as usize) >> 32) as u32;
    ```

    This is very rare, and in this case it would look better anyway to cast to usize first:

    ```rust
    let ptr_i = ptr as usize;
    let a = ptr_i as u32;
    let b = (ptr_i >> 32) as u32;
    ```

Of course, in any case where truncation is desired, you can always avoid the warning using `foo as usize as smaller_type`.

Comparison to other languages:

- In C++ such casts are a hard error (not just as implicit conversions but even when explicitly specified as casts).

- In C they are not; however, GCC emits a warning by default, though Clang does not.  (I haven't tested any other compilers.)

# Detailed design
[design]: #detailed-design

Add a new warn-by-default lint (I don't think it fits into any existing ones) which flags `as` casts from pointer types to integer types whose size on the current target is smaller than that of usize.

The lint would not apply to casts to types that are appropriately sized on the current target but would be too small if the project were compiled for some other target, e.g. casting a pointer to `u32` on a 32-bit platform.  In, say, low-level OS code that's only ever intended to run on 32-bit platforms, such casts are reasonably frequently justified, and requiring an unnecessary cast to `usize` is at best useless, at worst misleading or harmful if the code is ever ported.  In the future, once a design for "scenarios" (as discussed elsewhere) is fleshed out, there will likely be some mechanism for the user to indicate that their code ought to be portable to platforms with different pointer sizes, in which case this could be revisited.

# Drawbacks
[drawbacks]: #drawbacks

- Like any new or modified lint, this could break code that contains `#![deny(warnings)]`.

- In cases where truncation is desired, `ptr as usize as u32` is fairly ugly.  On the other hand, it makes it more clear that truncation is occurring.

- Users developing cross-platform code that casts `usize` to `u32`, intending truncation to occur on 64-bit platforms, won't see the warning if they happen to be developing on a 32-bit platform; anyone compiling it for a 64-bit platform will then get a spurious warning.  This isn't the end of the world, and should be addressed eventually by scenarios.

- Each new lint adds to compilation time.

# Alternatives
[alternatives]: #alternatives

- Put this in [clippy](https://github.com/Manishearth/rust-clippy).  As things stand, I can't discern much of a clear dividing line betweeen clippy and rustc in terms of degree of pedantry or prescriptiveness or whatever.  Many of the warn-by-default lints in clippy are more prescriptive than I'm used to, like (just going down the list) `block_in_if_condition_stmt`, `box_vec`, `cyclomatic_complexity`, `empty_loop`, and `inline_always`; but others are almost guaranteed to represent a mistake, like `absurd_extreme_comparisons`, `builtin_type_shadow`, `double_neg`, and `eq_op`.  I'd claim that my proposed lint is closer to the latter category.

- Always warn when casting directly from pointers to fixed-size integers, regardless of the current target.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
