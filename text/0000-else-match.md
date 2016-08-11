- Feature Name: Else Match
- Start Date: 2016-07-26
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Extend the `if` expression to accept an `else match` block.

# Motivation
[motivation]: #motivation

This proposal is meant to reduce the verbosity of writing `if ... else { match ... } ` style
statements.

It also makes code similar to:

```rs
let flag = false;

if foo() {
    do_this();
} else {
    match bar() {
        baz() => do_that(),
        _ => flag = true
    }
}

if flag {
    do_something();
}
```

simpler and more concise:

```rs
if foo() {
    do_this();
} else match bar() {
    baz() => do_that()
} else {
    do_something();
}
```

## Use-cases

Though rare, this pattern does exist in several Rust projects.

### [Servo](https://github.com/servo/servo)

<https://github.com/servo/servo/blob/master/components/layout/model.rs#L557>

Before:

```rs
/// Clamp the given size by the given `min` and `max` constraints.
pub fn clamp(&self, other: Au) -> Au {
    if other < self.min {
        self.min
    } else {
        match self.max {
            Some(max) if max < other => max,
            _ => other
        }
    }
}
```

After:

```rs
/// Clamp the given size by the given `min` and `max` constraints.
pub fn clamp(&self, other: Au) -> Au {
    if other < self.min {
        self.min
    } else match self.max {
        Some(max) if max < other => max,
        _ => other
    }
}
```

### [xsv](https://github.com/BurntSushi/xsv)

<https://github.com/BurntSushi/xsv/blob/master/src/cmd/stats.rs#L311>:

Before:

```rs
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

```rs
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

```rs
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

```rs
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
} else {
  return Err(ResponseCode::FormErr)
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
+                  | match_expr
                   | '{' block '}' ] ;
```

## Execution

An `else match` block should be treated similar to an `else` block with a single `match`
expression, with one key addition:

When an arbitrary expression `exp` in `else match <exp>` fails to match any of the cases in the
`else match` block, the next block (if it exists) is run.

### Dead code

An `else match` block with a `_ => ...` match means the next clause (if exists) will never run.
There might be more complicated cases to optimize for, and is outside the scope of this document.

# Drawbacks
[drawbacks]: #drawbacks

- Slight maintainability problems whenever you need to add additional logic to an `else` block.

# Alternatives
[alternatives]: #alternatives

Don't do this.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
