use crate::Serializer;

/// Trait for types which can be serialized.
pub trait Serialize<Ser: Serializer> {
	#[allow(unused_variables)]
	#[inline(always)]
	fn serialize_data(&self, serializer: &mut Ser) {}
}

/// Trait for use with `#[ser_with]`.
pub trait SerializeWith<T, Ser: Serializer> {
	fn serialize_data_with(t: &T, serializer: &mut Ser) -> ();
}
