- Feature Name: warning_on_tautology_else
- Start Date: 2017-07-27
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

When the compiler can statically determine if an if branch will always be `true` or `false` then a compiler warning should be outputted saying something like: `tautology detected else branch unreachable` or `contradiction detected if branch unreachable`.

# Motivation
[motivation]: #motivation

The motivation behind this so that the programmer can be told about items that may be logical mistakes.

# Detailed design
[design]: #detailed-design

When going through the branch detection if an expression within an if statement is true or if the expression is false within an if or while statement then the warning should be outputted.
Since this is a compiler warning the ability to ignore it should be also allowed so using a macro like `cfg!` does not throw this warning.
So either the compiler should look for the `#[allow]` statement before an if/while statement (which is currently not allowed) or head of where the value is defined.
The former seems like a more intuitive solution because it places the `#[allow]` in the context of where it will apply.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

This can be taught by adding to the books by adding examples like the following which will show the warnings in action.

```rust
if x > 5 && x < 5 {
    call_fn();
} else {
    call_other_fn();
}

------------------------------

warning: contradiction in if statement, associated block unreachable
1 | /     if x > 5 && x < 5 {
2 | |         call_fn();
3 | |     } else {
4 | |         call_other_fn();
5 | |     }
  | |_____^
  |
  = note: #[warn(tautology-contradiction)] on by default
```

# Drawbacks
[drawbacks]: #drawbacks

This would require allowing `#[allow]` to be placed before `if` and `while` statements

# Alternatives
[alternatives]: #alternatives

The impact of not doing this would be little since it is a warning addition which is currently not present.

# Unresolved questions
[unresolved]: #unresolved-questions
