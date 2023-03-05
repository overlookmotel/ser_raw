#[cfg(feature = "derive")]
pub use ser_raw_derive::Serialize;

mod serializer;
pub use serializer::Serializer;
mod base;
pub use base::{align_up_to, is_aligned_to, BaseSerializer};
mod aligned_vec;
pub use aligned_vec::AlignedByteVec;
mod unaligned_serializer;
pub use unaligned_serializer::UnalignedSerializer;

mod other;
mod primitives;
mod ptrs;

pub trait Serialize {
	#[allow(unused_variables)]
	#[inline(always)]
	fn serialize_data<S: Serializer>(&self, serializer: &mut S) {}
}

pub trait SerializeWith<T> {
	fn serialize_data_with<S: Serializer>(t: &T, serializer: &mut S) -> ();
}
