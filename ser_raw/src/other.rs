use crate::{Serialize, Serializer};

impl<T: Serialize> Serialize for Option<T> {
	fn serialize_data<S: Serializer>(&self, serializer: &mut S) {
		if let Some(value) = self {
			value.serialize_data(serializer);
		}
	}
}
