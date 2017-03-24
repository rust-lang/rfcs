- Start Date: 2014-07-16
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)
- Author: Liigo Zhuang

# Summary

Allow multiple crates in one `extern crate` declaration.

# Motivation

To make the source code more compact, and be consistent with `use` declaration, which allow use multiple types/functions in one line (one declaration).

# Detailed design

Instead of having to write:

```
extern crate collections;
extern crate flate;
extern crate libc;
extern crate log;
extern crate num;
extern crate regex;
extern crate serialize;
extern crate test;
```

... the programmers could be allowed to write these in one line:

```
extern crate collections, flate, libc, log, num, regex, serialize, test;
```

The new syntax EBNF would be:

```
extern_crate_decl : "extern" "crate" crate_list ;
crate_list : ident [ '(' link_attrs ')' ] ? [ '=' string_lit ] ? [ ',' crate_list ] + ;
link_attrs : link_attr [ ',' link_attrs ] + ;
link_attr  : ident '=' literal ;
```

After this change, the source code is more compact, but still keep clean, concise and readable.

# Drawbacks

Lexer syntax will be a little more complex.

# Alternatives

Use the `{ }` syntax of `use` declaration:

```
extern crate {collections, flate, libc, log, num, regex, serialize, test};
```

which is a little fussy.

# Unresolved questions

None.
