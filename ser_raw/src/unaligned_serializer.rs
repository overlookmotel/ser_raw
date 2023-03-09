use std::{borrow::BorrowMut, mem, slice};

use crate::Serializer;

// TODO: Should use `Vec<MaybeUninit<u8>>` not `Vec<u8>` as output likely
// includes uninitialized padding bytes

/// Serializer which does not respect alignment in the output.
///
/// Values are likely not be aligned as their types require.
///
/// If most of the allocated types you're serializing share the
/// same alignment, performance of `BaseSerializer`, which
/// does respect alignment, is likely to be almost exactly the same.
pub struct UnalignedSerializer<Buf: BorrowMut<Vec<u8>>> {
	buf: Buf,
}

impl UnalignedSerializer<Vec<u8>> {
	/// Create new Serializer without allocating any memory for output buffer.
	/// Memory will be allocated when first object is serialized.
	pub fn new() -> Self {
		Self { buf: Vec::new() }
	}

	/// Create new Serializer with buffer pre-allocated with capacity of
	/// `capacity` bytes.
	///
	/// If you know, or can estimate, the amount of buffer space that's going to
	/// be needed in advance, allocating upfront with `with_capacity` can
	/// dramatically improve performance vs `new`.
	pub fn with_capacity(capacity: usize) -> Self {
		Self {
			buf: Vec::with_capacity(capacity),
		}
	}
}

impl<Buf: BorrowMut<Vec<u8>>> UnalignedSerializer<Buf> {
	/// Create new Serializer from an existing `Vec<u8>` or `&mut Vec<u8>`.
	pub fn from_vec(buf: Buf) -> Self {
		Self { buf }
	}

	/// Consume Serializer and return the output buffer as a `Vec<u8>`
	/// or `&mut Vec<u8>`.
	pub fn into_vec(self) -> Buf {
		self.buf
	}
}

impl<Buf: BorrowMut<Vec<u8>>> Serializer for UnalignedSerializer<Buf> {
	#[inline]
	fn push_slice<T>(&mut self, slice: &[T]) {
		self.push_slice_raw(slice);
	}

	#[inline]
	fn push_bytes(&mut self, bytes: &[u8]) {
		self.buf.borrow_mut().extend_from_slice(bytes);
	}

	#[inline]
	fn push_slice_raw<T>(&mut self, slice: &[T]) {
		let ptr = slice.as_ptr() as *const u8;
		let bytes = unsafe { slice::from_raw_parts(ptr, slice.len() * mem::size_of::<T>()) };
		self.push_bytes(bytes);
	}

	/// Get current capacity of output.
	#[inline]
	fn capacity(&self) -> usize {
		self.buf.borrow().capacity()
	}

	/// Get current position in output.
	#[inline]
	fn pos(&self) -> usize {
		self.buf.borrow().len()
	}

	/// Move current position in output buffer.
	///
	/// # Safety
	///
	/// * `pos` must be less than or equal to `self.capacity()`.
	#[inline]
	unsafe fn set_pos(&mut self, pos: usize) {
		self.buf.borrow_mut().set_len(pos);
	}
}
