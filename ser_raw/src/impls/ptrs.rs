use std::{borrow::BorrowMut, mem};

use crate::{storage::Storage, Serialize, Serializer};

impl<T, Ser, Store, BorrowedStore> Serialize<Ser, Store, BorrowedStore> for Box<T>
where
	T: Serialize<Ser, Store, BorrowedStore>,
	Ser: Serializer<Store, BorrowedStore>,
	Store: Storage,
	BorrowedStore: BorrowMut<Store>,
{
	fn serialize_data(&self, serializer: &mut Ser) {
		// No need to do anything if box contains ZST
		if mem::size_of::<T>() == 0 {
			return;
		}

		// Write boxed value
		serializer.push_and_process(&**self, |serializer| {
			// Serialize boxed value
			(**self).serialize_data(serializer);
		});
	}
}

impl<T, Ser, Store, BorrowedStore> Serialize<Ser, Store, BorrowedStore> for Vec<T>
where
	T: Serialize<Ser, Store, BorrowedStore>,
	Ser: Serializer<Store, BorrowedStore>,
	Store: Storage,
	BorrowedStore: BorrowMut<Store>,
{
	fn serialize_data(&self, serializer: &mut Ser) {
		// No need to do anything if vec contains ZSTs
		if mem::size_of::<T>() == 0 {
			return;
		}

		// No need to write contents if vec is empty
		if self.len() == 0 {
			return;
		}

		// Write vec's contents
		serializer.push_and_process_slice(self.as_slice(), |serializer| {
			// Serialize vec's contents
			for value in &**self {
				value.serialize_data(serializer);
			}
		});
	}
}

impl<Ser, Store, BorrowedStore> Serialize<Ser, Store, BorrowedStore> for String
where
	Ser: Serializer<Store, BorrowedStore>,
	Store: Storage,
	BorrowedStore: BorrowMut<Store>,
{
	fn serialize_data(&self, serializer: &mut Ser) {
		// No need to write contents if string is empty
		if self.len() == 0 {
			return;
		}

		// Write string's content
		serializer.push_slice(self.as_bytes());
	}
}
