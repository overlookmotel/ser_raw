#[cfg(feature = "derive")]
pub use ser_raw_derive::Serialize;

mod serializer;
pub use serializer::Serializer;

mod unaligned;
pub use unaligned::UnalignedSerializer;

mod other;
mod primitives;
mod ptrs;

pub trait Serialize {
	#[allow(unused_variables)]
	fn serialize_data<S: Serializer>(&self, serializer: &mut S) {}
}

pub trait SerializeWith<T> {
	#[allow(unused_variables)]
	fn serialize_data_with<S: Serializer>(t: &T, serializer: &mut S) -> ();
}

pub fn serialize_unaligned<T: Serialize>(src: &T) -> Vec<u8> {
	let mut serializer = UnalignedSerializer::new();
	serializer.serialize_value(src);
	let mut vec = serializer.into_vec();
	vec.shrink_to_fit();
	vec
}
