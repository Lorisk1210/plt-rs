//! FreeType's numeric hash table (`fthash.c`), kept because the
//! autohinter iterates one in bucket order: open addressing probing
//! downwards from FreeType's "Mocklisp" hash, starting with 241 buckets
//! and doubling once a third of them is used.

/// A map from integer keys to values with FreeType's iteration order.
#[derive(Clone, Debug)]
pub(crate) struct NumHash {
    buckets: Vec<Option<(i32, usize)>>,
    used: usize,
}

impl NumHash {
    const INITIAL_SIZE: usize = 241;

    pub(crate) fn new() -> Self {
        Self {
            buckets: vec![None; Self::INITIAL_SIZE],
            used: 0,
        }
    }

    fn hash(key: i32) -> u64 {
        let num = i64::from(key) as u64;
        let mut res = num & 0xFF;
        for shift in [8, 16, 24] {
            res = (res << 5)
                .wrapping_sub(res)
                .wrapping_add((num >> shift) & 0xFF);
        }
        res
    }

    /// The bucket holding `key`, or the empty bucket where it would go.
    fn bucket(&self, key: i32) -> usize {
        let size = self.buckets.len();
        let mut index = (Self::hash(key) % size as u64) as usize;
        while let Some((k, _)) = self.buckets[index] {
            if k == key {
                break;
            }
            index = index.checked_sub(1).unwrap_or(size - 1);
        }
        index
    }

    /// Inserts `key`; an existing key keeps its value unless `overwrite`.
    pub(crate) fn insert(&mut self, key: i32, value: usize, overwrite: bool) {
        let index = self.bucket(key);
        match &mut self.buckets[index] {
            Some((_, old)) => {
                if overwrite {
                    *old = value;
                }
            }
            empty @ None => {
                *empty = Some((key, value));
                if self.used >= self.buckets.len() / 3 {
                    self.rehash();
                }
                self.used += 1;
            }
        }
    }

    fn rehash(&mut self) {
        let old = std::mem::replace(&mut self.buckets, vec![None; 0]);
        self.buckets = vec![None; old.len() * 2];
        for (key, value) in old.into_iter().flatten() {
            let index = self.bucket(key);
            self.buckets[index] = Some((key, value));
        }
    }

    pub(crate) fn get(&self, key: i32) -> Option<usize> {
        self.buckets[self.bucket(key)].map(|(_, value)| value)
    }

    /// The entries in bucket order (`ft_hash_num_iterator`).
    pub(crate) fn iter(&self) -> impl Iterator<Item = (i32, usize)> + '_ {
        self.buckets.iter().flatten().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iterates_in_bucket_order_and_grows() {
        let mut hash = NumHash::new();
        hash.insert(3, 30, true);
        hash.insert(2, 20, true);
        hash.insert(3, 31, false);
        assert_eq!(hash.get(3), Some(30));
        hash.insert(3, 32, true);
        assert_eq!(hash.get(3), Some(32));
        assert_eq!(hash.iter().collect::<Vec<_>>(), [(2, 20), (3, 32)]);
        for key in 0..1000 {
            hash.insert(key, key as usize, false);
        }
        assert_eq!(hash.iter().count(), 1000);
        assert_eq!(hash.get(999), Some(999));
        assert_eq!(hash.get(1000), None);
        assert_eq!(NumHash::hash(0x0102_0304), 4 * 29791 + 3 * 961 + 2 * 31 + 1);
    }
}
