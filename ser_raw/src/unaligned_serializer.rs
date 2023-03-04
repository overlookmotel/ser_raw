use std::{mem, slice};

use crate::Serializer;

/// Serializer which does not respect alignment in the output.
///
/// Values are likely not be aligned as their types require.
///
/// If most of the allocated types you're serializing share the
/// same alignment, performance of `BaseSerializer`, which
/// does respect alignment, is likely to be almost exactly the same.
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
	#[inline]
	fn push_slice<T: Serialize>(&mut self, slice: &[T]) {
		unsafe { self.push_slice_raw(slice) };
	}

	#[inline]
	fn push_bytes(&mut self, bytes: &[u8]) {
		self.buf.extend_from_slice(bytes);
	}

	#[inline]
	unsafe fn push_slice_raw<T>(&mut self, slice: &[T]) {
		let ptr = slice.as_ptr() as *const u8;
		let bytes = slice::from_raw_parts(ptr, slice.len() * mem::size_of::<T>());
		self.push_bytes(bytes);
	}
}
