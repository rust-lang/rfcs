- Feature Name: safe_blocks
- Start Date: 2025-01-31
- RFC PR: [rust-lang/rfcs#3768](https://github.com/rust-lang/rfcs/pull/3768)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This is a RFC to add safe blocks to the language. Safe blocks would have the opposite effect of unsafe blocks and only be valid within unsafe blocks.

This RFC also proposes adding a lint when you do something like `/* safe context */ unsafe { safe { /* code */ } }`, as this would be an example of an anti-pattern.

# Motivation
[motivation]: #motivation

Adding safe blocks can be useful in many contexts, an example of which was shared by programmerjake on the Rust Internals forum:
> That said, I think having some form of safe {} blocks is a great idea that I've been wanting for some time, mostly because it'll be useful in macros, e.g.:
>
> ```rust
> macro_rules! inc {
>    ($a:expr) => {
>        unsafe {
>            // not the best example...other macros much more
>            // naturally have inputs in the middle of an `unsafe` block
>            let mut a: u64 = safe { $a };
>            asm!("inc {}", inlateout(reg) a);
>            a
>        }
>    };
> }
> ```

Another example, shown by me (AverseABFun) on the Rust Internals forum:

> [snip] for example, I was writing a function like this(ignore the contents, it's not super important):
>```rust
>/// Parses a PC Screen Font into a [PCScreenFont].
>pub fn parse_pc_screen_font(data: RawPCScreenFont) -> Result<PCScreenFont, crate::Error<'static>> {
>    unsafe {
>        let unitable: &[&str] = &[];
>        let unistr = data.glyphs.byte_add(data.bytes_per_glyph as usize*data.num_glyphs as usize);
>        /* Safe block start */
>        for i in 0..(data.num_glyphs as usize) {
>            let char = (*unistr)[i];
>            /* Didn't finish writing code here lol */
>        }
>        /* Safe block end */
>        
>        /* Snip */
>    }
>}
>```
>
>And I'd like to have the for loop be safe [snip], however having something like:
>
>```rust
>/// Parses a PC Screen Font into a [PCScreenFont].
>pub fn parse_pc_screen_font(data: RawPCScreenFont) -> Result<PCScreenFont, crate::Error<'static>> {
>    unsafe {
>        let unitable: &[&str] = &[];
>        let unistr = data.glyphs.byte_add(data.bytes_per_glyph as usize*data.num_glyphs as usize);
>    }
>    for i in 0..(data.num_glyphs as usize) {
>        let char = (*unistr)[i];
>         /* Didn't finish writing code here lol */
>    }
>    unsafe {
>        /* Snip */
>    }
>}
>```
>
>seems too obtrusive. Please let me know if I'm missing something, but here's an example code sample with my same example:
>
>```rust
>/// Parses a PC Screen Font into a [PCScreenFont].
>pub fn parse_pc_screen_font(data: RawPCScreenFont) -> Result<PCScreenFont, crate::Error<'static>> {
>    unsafe {
>        let unitable: &[&str] = &[];
>        let unistr = data.glyphs.byte_add(data.bytes_per_glyph as usize*data.num_glyphs as usize);
>        safe {
>            for i in 0..(data.num_glyphs as usize) {
>                let char = (*unistr)[i];
>                /* Didn't finish writing code here lol */
>            }
>        }
>        
>        /* Snip */
>    }
>}
>```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

So they're called safe blocks, and are only allowed within unsafe blocks. The idea is that you can mark parts of code as being not allowed to have unsafe code. An example might be if you were writing a macro where it had to be unsafe, however you wanted the inputs to be safe, then you could wrap the input in `safe { }` so that it would be interpreted as safe code. This generally makes it more clear that code is safe than if you ended an unsafe block before the code and started it after, however the same thing applies the other way where it's less clear that code is unsafe again after the safe block. The compiler also gives a lint if you do something like `unsafe { safe { /* code */ } }`, as there's not supposed to be any logical reason to do this since the inside of safe blocks and outside of unsafe blocks are supposed to be identical. This can of course be disabled, though.

An example might be:
```rust
macro_rules! inc {
    ($a:expr) => {
        unsafe {
            // not the best example...other macros much more
            // naturally have inputs in the middle of an `unsafe` block
            let mut a: u64 = safe { $a };
            asm!("inc {}", inlateout(reg) a);
            a
        }
    };
}
```
As you can see, the code needs to be unsafe and putting the safe code out of the unsafe block decreases readability, and so safe blocks would likely be the best choice in this scenario.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A block of code can be prefixed with the `safe` keyword to disable [unsafe operations](https://doc.rust-lang.org/reference/unsafety.html) within an unsafe block. Example:
```rust
unsafe {
    let b = [13u8, 17u8];
    let a = &b[0] as *const u8;
    assert_eq!(*a, 13);
    assert_eq!(*a.offset(1), 17);
    safe {
        b[0] = 1;
        b[1] = b.len();
    }
}

unsafe {
    let a = safe { a_safe_fn() };
}

unsafe fn an_unsafe_fn() -> [u8; 2] {
    let b = [13u8, 17u8];
    let a = &b[0] as *const u8;
    safe {
        b[0] = 1;
        b[1] = b.len();
    }
    b
}
```

# Drawbacks
[drawbacks]: #drawbacks

For one thing, there are alternatives as mentioned below. Outside of that, I can't think of any particular drawbacks.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This design fits in nicely with unsafe blocks and how they look/are implemented. This is a matter of preference, however and there are alternatives. Using the incrementing example, you could do this:
```rust
macro_rules! inc {
    ($a:expr) => {
        let mut a: u64 = $a;
        unsafe {
            asm!("inc {}", inlateout(reg) a);
            a
        }
    };
}
```
However, this decreases readability in my opinion, however this is a matter of opinion. This design also seems the most natural when you are learning rust as it is very similar to unsafe blocks. If this isn't done, there isn't much direct impact, however it does decrease readability in certain circumstances. As far as I know, this couldn't be implemented in a macro, but I could be wrong. As mentioned, this generally increases readability.

# Prior art
[prior-art]: #prior-art

There are some relevant internals forum threads: [thread 1](https://internals.rust-lang.org/t/ability-to-call-unsafe-functions-without-curly-brackets/19635) [thread from me about this](https://internals.rust-lang.org/t/idea-safe-blocks-for-inside-unsafe-blocks/22300)

Also, someone on the forum mentioned that asm_goto had to do a similar thing as the context that is called is safe. Also, programmerjake mentioned in the comments for the asm_goto tracking issue an idea for safe blocks: [comment](https://github.com/rust-lang/rust/issues/119364#issuecomment-2323435162)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Before this RFC gets merged, I expect to resolve how where this would be implemented in the compilation process so this can be effectively implemented. Before merging, I also expect to resolve information relevant to the proposed lint such as the name, detection, and if adding said lint is even a good idea.

Something considered out-of-scope would be single-operation unsafes. A brief explanation is it is an idea to add a way to mark a single operation as unsafe without making an unsafe block; however this is out-of-scope.

# Future possibilities
[future-possibilities]: #future-possibilities

At the moment, I cannot think of any future possiblities.
