- Feature Name: Else Match
- Start Date: 2016-07-26
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Extend the `if` expression to accept an `else match` block.

# Motivation
[motivation]: #motivation

This proposal is meant to reduce the verbosity of writing `if ... else { match ... } ` expressions.

```rust
if foo() {
    do_this();
} else {
    match bar() {
        baz() => do_that(),
        _ => flag = true
    }
}
```

simpler and more concise:

```rust
if foo() {
    do_this();
} else match bar() {
    baz() => do_that()
}
```

## Use-cases

Though rare, this pattern does exist in several Rust projects.

### [xsv](https://github.com/BurntSushi/xsv)

<https://github.com/BurntSushi/xsv/blob/master/src/cmd/stats.rs#L311>:

Before:

```rust
if !self.typ.is_number() {
    pieces.push(empty()); pieces.push(empty());
} else {
    match self.online {
        Some(ref v) => {
            pieces.push(v.mean().to_string());
            pieces.push(v.stddev().to_string());
        }
        None => { pieces.push(empty()); pieces.push(empty()); }
    }
}
```

After:

```rust
if !self.typ.is_number() {
    pieces.push(empty()); pieces.push(empty());
} else match self.online {
    Some(ref v) => {
        pieces.push(v.mean().to_string());
        pieces.push(v.stddev().to_string());
    }
    None => { pieces.push(empty()); pieces.push(empty()); }
}
```

### [trust-dns](https://github.com/bluejekyll/trust-dns)

<https://github.com/bluejekyll/trust-dns/blob/master/src/authority/authority.rs#L558>:

Before:

```rust
if class == self.class {
  match rr.get_rr_type() {
    RecordType::ANY | RecordType::AXFR | RecordType::IXFR => return Err(ResponseCode::FormErr),
    _ => (),
  }
} else {
  match class {
    DNSClass::ANY => {
      if rr.get_ttl() != 0 { return Err(ResponseCode::FormErr) }
      if let &RData::NULL(..) = rr.get_rdata() { () }
      else { return Err(ResponseCode::FormErr) }
      match rr.get_rr_type() {
        RecordType::AXFR | RecordType::IXFR => return Err(ResponseCode::FormErr),
        _ => (),
      }
    },
    DNSClass::NONE => {
      if rr.get_ttl() != 0 { return Err(ResponseCode::FormErr) }
      match rr.get_rr_type() {
        RecordType::ANY | RecordType::AXFR | RecordType::IXFR => return Err(ResponseCode::FormErr),
        _ => (),
      }
    },
    _ => return Err(ResponseCode::FormErr),
  }
}
```

After:

```rust
if class == self.class {
  match rr.get_rr_type() {
    RecordType::ANY | RecordType::AXFR | RecordType::IXFR => return Err(ResponseCode::FormErr),
    _ => (),
  }
} else match class {
  DNSClass::ANY => {
    if rr.get_ttl() != 0 { return Err(ResponseCode::FormErr) }
    if let &RData::NULL(..) = rr.get_rdata() { () }
    else { return Err(ResponseCode::FormErr) }
    match rr.get_rr_type() {
      RecordType::AXFR | RecordType::IXFR => return Err(ResponseCode::FormErr),
      _ => (),
    }
  },
  DNSClass::NONE => {
    if rr.get_ttl() != 0 { return Err(ResponseCode::FormErr) }
    match rr.get_rr_type() {
      RecordType::ANY | RecordType::AXFR | RecordType::IXFR => return Err(ResponseCode::FormErr),
      _ => (),
    }
  },
  _ => return Err(ResponseCode::FormErr),
}
```

# Detailed design
[design]: #detailed-design

## Grammar

See the following document for an (incomplete) guide to the grammar used in Rust:
[Rust Documentation → Grammar](https://doc.rust-lang.org/grammar.html).

This proposal modifies the
[if expression grammar](https://doc.rust-lang.org/grammar.html#if-expressions).

```
 else_tail : "else" [ if_expr | if_let_expr
+                   | match_expr
                    | '{' block '}' ] ;
```

## Execution

An `else match` block should be treated exactly the same as an `else` block with a single `match`
expression.

### Dead code

The next `else` block after an `else match` is never run. This is because `match` itself does not
take on an `else` block. Whether `match` should allow an `else` or not should be addressed in a
separate proposal.

# Drawbacks
[drawbacks]: #drawbacks

- Slight maintainability problems whenever you need to add additional logic to an `else` block.
- Can be more of a stylistic issue than an ergonomics issue.

# Alternatives
[alternatives]: #alternatives

Not an alternative but an addition to the proposal: `if match` expressions. This would add an
additional grammar rule and modify an existing one:

```
+ if_match_expr : "if" match_expr else_tail ? ;
```

```
 expr : literal | path | tuple_expr | unit_expr | struct_expr
      | block_expr | method_call_expr | field_expr | array_expr
      | idx_expr | range_expr | unop_expr | binop_expr
      | paren_expr | call_expr | lambda_expr | while_expr
      | loop_expr | break_expr | continue_expr | for_expr
      | if_expr | match_expr | if_let_expr | while_let_expr
-     | return_expr ;
+     | if_match_expr | return_expr ;
```

Should work nearly the same as `else match`.

# Unresolved questions
[unresolved]: #unresolved-questions

Whether to allow `match` to take on an `else` block. This should be addressed
in a separate proposal.
