A splay tree implementation in Rust with zig/zig-zig/zig-zag rotations, providing amortized O(log n) access per operation and O(n) total cost for sequential access of n elements.

Supports insert, delete, contains (splays on access), min, max, split, and merge operations, all with a safe public API backed by a top-down splay algorithm using the Sleator-Tarjan header trick.
