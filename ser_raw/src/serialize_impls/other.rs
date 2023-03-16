use crate::{Serialize, Serializer};

impl<T, S> Serialize<S> for Option<T>
where
	S: Serializer,
	T: Serialize<S>,
{
	fn serialize_data(&self, serializer: &mut S) {
		if let Some(value) = self {
			value.serialize_data(serializer);
		}
	}
}
