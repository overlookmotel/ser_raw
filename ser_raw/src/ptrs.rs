use std::mem;

use crate::{Serialize, Serializer};

impl<T: Serialize<S>, S: Serializer> Serialize<S> for Box<T> {
	fn serialize_data(&self, serializer: &mut S) {
		// No need to do anything if box contains ZST
		if mem::size_of::<T>() == 0 {
			return;
		}

		// Write boxed value
		serializer.push_and_process(&**self, |serializer| {
			// Serialize boxed value
			(**self).serialize_data(serializer);
		});
	}
}

impl<T: Serialize<S>, S: Serializer> Serialize<S> for Vec<T> {
	fn serialize_data(&self, serializer: &mut S) {
		// No need to do anything if vec contains ZSTs
		if mem::size_of::<T>() == 0 {
			return;
		}

		// No need to write contents if vec is empty
		if self.len() == 0 {
			return;
		}

		// Write vec's contents
		serializer.push_and_process_slice(self.as_slice(), |serializer| {
			// Serialize vec's contents
			for value in &**self {
				value.serialize_data(serializer);
			}
		});
	}
}

impl<S: Serializer> Serialize<S> for String {
	fn serialize_data(&self, serializer: &mut S) {
		// No need to write contents if string is empty
		if self.len() == 0 {
			return;
		}

		// Write string's content
		serializer.push_slice(self.as_bytes());
	}
}
