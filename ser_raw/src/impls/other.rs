use std::borrow::BorrowMut;

use crate::{storage::Storage, Serialize, Serializer};

impl<T, Ser, Store, BorrowedStore> Serialize<Ser, Store, BorrowedStore> for Option<T>
where
	T: Serialize<Ser, Store, BorrowedStore>,
	Ser: Serializer<Store, BorrowedStore>,
	Store: Storage,
	BorrowedStore: BorrowMut<Store>,
{
	fn serialize_data(&self, serializer: &mut Ser) {
		if let Some(value) = self {
			value.serialize_data(serializer);
		}
	}
}
