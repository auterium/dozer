use lmdb::{RoCursor, RwTransaction, Transaction};

use crate::{errors::StorageError, lmdb_storage::CreateDatabase, KeyIterator, LmdbKey, LmdbMap};

#[derive(Debug)]
pub struct LmdbSet<K>(LmdbMap<K, Vec<u8>>);

impl<K> Clone for LmdbSet<K> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<K> Copy for LmdbSet<K> {}

impl<K: LmdbKey> LmdbSet<K> {
    pub fn new<C: CreateDatabase>(
        c: &mut C,
        name: Option<&str>,
        create_if_not_exist: bool,
    ) -> Result<Self, StorageError> {
        LmdbMap::new(c, name, create_if_not_exist).map(Self)
    }

    pub fn count<T: Transaction>(&self, txn: &T) -> Result<usize, StorageError> {
        self.0.count(txn)
    }

    pub fn contains<T: Transaction>(
        &self,
        txn: &T,
        key: K::Encode<'_>,
    ) -> Result<bool, StorageError> {
        self.0.get(txn, key).map(|value| value.is_some())
    }

    /// Returns if the key was actually inserted.
    pub fn insert(
        &self,
        txn: &mut RwTransaction,
        key: K::Encode<'_>,
    ) -> Result<bool, StorageError> {
        self.0.insert(txn, key, &[])
    }

    /// Returns if the key was actually removed.
    pub fn remove(
        &self,
        txn: &mut RwTransaction,
        key: K::Encode<'_>,
    ) -> Result<bool, StorageError> {
        self.0.remove(txn, key)
    }

    pub fn clear(&self, txn: &mut RwTransaction) -> Result<(), StorageError> {
        self.0.clear(txn)
    }

    pub fn iter<'txn, T: Transaction>(
        &self,
        txn: &'txn T,
    ) -> Result<KeyIterator<'txn, RoCursor<'txn>, K>, StorageError> {
        self.0.keys(txn)
    }
}

impl<'a, K: LmdbKey + 'a> LmdbSet<K> {
    /// Extend the set with the contents of an iterator.
    ///
    /// Keys that exist in the map before insertion are ignored.
    pub fn extend(
        &self,
        txn: &mut RwTransaction,
        iter: impl IntoIterator<Item = K::Encode<'a>>,
    ) -> Result<(), StorageError> {
        self.0
            .extend(txn, iter.into_iter().map(|k| (k, [].as_slice())))
    }
}

#[cfg(test)]
mod tests {
    use dozer_types::borrow::IntoOwned;
    use tempdir::TempDir;

    use crate::lmdb_storage::{LmdbEnvironmentManager, LmdbEnvironmentOptions};

    use super::*;

    #[test]
    fn test_lmdb_set() {
        let temp_dir = TempDir::new("test_lmdb_set").unwrap();
        let mut env = LmdbEnvironmentManager::create(
            temp_dir.path(),
            "test",
            LmdbEnvironmentOptions::default(),
        )
        .unwrap();
        let set = LmdbSet::<u32>::new(&mut env, Some("test"), true).unwrap();

        let txn = env.create_txn().unwrap();
        let mut txn = txn.write();

        assert_eq!(set.count(txn.txn()).unwrap(), 0);

        set.insert(txn.txn_mut(), &1).unwrap();

        assert_eq!(set.count(txn.txn()).unwrap(), 1);

        assert!(set.contains(txn.txn(), &1).unwrap());
        assert!(!set.contains(txn.txn(), &2).unwrap());

        assert!(!set.remove(txn.txn_mut(), &2).unwrap());
        assert!(set.remove(txn.txn_mut(), &1).unwrap());
        assert_eq!(set.count(txn.txn()).unwrap(), 0);

        set.extend(txn.txn_mut(), [&5, &4, &3]).unwrap();
        assert_eq!(
            set.iter(txn.txn())
                .unwrap()
                .map(|result| result.map(IntoOwned::into_owned))
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            vec![3, 4, 5]
        );

        set.clear(txn.txn_mut()).unwrap();
        assert_eq!(
            set.iter(txn.txn())
                .unwrap()
                .map(|result| result.map(IntoOwned::into_owned))
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            Vec::<u32>::new()
        );
    }
}
