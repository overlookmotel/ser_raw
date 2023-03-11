use std::borrow::BorrowMut;

use crate::{storage::Storage, Serializer};

/// Trait for types which can be serialized.
pub trait Serialize<Ser, Store, BorrowedStore>
where
	Ser: Serializer<Store, BorrowedStore>,
	Store: Storage,
	BorrowedStore: BorrowMut<Store>,
{
	#[allow(unused_variables)]
	#[inline(always)]
	fn serialize_data(&self, serializer: &mut Ser) {}
}

/// Trait for use with `#[ser_with]`.
pub trait SerializeWith<T, Ser, Store, BorrowedStore>
where
	Ser: Serializer<Store, BorrowedStore>,
	Store: Storage,
	BorrowedStore: BorrowMut<Store>,
{
	fn serialize_data_with(t: &T, serializer: &mut Ser) -> ();
}
