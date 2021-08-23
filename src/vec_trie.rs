/// A trie with the following properties:
///
/// - It is stored in a Vec, to avoid allocation.
/// - You can only add nodes, not delete them. This allows for a simpler and more compact data
///   structure. (While you cannot delete individual nodes, you can clear() the whole trie.)
/// - You can only navigate the trie down and foward, not up or backward, because we do not need
///   to.
/// - It does not use generational indices, so the ABA problem is present. But this crate is the
///   only use site, and its simple and trusted, so it makes sense to take this performance
///   improvement.
///
/// Each node in the trie contains a `V`, and each edge is labelled with a `K`. Every node is given
/// an [Index] that uniquely identifies it.
///
/// # Panics
///
/// Don't use an [Index] after the trie has been `clear()`ed! You will either get the wrong index,
/// or a panic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VecTrie<K: Eq, V: Default>(Vec<Node<K, V>>);

/// An index into a [VecTrie]. Only valid until the next call to `clear()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Index(usize);

#[derive(Debug, Clone, PartialEq, Eq)]
struct Node<K: Eq, V: Default> {
    key: K,
    value: V,
    first_child: Option<usize>,
    next_sibling: Option<usize>,
}

impl<K: Eq, V: Default> VecTrie<K, V> {
    /// Construct a new empty trie.
    pub fn new() -> VecTrie<K, V> {
        VecTrie(Vec::new())
    }

    /// View the root of the trie, if any. The [Visitor] will let you navigate to children as well.
    pub fn root(&self) -> Option<Visitor<K, V>> {
        self.0.get(0).map(|_| Visitor {
            trie: self,
            index: 0,
        })
    }

    pub fn value_mut(&mut self, node_idx: Index) -> &mut V {
        &mut self.0[node_idx.0].value
    }

    /// If there is already a matching child, return its index. Otherwise insert a new child.
    ///
    /// # Panics
    ///
    /// Panics if `parent_idx` is `None`, but the trie is non-empty. Only the root lacks a parent!
    pub fn insert_child(&mut self, parent_idx: Option<Index>, key: K) -> Index {
        if let Some(parent_idx) = parent_idx {
            if let Some(mut child_idx) = self.0[parent_idx.0].first_child {
                if self.0[child_idx].key == key {
                    return Index(child_idx);
                }
                while let Some(idx) = self.0[child_idx].next_sibling {
                    if self.0[idx].key == key {
                        return Index(idx);
                    }
                    child_idx = idx;
                }
                let new_child_idx = self.push_new_node(key);
                self.0[child_idx].next_sibling = Some(new_child_idx);
                Index(new_child_idx)
            } else {
                let new_child_idx = self.push_new_node(key);
                self.0[parent_idx.0].first_child = Some(new_child_idx);
                Index(new_child_idx)
            }
        } else {
            assert!(self.0.is_empty());
            let idx = self.push_new_node(key);
            Index(idx)
        }
    }

    /// Empty out the entire Trie. Nothing will remain.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    fn push_new_node(&mut self, key: K) -> usize {
        let child = Node {
            key,
            value: V::default(),
            first_child: None,
            next_sibling: None,
        };
        self.0.push(child);
        self.0.len() - 1
    }
}

/// Lets you navigate the trie.
pub struct Visitor<'a, K: Eq, V: Default> {
    trie: &'a VecTrie<K, V>,
    index: usize,
}

impl<'a, K: Eq, V: Default> Visitor<'a, K, V> {
    pub fn key(&self) -> &'a K {
        &self.trie.0[self.index].key
    }

    pub fn value(&self) -> &'a V {
        &self.trie.0[self.index].value
    }

    /// Returns an `Iterator<Item = Visitor>`.
    pub fn children(&self) -> VisitorIter<'a, K, V> {
        VisitorIter {
            trie: self.trie,
            index: self.trie.0[self.index].first_child,
        }
    }
}

/// An Iterator of [Visitor]s.
pub struct VisitorIter<'a, K: Eq, V: Default> {
    trie: &'a VecTrie<K, V>,
    index: Option<usize>,
}

impl<'a, K: Eq, V: Default> Iterator for VisitorIter<'a, K, V> {
    type Item = Visitor<'a, K, V>;

    fn next(&mut self) -> Option<Visitor<'a, K, V>> {
        if let Some(index) = self.index {
            self.index = self.trie.0[index].next_sibling;
            Some(Visitor {
                trie: self.trie,
                index,
            })
        } else {
            None
        }
    }
}
