- Start Date: 2014-07-23
- RFC PR #: 
- Rust Issue #: 

# Summary

A chainable variant of `match` only for `Option`:

    with caps in re1.captures(name){
        ...     // case 1
    }else with caps in re2.captures(name){
        ...     // case 2
    }else{
        ...     // default
    }

# Motivation

Without this, the above pattern can be written two ways:

    // Stacked matches
    match re1.captures(name){
        Some(caps) => {
            ... // case 1
        },
        None => {
            match re2.captures(name){
                Some(caps) => {
                    ... // case 2
                },
                None => {
                    ... // default
                }
            }
        }
    }

This has the same effect, while being longer and requiring lots of indentation
(like `if ... else if` without chaining).

    // Unstacked matches
    match re1.captures(name){
        Some(caps) => {
            ... // case 1
            return;     // or continue or goto
        },
        None => {}
    }
    
    match re2.captures(name){
        Some(caps) => {
            ... // case 2
            return;     // or continue or goto
        },
        None => {}
    }
    
    ... // default

This version is longer, and not always applicable (unless you can really use
goto).

Another use case would be to only do some action on an option with a value,
though this is covered by .map().

# Detailed design

The above is pretty self-explanatory. Note that the pattern could be
implemented with macros, but the syntax would be aweful.

With the above syntax, `with` must be added as a keyword.

Vaguely similar features:

1.  [type narrowing/flow-sensitive typing][1]
2.  [for/else or for/default][2]

[1]: http://ceylon-lang.org/documentation/current/introduction/#typesafe_null_and_flow_sensitive_typing
[2]: https://docs.python.org/3/reference/compound_stmts.html#the-for-statement

# Drawbacks

Language complexity is increased to cover a small use case.

# Alternatives

1.  #160 if let
    
        if let Some(caps) = re1.captures(name)){
            ...     // case 1
        }else if let Some(caps) = re2.captures(name){
            ...     // case 2
        }else{
            ...     // default
        }
2.  For/default + option iter:
    
        for caps in re1.captures(name){
            ...     // case 1
            break;
        }default for caps in re2.captures(name){
            ...     // case 2
            break;
        }default{
            ...     // default
        }
3.  Using one of the more verbose syntaxes above

# Unresolved questions

New syntax can always be "bike-shedded".

A new or existing alternative may be a better compromise between making Rust
succinct and simple.
