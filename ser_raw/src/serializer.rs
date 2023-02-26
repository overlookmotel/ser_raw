use crate::Serialize;

pub trait Serializer: Sized {
	fn serialize_value<T: Serialize>(&mut self, t: &T) -> ();
	fn push_bytes(&mut self, bytes: &[u8]) -> ();
}
