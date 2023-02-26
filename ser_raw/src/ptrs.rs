use std::{mem, slice};

use crate::{Serialize, Serializer};

impl<T: Serialize> Serialize for Box<T> {
	fn serialize_data<S: Serializer>(&self, serializer: &mut S) {
		// No need to do anything if box contains ZST
		if mem::size_of::<T>() == 0 {
			return;
		}

		// Write boxed value
		let ptr = (&**self) as *const T as *const u8;
		let bytes = unsafe { slice::from_raw_parts(ptr, mem::size_of::<T>()) };
		serializer.push_bytes(bytes);

		// Serialize boxed value
		(**self).serialize_data(serializer);
	}
}

impl<T: Serialize> Serialize for Vec<T> {
	fn serialize_data<S: Serializer>(&self, serializer: &mut S) {
		// No need to do anything if vec contains ZSTs
		if mem::size_of::<T>() == 0 {
			return;
		}

		// No need to write contents if vec is empty
		if self.len() == 0 {
			return;
		}

		// Write vec's contents
		let ptr = self.as_ptr() as *const u8;
		let bytes = unsafe { slice::from_raw_parts(ptr, self.len() * mem::size_of::<T>()) };
		serializer.push_bytes(bytes);

		// Serialize vec's contents
		for value in &**self {
			value.serialize_data(serializer);
		}
	}
}

impl Serialize for String {
	fn serialize_data<S: Serializer>(&self, serializer: &mut S) {
		// No need to write contents if string is empty
		if self.len() == 0 {
			return;
		}

		// Write string's content
		serializer.push_bytes(self.as_bytes());
	}
}
