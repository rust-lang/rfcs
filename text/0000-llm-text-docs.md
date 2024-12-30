# Feature Name: rustdoc_text_format

Start Date: 2024-12-30
RFC PR: (leave this empty)
Rust Issue: (leave this empty)

# Summary
Add a new rustdoc output format that generates a simplified, AI-friendly version of the crate's public API surface. This format excludes private items and function implementations while preserving documentation, type signatures, and the module structure.

# Motivation
As artificial intelligence becomes increasingly important in software development, there's a growing need for machine-readable documentation that can help AI systems quickly understand crate structure and capabilities. Current documentation formats are either:

1. Too verbose (full source code)
2. Too sparse (generated HTML docs)
3. Not machine-optimized (markdown/text documentation)

This proposal aims to create an intermediate format that maintains the essential structure and documentation while removing implementation details that aren't necessary for understanding the public API.

# Guide-level explanation
The new format can be generated using a new rustdoc flag:

```bash
cargo rustdoc --output-format=text
```

This will generate a .txt file containing the crate's public API surface, structured similarly to the source code but with the following modifications:

- All private items (functions, structs, fields, etc.) are excluded
- Function bodies are omitted
- Documentation comments are preserved
- Type signatures and trait bounds are preserved
- Module structure is maintained
- Macros are included with their documentation but not their implementation

Example output:

```rust
/// A collection type that stores elements in sorted order
pub struct BTreeMap<K, V> 
where 
    K: Ord
{
    /// The comparison function used to maintain ordering
    pub comparator: Option<Box<dyn Fn(&K, &K) -> Ordering>>,
}

impl<K: Ord, V> BTreeMap<K, V> {
    /// Creates an empty BTreeMap
    /// 
    /// # Examples
    /// ```
    /// use std::collections::BTreeMap;
    /// let map: BTreeMap<i32, &str> = BTreeMap::new();
    /// ```
    pub fn new() -> Self

    /// Returns a reference to the value corresponding to the key
    pub fn get(&self, key: &K) -> Option<&V>
}

pub mod operations {
    /// Merges two BTrees into a new tree
    pub fn merge<K: Ord, V>(left: &BTreeMap<K, V>, right: &BTreeMap<K, V>) -> BTreeMap<K, V>
}
```

# Reference-level explanation
The implementation will require:

1. Add `text` as a new value for the existing [`--output-format` flag](https://doc.rust-lang.org/nightly/cargo/commands/cargo-rustdoc.html#option-cargo-rustdoc---output-format)
2. New visitor pattern in rustdoc that:
   - Only traverses public items
   - Collects documentation strings
   - Records type signatures
   - Maintains module hierarchy
   - Skips function bodies
   - Preserves macro documentation

3. New text formatter that:
   - Maintains proper indentation
   - Uses consistent spacing
   - Preserves doc comments in their original format
   - Includes essential type bounds and where clauses
   - Formats signatures consistently

4. Integration with existing `--output-format` flag:
   ```bash
   cargo rustdoc --output-format=text
   ```

The `text` format will join the existing options:
* `html` (default): Emit the documentation in HTML format
* `json`: Emit the documentation in the experimental JSON format
* `text`: Emit the documentation in the new LLM-friendly text format

# Drawbacks
1. Additional maintenance burden for rustdoc

# Rationale and alternatives
## Why this design
1. Maintains familiar Rust syntax
2. Preserves essential information for understanding the API
3. Removes noise (private items and implementations)
4. Easy to generate and parse
5. Human-readable as a bonus

## Alternatives
1. Do nothing
   - Pro: No maintenance burden
   - Con: AI tools must parse full source or incomplete docs

# Prior art
1. TypeScript's .d.ts declaration files
3. Java's javadoc machine-readable output

# Unresolved questions
1. Should macro implementations be included?
2. How should cross-references be handled?
3. Should there be options to include private items?
4. How should documentation examples be handled?
5. Should type aliases be expanded or left as-is?

# Future possibilities
1. Add structured metadata for AI consumption
2. Add cross-reference resolution
