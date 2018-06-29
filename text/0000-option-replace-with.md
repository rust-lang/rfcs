- Feature Name: `option-replace-with`
- Start Date: 2018-06-28
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC proposes the addition of `Option::replace_with` to compliment `Option::replace` ([RFC #2296](https://github.com/rust-lang/rfcs/pull/2296)) and `Option::take` methods. It replaces the actual value in the option with the value returned from a closure given as a parameter, while the old value is passed into the closure.

# Motivation
[motivation]: #motivation

`Option::replace_with` helps to clarify the intent and also [improves the performance of a naive `Option::take` + assignment implementation](https://barrielle.cedeela.fr/research_page/dropping-drops.html).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`Option::replace_with` is a replacement for the following code:

```rust
let mut some_option: Option<i32> = Some(123);

some_option = consume_option_i32_and_produce_option_i32(some_option.take());
```

With `Option::replace_with` you will write:

```rust
let mut some_option: Option<i32> = Some(123);

some_option.replace_with(|old_value| consume_option_i32_and_produce_option_i32(old_value));

// OR

some_option.replace_with(consume_option_i32_and_produce_option_i32);
```

While the first implementation works fine, it generates suboptimal code due to unnecessary "allocation" and "deallocation" of `None` value. The naive implementation is about 10% slower than the optimal solution:

```rust
let mut some_option: Option<i32> = Some(123);

let old_value = unsafe { mem::uninitialized() };
mem::swap(&mut some_option, old_value);
let mut new_value = consume_option_i32_and_produce_option_i32(old_value);
mem::swap(&mut some_option, &mut new_value);
mem::forget(new_value);
```

`Option::replace_with` can implement the trick and reach the maximum performance.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This method will be added to the `core::option::Option` type implementation:

```rust
use core::mem;

impl<T> Option<T> {
    // ...

    #[inline]
    fn replace_with<F>(&mut self, f: F)
    where
        F: FnOnce(Option<T>) -> Option<T>,
    {
        let mut old_value = unsafe { mem::uninitialized() };
        mem::swap(self, &mut old_value);
        let mut new_value = f(old_value);
        mem::swap(self, &mut new_value);
        // After two swaps (`old_value` -> `self` -> `new_value`), `new_value`
        // holds an `uninitialized` value, so we just forget about it.
        mem::forget(new_value);
    }
}
```

Here is a benchmark: [link](https://github.com/frol/rust-benchmark-option-replace_with-rfc).

Here is a generated assembly code comparison in Compiler Explorer: [link](https://godbolt.org/g/6Cukig) (naive implementation is on the left, and optimized implementation is on the right).

# Drawbacks
[drawbacks]: #drawbacks

There will be no need in this method if the compiler can optimize the cases when it is clear that the variable holds `None`, i.e. `Option::take` and simple assignment would not produce unnecessary `moveq 0` and `drop_in_place` call.

This `Option::replace_with` solves only a single case and even than it has limits, e.g. if the function you call inside the closure needs to produce some other value in addition to the value that is going to be a new replacement, [that value cannot "leak" the closure efficiently in safe Rust](https://stackoverflow.com/questions/50985651/how-to-hint-that-a-fnonce-closure-will-be-executed-exactly-once-to-avoid-a-capt).

# Rationale and alternatives
[alternatives]: #alternatives

The rationale for proposing `Option::replace_with` is that it is the simplest way to boost the performance for the use-case.

The alternative is to teach Rust compiler or LLVM to optimize the use-case expressed with a simple assignment.

# Prior art
[prior-art]: #prior-art

[The performance issue and the workaround were initially discovered](https://barrielle.cedeela.fr/research_page/dropping-drops.html) during the digging into [Completely Unscientific Benchmark](https://www.reddit.com/r/rust/comments/8jbjku/naive_benchmark_treap_implementation_of_c_rust/).

Naive searching through Rust codebase revealed only a single case where currently a simple assignment is used: [`src/librustdoc/passes/collapse_docs.rs`](https://github.com/rust-lang/rust/blob/e3bf634e060bc2f8665878288bcea02008ca346e/src/librustdoc/passes/collapse_docs.rs#L52-L81).

# Unresolved questions
[unresolved]: #unresolved-questions

- Should `Option::replace_with` be introduced or LLVM/Rustc should implement a general optimization which will cover this use-case as well as many others?
