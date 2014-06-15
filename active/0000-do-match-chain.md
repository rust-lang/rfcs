- Start Date: 2014-06-15
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Reintroduce the `do` keyword as sugar for nested match statements.

# Motivation

Flatten code that suffers from 'match pyramids' without losing the expressiveness of match statements.

# Detailed design

A macro implementation can be found here: https://gist.github.com/bvssvni/9674632

The chain macro is syntactically similar to a 'match' statement.
Each arm pattern matches against the result of the previous arm.
If any pattern match fails, the expression after the 'else' keyword is executed.
In the case the 'else' part is omitted, all arms must return same type (usually Result).

Syntax:

```Rust
do <start> {
    <pattern0> => <step1>,
    <pattern1> => <step2>,
    <pattern2> => <step3>,
    ... => <result>
} else <err>;
```

Example:

```Rust
// Creates a list of numbers from 1 to n where n is the first line in 'n.txt'.
let numbers: Vec<uint> = do File::open(&Path::new("n.txt")) {
        Ok(file) => BufferedReader::new(file).lines()
            .map(|line| line.unwrap()).next(),
        Some(first_line) => from_str::<uint>(first_line.trim()),
        Some(n) => range(0, n).map(|x| x + 1).collect()
    } else Vec::new();
```

This is equivalent to:

```Rust
// Creates a list of numbers from 1 to n where n is the first line in 'n.txt'.
let numbers: Vec<uint> = match File::open(&Path::new("n.txt")) {
        Ok(file) => match BufferedReader::new(file)
            .lines().map(|line| line.unwrap()).next() {
            Some(first_line) => match from_str::<uint>(first_line.trim()) {
                Some(n) => range(0, n).map(|x| x + 1).collect(),
                _ => Vec::new()
            },
            _ => Vec::new()
        },
        _ => Vec::new()
    };
```

The scope of a variable expands to the following arms:

```Rust
// Assigns 1 + 2 + 3 to 'res'.
let res = do Ok(1) {
        Ok(x) => Ok(2),
        Ok(y) => Ok(3),
        Ok(z) => x + y + z
    } else 0;
```

When the 'else' part is omitted, all arms need to return same type.

```Rust
// Assigns Err('z') to 'res'.
let res = do Ok(1) {
        Ok(x) => Ok(2),
        Ok(y) => Err('z'),
        Ok(z) => Ok(x + y + z)
    };
```

This is equivalent to:

```Rust
// Assigns Err('z') to 'res'.
let res = match Ok(1) {
        Ok(x) => match Ok(2) {
            Ok(y) => match Err('z') {
                Ok(z) => Ok(x+y+z)
                x => x
            },
            x => x
        },
        x => x
    };
```

### Omitting block for default enum variant

In case Rust gets a way to annotate default enum variants,
the block can be omitted from the statement.

Syntax:
```Rust
do <expr> else <err>;
```

the 'try!' macro can then be replaced by the following sugar to give more descriptive errors:

```Rust
use std::io::{File, IoResult};

fn file_product(p: &Path) -> IoResult<u32> {
    let mut f = do File::open(p) else return Err("Could not open file");
    let x1 = do f.read_le_u32() else return Err("Could not read 'x1'");
    let x2 = do f.read_le_u32() else return Err("Could not read 'x2'");

    Ok(x1 * x2)
}

match file_product(&Path::new("numbers.bin")) {
    Ok(x) => println!("{}", x),
    Err(e) => println!("{}", e)
}
```

# Drawbacks

The `do` keyword might be useful for other syntax.

# Alternatives

Can be added behind a feature flag `#![feature(do_chain)]`.

Add it as a macro or syntax extension to the standard library.

Syntax:

```Rust
chain!(do <start> {
    <pattern0> => <step1>,
    <pattern1> => <step2>,
    <pattern2> => <step3>,
    ... => <result>
} else <err>);
```

# Unresolved questions

None so far.
