#[cfg(feature = "derive")]
pub use ser_raw_derive::Serialize;

mod serializer;
pub use serializer::Serializer;
mod aligned_vec;
pub use aligned_vec::AlignedByteVec;
mod aligned_serializer;
pub use aligned_serializer::AlignedSerializer;
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
