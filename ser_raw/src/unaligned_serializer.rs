use std::{mem, slice};

use crate::{Serialize, Serializer};

pub struct UnalignedSerializer {
	buf: Vec<u8>,
}

impl UnalignedSerializer {
	pub fn new() -> Self {
		UnalignedSerializer { buf: Vec::new() }
	}

	pub fn with_capacity(capacity: usize) -> Self {
		UnalignedSerializer {
			buf: Vec::with_capacity(capacity),
		}
	}

	pub fn into_vec(self) -> Vec<u8> {
		self.buf
	}
}

impl Serializer for UnalignedSerializer {
	fn serialize_value<T: Serialize>(&mut self, t: &T) {
		self.push(t);
		t.serialize_data(self);
	}

	#[inline]
	fn push<T: Serialize>(&mut self, t: &T) {
		let ptr = t as *const T as *const u8;
		let bytes = unsafe { slice::from_raw_parts(ptr, mem::size_of::<T>()) };
		self.buf.extend_from_slice(bytes);
	}

	#[inline]
	fn push_slice<T: Serialize>(&mut self, slice: &[T]) {
		let ptr = slice.as_ptr() as *const u8;
		let bytes = unsafe { slice::from_raw_parts(ptr, slice.len() * mem::size_of::<T>()) };
		self.buf.extend_from_slice(bytes);
	}
}
