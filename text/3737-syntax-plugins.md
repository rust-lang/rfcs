- Feature Name: Syntax_Plugins
- Start Date: 2015-03-23 13:27:58
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Q: One para explanation of the feature.

A: if that supported, so I could implement like that:
   "实现 { ... }" could be equal to "impl { ... }".

# Motivation

Q: Why are we doing this?

A: I don't know. but if you don't do that, I will try to do.

Q: What use cases does it support?

A: The syntax plugins could implement the plugin keywords as what rust language did.

Q: What is the expected outcome?

A: Syntax plugins can't overload/replace/use rust language keywords.
   The Syntax plugin's keywords could not conflict another syntax plugin's keywords.

# Detailed design

This is the bulk of the RFC. Explain the design in enough detail for somebody familiar
with the language to understand, and for somebody familiar with the compiler to implement.
This should get into specifics and corner-cases, and include examples of how the feature is used.


```rust

impl foo {
	fn bar() {
		let a = ();
	}
}

```

```
实现 甲 {
	函数 丙 {
		变量 乙 = ();
	}
}
```

I have a file, content: `"Hello {}": "你好 {}"`

If it implemented, so I can do that:

`println!(trans "Hello {}", "World!");` which outputs: `你好 World!`

# Drawbacks

Q: Why should we *not* do this?

A: If you don't do that, I'm afraid I have to fork rustc to do my own version.
   Let's see does I have that capability. :S

# Alternatives

Q: What other designs have been considered? What is the impact of not doing this?

A: Macro plugins. But I think that will reinvert the wheel for any exists keywords/macros/attrs.

# Unresolved questions

What parts of the design are still TBD?
