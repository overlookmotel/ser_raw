use crate::{Serialize, Serializer};

impl<T: Serialize<S>, S: Serializer> Serialize<S> for Option<T> {
	fn serialize_data(&self, serializer: &mut S) {
		if let Some(value) = self {
			value.serialize_data(serializer);
		}
	}
}
