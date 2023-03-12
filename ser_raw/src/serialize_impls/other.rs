use crate::{Serialize, Serializer};

impl<T, Ser: Serializer> Serialize<Ser> for Option<T>
where T: Serialize<Ser>
{
	fn serialize_data(&self, serializer: &mut Ser) {
		if let Some(value) = self {
			value.serialize_data(serializer);
		}
	}
}
