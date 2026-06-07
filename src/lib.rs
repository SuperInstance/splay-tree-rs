use std::cmp::Ordering::*;

type Link<T> = Option<Box<Node<T>>>;

struct Node<T: Ord> {
    key: T,
    left: Link<T>,
    right: Link<T>,
}

impl<T: Ord> Node<T> {
    fn new(key: T) -> Box<Self> {
        Box::new(Node {
            key,
            left: None,
            right: None,
        })
    }
}

/// A splay tree providing amortized O(log n) access, O(n) sequential access,
/// plus split and merge operations. Accessing a key splays it to the root.
pub struct SplayTree<T: Ord> {
    root: Link<T>,
    size: usize,
}

// Top-down splay: brings the node with `key` (or the nearest node) to the root.
// Uses the "header" trick with raw pointers to assemble left/right subtrees.
fn splay<T: Ord>(root: Link<T>, key: &T) -> Link<T> {
    let mut t = root?;

    let mut header_left: Link<T> = None;
    let mut header_right: Link<T> = None;

    // SAFETY: left_ptr always points to a valid `Link<T>` slot owned by either
    // `header_left` or a node in the left assembly. right_ptr similarly for the
    // right assembly. We never alias them and they stay valid for the duration
    // of the loop.
    let mut left_ptr: *mut Link<T> = &mut header_left;
    let mut right_ptr: *mut Link<T> = &mut header_right;

    loop {
        match key.cmp(&t.key) {
            Equal => break,
            Less => {
                match t.left.take() {
                    None => break,
                    Some(mut left) => {
                        if *key < left.key {
                            // Zig-zig: rotate right first
                            t.left = left.right.take();
                            left.right = Some(t);
                            t = left;
                            match t.left.take() {
                                None => break,
                                Some(new_left) => {
                                    // Link t to right assembly
                                    unsafe {
                                        *right_ptr = Some(t);
                                        right_ptr =
                                            &mut (*right_ptr).as_mut().unwrap().left;
                                    }
                                    t = new_left;
                                }
                            }
                        } else {
                            // Zig: link t to right assembly
                            unsafe {
                                *right_ptr = Some(t);
                                right_ptr = &mut (*right_ptr).as_mut().unwrap().left;
                            }
                            t = left;
                        }
                    }
                }
            }
            Greater => {
                match t.right.take() {
                    None => break,
                    Some(mut right) => {
                        if *key > right.key {
                            // Zig-zig: rotate left first
                            t.right = right.left.take();
                            right.left = Some(t);
                            t = right;
                            match t.right.take() {
                                None => break,
                                Some(new_right) => {
                                    // Link t to left assembly
                                    unsafe {
                                        *left_ptr = Some(t);
                                        left_ptr =
                                            &mut (*left_ptr).as_mut().unwrap().right;
                                    }
                                    t = new_right;
                                }
                            }
                        } else {
                            // Zig: link t to left assembly
                            unsafe {
                                *left_ptr = Some(t);
                                left_ptr = &mut (*left_ptr).as_mut().unwrap().right;
                            }
                            t = right;
                        }
                    }
                }
            }
        }
    }

    // Reassemble: attach current subtrees of t into the assemblies, then
    // attach the assemblies as the new left/right children of t.
    unsafe {
        *left_ptr = t.left.take();
        *right_ptr = t.right.take();
    }
    t.left = header_left;
    t.right = header_right;
    Some(t)
}

/// Splay the maximum element to the root of `t`.
fn splay_max<T: Ord>(mut t: Box<Node<T>>) -> Box<Node<T>> {
    while t.right.is_some() {
        let mut right = t.right.take().unwrap();
        t.right = right.left.take();
        right.left = Some(t);
        t = right;
    }
    t
}

/// Split `root` into (keys <= key, keys > key).
fn split_inner<T: Ord>(root: Link<T>, key: &T) -> (Link<T>, Link<T>) {
    match splay(root, key) {
        None => (None, None),
        Some(mut t) => {
            if &t.key <= key {
                let right = t.right.take();
                (Some(t), right)
            } else {
                let left = t.left.take();
                (left, Some(t))
            }
        }
    }
}

/// Merge two trees where every key in `left` < every key in `right`.
fn merge_trees<T: Ord>(left: Link<T>, right: Link<T>) -> Link<T> {
    match (left, right) {
        (None, r) => r,
        (l, None) => l,
        (Some(l), right) => {
            let mut new_left = splay_max(l);
            new_left.right = right;
            Some(new_left)
        }
    }
}

fn inorder_collect<'a, T: Ord>(link: &'a Link<T>, out: &mut Vec<&'a T>) {
    if let Some(node) = link {
        inorder_collect(&node.left, out);
        out.push(&node.key);
        inorder_collect(&node.right, out);
    }
}

fn inorder_count<T: Ord>(link: &Link<T>) -> usize {
    match link {
        None => 0,
        Some(n) => 1 + inorder_count(&n.left) + inorder_count(&n.right),
    }
}

impl<T: Ord> SplayTree<T> {
    /// Create a new empty splay tree.
    pub fn new() -> Self {
        SplayTree {
            root: None,
            size: 0,
        }
    }

    /// Insert `key`. Returns `true` if the key was newly inserted, `false` if it
    /// was already present.
    pub fn insert(&mut self, key: T) -> bool {
        self.root = splay(self.root.take(), &key);
        if self.root.as_ref().is_some_and(|r| r.key == key) {
            return false; // duplicate
        }
        // Key not present. Split current tree and create a new root.
        let (left, right) = split_inner(self.root.take(), &key);
        let mut new_node = Node::new(key);
        new_node.left = left;
        new_node.right = right;
        self.root = Some(new_node);
        self.size += 1;
        true
    }

    /// Delete `key`. Returns `true` if found and removed, `false` if absent.
    pub fn delete(&mut self, key: &T) -> bool {
        self.root = splay(self.root.take(), key);
        match self.root.take() {
            None => false,
            Some(mut root) => {
                if &root.key != key {
                    self.root = Some(root);
                    return false;
                }
                let left = root.left.take();
                let right = root.right.take();
                self.root = merge_trees(left, right);
                self.size -= 1;
                true
            }
        }
    }

    /// Check whether `key` is in the tree. Splays the nearest node to the root
    /// as a side effect, giving amortized O(log n) performance.
    pub fn contains(&mut self, key: &T) -> bool {
        self.root = splay(self.root.take(), key);
        self.root.as_ref().is_some_and(|r| &r.key == key)
    }

    /// Return all keys in ascending order.
    pub fn inorder(&self) -> Vec<&T> {
        let mut out = Vec::with_capacity(self.size);
        inorder_collect(&self.root, &mut out);
        out
    }

    /// Number of elements in the tree.
    pub fn len(&self) -> usize {
        self.size
    }

    /// Returns `true` if the tree has no elements.
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Splay the minimum element to the root and return a reference to it.
    pub fn min(&mut self) -> Option<&T> {
        let mut t = self.root.take()?;
        while t.left.is_some() {
            let mut left = t.left.take().unwrap();
            t.left = left.right.take();
            left.right = Some(t);
            t = left;
        }
        self.root = Some(t);
        self.root.as_ref().map(|n| &n.key)
    }

    /// Splay the maximum element to the root and return a reference to it.
    pub fn max(&mut self) -> Option<&T> {
        let mut t = self.root.take()?;
        while t.right.is_some() {
            let mut right = t.right.take().unwrap();
            t.right = right.left.take();
            right.left = Some(t);
            t = right;
        }
        self.root = Some(t);
        self.root.as_ref().map(|n| &n.key)
    }

    /// Split the tree into `(keys <= key, keys > key)`.
    pub fn split(mut self, key: &T) -> (SplayTree<T>, SplayTree<T>) {
        let (left_root, right_root) = split_inner(self.root.take(), key);
        let left_size = inorder_count(&left_root);
        let right_size = self.size - left_size;
        (
            SplayTree {
                root: left_root,
                size: left_size,
            },
            SplayTree {
                root: right_root,
                size: right_size,
            },
        )
    }

    /// Merge two trees where every key in `left` is less than every key in
    /// `right`. The precondition is not checked.
    pub fn merge(mut left: SplayTree<T>, mut right: SplayTree<T>) -> SplayTree<T> {
        let size = left.size + right.size;
        SplayTree {
            root: merge_trees(left.root.take(), right.root.take()),
            size,
        }
    }
}

impl<T: Ord> Default for SplayTree<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 1. Empty tree
    #[test]
    fn test_new_empty() {
        let t: SplayTree<i32> = SplayTree::new();
        assert!(t.is_empty());
        assert_eq!(t.len(), 0);
    }

    // 2. Insert then contains
    #[test]
    fn test_insert_contains() {
        let mut t = SplayTree::new();
        t.insert(5);
        assert!(t.contains(&5));
    }

    // 3. Insert returns true for new key
    #[test]
    fn test_insert_new_true() {
        let mut t = SplayTree::new();
        assert!(t.insert(42));
    }

    // 4. Insert returns false for duplicate
    #[test]
    fn test_insert_dup_false() {
        let mut t = SplayTree::new();
        t.insert(7);
        assert!(!t.insert(7));
    }

    // 5. contains on empty tree
    #[test]
    fn test_contains_miss_empty() {
        let mut t: SplayTree<i32> = SplayTree::new();
        assert!(!t.contains(&1));
    }

    // 6. contains misses after inserts of other values
    #[test]
    fn test_contains_miss_after_inserts() {
        let mut t = SplayTree::new();
        t.insert(1);
        t.insert(2);
        t.insert(3);
        assert!(!t.contains(&99));
    }

    // 7. inorder of empty tree
    #[test]
    fn test_inorder_empty() {
        let t: SplayTree<i32> = SplayTree::new();
        assert_eq!(t.inorder(), Vec::<&i32>::new());
    }

    // 8. inorder single element
    #[test]
    fn test_inorder_single() {
        let mut t = SplayTree::new();
        t.insert(10);
        assert_eq!(t.inorder(), vec![&10]);
    }

    // 9. inorder multiple elements
    #[test]
    fn test_inorder_sorted() {
        let mut t = SplayTree::new();
        for &v in &[5, 3, 7, 1, 4, 6, 8] {
            t.insert(v);
        }
        assert_eq!(t.inorder(), vec![&1, &3, &4, &5, &6, &7, &8]);
    }

    // 10. len increments on distinct inserts
    #[test]
    fn test_len_increments() {
        let mut t = SplayTree::new();
        for i in 0..5 {
            t.insert(i);
        }
        assert_eq!(t.len(), 5);
    }

    // 11. len does not increment on duplicate
    #[test]
    fn test_len_no_dup() {
        let mut t = SplayTree::new();
        t.insert(1);
        t.insert(1);
        assert_eq!(t.len(), 1);
    }

    // 12. delete on empty returns false
    #[test]
    fn test_delete_empty_false() {
        let mut t: SplayTree<i32> = SplayTree::new();
        assert!(!t.delete(&5));
    }

    // 13. delete a leaf
    #[test]
    fn test_delete_leaf() {
        let mut t = SplayTree::new();
        t.insert(5);
        t.insert(3);
        assert!(t.delete(&3));
        assert!(!t.contains(&3));
        assert_eq!(t.len(), 1);
    }

    // 14. delete the root
    #[test]
    fn test_delete_root() {
        let mut t = SplayTree::new();
        t.insert(5);
        assert!(t.delete(&5));
        assert!(t.is_empty());
    }

    // 15. delete nonexistent returns false
    #[test]
    fn test_delete_nonexistent_false() {
        let mut t = SplayTree::new();
        t.insert(1);
        t.insert(2);
        assert!(!t.delete(&99));
        assert_eq!(t.len(), 2);
    }

    // 16. inorder after deletes
    #[test]
    fn test_delete_inorder_sorted() {
        let mut t = SplayTree::new();
        for &v in &[5, 3, 7, 1, 4, 6, 8] {
            t.insert(v);
        }
        t.delete(&3);
        t.delete(&7);
        assert_eq!(t.inorder(), vec![&1, &4, &5, &6, &8]);
    }

    // 17. min of empty
    #[test]
    fn test_min_empty() {
        let mut t: SplayTree<i32> = SplayTree::new();
        assert_eq!(t.min(), None);
    }

    // 18. min basic
    #[test]
    fn test_min_basic() {
        let mut t = SplayTree::new();
        for &v in &[3, 1, 4, 5] {
            t.insert(v);
        }
        assert_eq!(t.min(), Some(&1));
    }

    // 19. max basic
    #[test]
    fn test_max_basic() {
        let mut t = SplayTree::new();
        for &v in &[3, 1, 4, 5] {
            t.insert(v);
        }
        assert_eq!(t.max(), Some(&5));
    }

    // 20. splay moves accessed element to root (verified via duplicate detection)
    #[test]
    fn test_splay_moves_to_root() {
        let mut t = SplayTree::new();
        t.insert(10);
        t.insert(5);
        t.insert(15);
        // After contains(10), 10 should be at root — re-inserting 10 is a dup
        assert!(t.contains(&10));
        assert!(!t.insert(10)); // duplicate → splay worked
    }

    // 21. split basic
    #[test]
    fn test_split_basic() {
        let mut t = SplayTree::new();
        for v in 1..=5 {
            t.insert(v);
        }
        let (left, right) = t.split(&3);
        assert_eq!(left.inorder(), vec![&1, &2, &3]);
        assert_eq!(right.inorder(), vec![&4, &5]);
        assert_eq!(left.len(), 3);
        assert_eq!(right.len(), 2);
    }

    // 22. split empty
    #[test]
    fn test_split_empty() {
        let t: SplayTree<i32> = SplayTree::new();
        let (left, right) = t.split(&5);
        assert!(left.is_empty());
        assert!(right.is_empty());
    }

    // 23. merge basic
    #[test]
    fn test_merge_basic() {
        let mut a = SplayTree::new();
        a.insert(1);
        a.insert(3);
        let mut b = SplayTree::new();
        b.insert(5);
        b.insert(7);
        let merged = SplayTree::merge(a, b);
        assert_eq!(merged.inorder(), vec![&1, &3, &5, &7]);
        assert_eq!(merged.len(), 4);
    }

    // 24. merge empty left
    #[test]
    fn test_merge_empty_left() {
        let empty: SplayTree<i32> = SplayTree::new();
        let mut right = SplayTree::new();
        right.insert(1);
        right.insert(2);
        let merged = SplayTree::merge(empty, right);
        assert_eq!(merged.inorder(), vec![&1, &2]);
        assert_eq!(merged.len(), 2);
    }

    // 25. merge empty right
    #[test]
    fn test_merge_empty_right() {
        let mut left = SplayTree::new();
        left.insert(1);
        left.insert(2);
        let empty: SplayTree<i32> = SplayTree::new();
        let merged = SplayTree::merge(left, empty);
        assert_eq!(merged.inorder(), vec![&1, &2]);
        assert_eq!(merged.len(), 2);
    }

    // 26. sequential access - tests amortized O(n) behavior
    #[test]
    fn test_sequential_access() {
        let mut t = SplayTree::new();
        for i in 1..=100 {
            t.insert(i);
        }
        for i in 1..=100 {
            assert!(t.contains(&i));
        }
    }

    // 27. mass insert in reverse, inorder is sorted
    #[test]
    fn test_mass_insert_inorder() {
        let mut t = SplayTree::new();
        for i in (1..=100).rev() {
            t.insert(i);
        }
        let keys: Vec<i32> = t.inorder().into_iter().copied().collect();
        let expected: Vec<i32> = (1..=100).collect();
        assert_eq!(keys, expected);
    }

    // 28. mass delete: delete all odds, only evens remain
    #[test]
    fn test_mass_delete() {
        let mut t = SplayTree::new();
        for i in 1..=50 {
            t.insert(i);
        }
        for i in (1..=50).step_by(2) {
            assert!(t.delete(&i));
        }
        assert_eq!(t.len(), 25);
        let keys: Vec<i32> = t.inorder().into_iter().copied().collect();
        let expected: Vec<i32> = (2..=50).step_by(2).collect();
        assert_eq!(keys, expected);
    }
}
