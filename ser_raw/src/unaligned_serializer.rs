use std::{mem, slice};

use crate::{Serialize, Serializer};

/// Serializer which does not respect alignment in the output.
/// Values may not be aligned as their types require.
///
/// It is NOT recommended to use this. Mainly for testing.
pub struct UnalignedSerializer {
	buf: Vec<u8>,
}

impl UnalignedSerializer {
	/// Create new Serializer without allocating any memory for output buffer.
	/// Memory will be allocated when first object is serialized.
	pub fn new() -> Self {
		UnalignedSerializer { buf: Vec::new() }
	}

	/// Create new Serializer with buffer pre-allocated with capacity of
	/// `capacity` bytes.
	///
	/// If you know, or can estimate, the amount of buffer space that's going to
	/// be needed in advance, allocating upfront with `with_capacity` can
	/// dramatically improve performance vs `new`.
	pub fn with_capacity(capacity: usize) -> Self {
		UnalignedSerializer {
			buf: Vec::with_capacity(capacity),
		}
	}

	/// Consume Serializer and return the output buffer as a `Vec<u8>`.
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
		self.push_bytes(bytes);
	}

	#[inline]
	fn push_slice<T: Serialize>(&mut self, slice: &[T]) {
		let ptr = slice.as_ptr() as *const u8;
		let bytes = unsafe { slice::from_raw_parts(ptr, slice.len() * mem::size_of::<T>()) };
		self.push_bytes(bytes);
	}

	#[inline]
	fn push_bytes(&mut self, bytes: &[u8]) {
		self.buf.extend_from_slice(bytes);
	}
}
