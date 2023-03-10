#[cfg(feature = "derive")]
pub use ser_raw_derive::Serialize;

mod serializer;
pub use serializer::Serializer;
mod base;
pub use base::BaseSerializer;
mod unaligned_serializer;
pub use unaligned_serializer::UnalignedSerializer;

mod other;
mod primitives;
mod ptrs;

pub mod storage;
pub mod util;

pub trait Serialize<S: Serializer> {
	#[allow(unused_variables)]
	#[inline(always)]
	fn serialize_data(&self, serializer: &mut S) {}
}

pub trait SerializeWith<T, S: Serializer> {
	fn serialize_data_with(t: &T, serializer: &mut S) -> ();
}
