use std::borrow::BorrowMut;

// Derive macro for `Serialize`
#[cfg(feature = "derive")]
pub use ser_raw_derive::Serialize;

// Export Serializers, Storage, and utils
mod serializer;
pub use serializer::{InstantiableSerializer, Serializer};
// TODO: Rename to `AlignedSerializer`
mod base;
pub use base::BaseSerializer;
mod unaligned_serializer;
pub use unaligned_serializer::UnalignedSerializer;
pub mod storage;
pub mod util;

// `Serialize` implementations for Rust internal types
mod other;
mod primitives;
mod ptrs;

use storage::Storage;

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
