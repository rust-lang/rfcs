- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

The plan is to refine the syntax of the `asm!` macro to make it look idiomatic
and easier to use. Currently, the inline assembly syntax is based on that
of clang.

# Motivation

The inline assembly extension has several slight deficiencies in its current
revision. To begin with, operands are always grouped by type (output/input) in
the declaration. Moreover, operands are referenced in the template by their
index. This interface seems inferior to our `format!` macro.

Fortunately, this extension is feature gated, with potential for modification.

# Detailed design

The `asm!` extension is substantially changed.

Original `asm!` usage as implemented by Luqman, for reference:

```rust
asm!(" //assembly template "
      : output_operands   // format: "constraint1"(expr1), "constraint2"(expr2), etc
      : input_operands    // format: "constraint1"(expr1), "constraint2"(expr2), etc
      : clobbers          // format: "eax", "ebx", "memory", etc
      : options           // comma separated string literals
);
```

The revised extension is used as follows:

```rust
asm!(" //assembly template ",
      positional parameters, // format: expr1, expr2_in -> expr2_out, "{eax}" = expr3_in -> expr3_out, etc
      named parameters,      // format: name1 = expr_in_out, name2 = expr_in -> expr_out, etc
      clobbers and options   // format: "eax", "ebx", "memory", "volatile", "intel" etc
);
```

A parameter consists of an expression at the minimum. Its other properties
(type and constraint) can be dictated within the assembly string.

In contrast to the `format!` macro, an argument can be referred to using many
constraints.
Referring to a parameter with different constraints such as {:r} and {:m}
in the template will generate many separate old-style operands. Additionally,
it's easier to see which one is allowed in an instruction.

An unused argument should cause an error, unless a constraint is explicitly
specified with a string literal.

## Positional parameters

```
[ string_lit '=' ] ? expr [ "->" expr ] ?
```

An optional string literal sets the constraint regardless of the template.

The expression can be both input and output expression. It depends on the
template. At least one operand `"{,=,+}constraint"(expr)` is generated.

An optional output expression follows. It makes the parameter read+write if
it isn't already. It basically generates an additional operand in the form of
`"0"(expr_out)`.

## Named parameters

```
[ ident '=' ] ? expr [ '->' expr ] ?
```

The only difference is that they can be only referenced by name.

## Examples

Consider this excerpt from Rust by Example:

```rust
asm!("add $2, $1; mov $1, $0" : "=r"(sum) : "r"(a), "r"(b));
```

This simple addition could use positional parameters:

```rust
asm!("add {:r}, {:r}", b, a -> sum);
```

It's also possible to set constraints for parameters that aren't referred to
within the assembly string:

```rust
asm!("syscall" : "{rax}" = n -> ret, {rdi}" = a1, "{rsi}" = a2, "rcx", "r11", "memory", "volatile");
```

This example uses multiple outputs:

```rust
fn addsub(a: int, b: int) -> (int, int) {
    let mut c = 0;
    let mut d = 0;
    unsafe {
        asm!("add {2:r}, {:=r}\n\t\
              sub {2:r}, {:=r}",
              a -> c, a -> d, b)
    }
    (c, d)
}

fn main() {
    io::println(fmt!("%?", addsub(5, 1)));
}
```

# Drawbacks

* The syntax is new and partly unfamiliar. The meaning of `->` placed in
between expressions is not immediately obvious.
* The `format` parser needs a slight modification or refactoring to allow
`{:=}` and perhaps `{:+}`.
* Some `asm!` code might already contain `{}`, so this change can't be entirely
painless and backwards compatible. ARM register lists are enclosed in braces:
`push {r11, lr}`. This instruction would have to look like `push {{r11, lr}}`.
* Automatic indexing won't work with comments containing braces such as `// {}`.

# Alternatives

* Implement this as `asm_format!` extension alongside `asm!`.
* Keep the current extension. `asm_format!` can be implemented and maintained
separately.

# Unresolved questions

* Is it sane to mix clobbers and options in the same place?
* How to set the type of a parameter? Is it possible to avoid writing
`var = var -> var` to set the type?
