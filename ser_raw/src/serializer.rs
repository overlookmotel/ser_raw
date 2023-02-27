use crate::Serialize;

pub trait Serializer: Sized {
	fn serialize_value<T: Serialize>(&mut self, t: &T) -> ();
	fn push<T: Serialize>(&mut self, t: &T) -> ();
	fn push_slice<T: Serialize>(&mut self, slice: &[T]) -> ();
}
