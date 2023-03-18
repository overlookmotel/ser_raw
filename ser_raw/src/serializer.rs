use std::{borrow::BorrowMut, slice};

use crate::{pos::Addr, storage::Storage, Serialize};

/// Serializers implement this trait.
///
/// Implementing a serializer requires multiple trait implementations.
/// It's split into these 5 traits to avoid bounds on the main `Serializer`
/// trait, and to allow composing different parts to create custom serializers.
///
/// A serializer will implement:
///
/// * `Serializer`: Mandatory. Defines serialization behavior.
/// * `SerializerStorage`: Mandatory. Defines how to access to backing storage.
/// * `SerializerWrite`: Mandatory. Defines how to write to backing storage
/// 	at arbitrary locations.
/// * `InstantiableSerializer`: Optional. Defines how to create a new serializer
///   instance along with it's own owned backing `Storage`.
/// * `BorrowingSerializer`: Optional. Defines how to create a new serializer
///   instance which mutably borrows an existing `Storage`.
///
/// Implementations need not define any methods for the `Serializer` trait.
/// Default implementation forwards all method calls to the underlying
/// `Storage`. They only need to define associated type `Addr`.
pub trait Serializer: SerializerStorage + SerializerWrite + Sized {
	/// `Addr` type this serializer uses.
	type Addr: Addr;

	/// Serialize a value and all its dependencies.
	///
	/// The entry point for serializing, which user will call.
	fn serialize_value<T: Serialize<Self>>(&mut self, value: &T) {
		self.push_raw(value);
		value.serialize_data(self);
	}

	/// Push a value to output.
	///
	/// This is a value in a separate allocation, reached by a pointer
	/// (e.g. `Box<T>`), where `T` does not need further serialization.
	/// If `T` does need further serialization, use `push_and_process` instead.
	///
	/// Some Serializers may record/overwrite the pointer address.
	#[inline]
	fn push<T>(&mut self, value: &T, ptr_addr: Self::Addr) {
		self.push_slice(slice::from_ref(value), ptr_addr);
	}

	/// Push a slice of values to output.
	///
	/// This is a slice in a separate allocation, reached by a pointer
	/// (e.g. `Vec<T>`), where `T` does not need further serialization.
	/// If `T` does need further serialization, use `push_and_process_slice`
	/// instead.
	///
	/// Some Serializers may record/overwrite the pointer address.
	#[inline]
	fn push_slice<T>(&mut self, slice: &[T], ptr_addr: Self::Addr) {
		self.push_and_process_slice(slice, ptr_addr, |_| {});
	}

	/// Push a value to output and continue processing the value.
	///
	/// This is a value in a separate allocation, reached by a pointer
	/// (e.g. `Box<T>`).
	///
	/// Some Serializers may record/overwrite the pointer address.
	#[inline]
	fn push_and_process<T, P: FnOnce(&mut Self)>(&mut self, t: &T, ptr_addr: Self::Addr, process: P) {
		self.push_and_process_slice(slice::from_ref(t), ptr_addr, process);
	}

	/// Push a slice of values to output and continue processing content of the
	/// slice.
	///
	/// This is a slice in a separate allocation, reached by a pointer
	/// (e.g. `Vec<T>`).
	///
	/// Some Serializers may record/overwrite the pointer address.
	#[inline]
	fn push_and_process_slice<T, P: FnOnce(&mut Self)>(
		&mut self,
		slice: &[T],
		#[allow(unused_variables)] ptr_addr: Self::Addr,
		process: P,
	) {
		self.push_raw_slice(slice);
		process(self);
	}

	/// Push a value to output.
	///
	/// Unlike `push` and `push_and_process`, this is not for values for which a
	/// Serializer may need to record a pointer address.
	#[inline]
	fn push_raw<T>(&mut self, value: &T) {
		self.push_raw_slice(slice::from_ref(value));
	}

	/// Push a slice of values to output.
	///
	/// Unlike `push_slice` and `push_and_process_slice`, this is not for values
	/// for which a Serializer may need to record a pointer address.
	#[inline]
	fn push_raw_slice<T>(&mut self, slice: &[T]) {
		self.storage_mut().push_slice(slice);
	}

	/// Push raw bytes to output.
	///
	/// Unlike `push`, `push_slice`, `push_and_process` and
	/// `push_and_process_slice`, this is not for values for which a Serializer
	/// may need to record a pointer address.
	///
	/// Mainly for use in custom serialization functions, where output
	/// representation includes multiple parts, and Deserializer only
	/// needs to know the location of the first part.
	///
	/// # Example
	///
	/// ```
	/// use ser_raw::{Serializer, SerializeWith, pos::Addr};
	///
	/// struct MyString { inner: String }
	///
	/// struct MyStringProxy;
	/// impl<S> SerializeWith<MyString, S> for MyStringProxy
	/// where S: Serializer
	/// {
	/// 	fn serialize_data_with(my_str: &MyString, serializer: &mut S) {
	/// 		// Serializer may record pointer to this
	/// 		let ptr_addr = S::Addr::from_ref(&my_str.inner);
	/// 		serializer.push(&my_str.inner.len(), ptr_addr);
	/// 		// No need to record pointer to this, as it's deductible from pointer to `len`
	/// 		serializer.push_raw_bytes(my_str.inner.as_bytes());
	/// 	}
	/// }
	/// ```
	#[inline]
	fn push_raw_bytes(&mut self, bytes: &[u8]) {
		self.storage_mut().push_bytes(bytes);
	}

	/// Get current capacity of output.
	#[inline]
	fn capacity(&self) -> usize {
		self.storage().capacity()
	}

	/// Get current position in output.
	#[inline]
	fn pos(&self) -> usize {
		self.storage().len()
	}
}

/// Trait for accessing backing `Storage` of a `Serializer`.
///
/// `SerializerStorage` is a supertrait of `Serializer`. All serializers must
/// implement this trait.
pub trait SerializerStorage {
	/// `Storage` which backs this serializer.
	type Storage: Storage;
	type BorrowedStorage: BorrowMut<Self::Storage>;

	/// Get immutable ref to `Storage` backing this `Serializer`.
	fn storage(&self) -> &Self::Storage;

	/// Get mutable ref to `Storage` backing this `Serializer`.
	fn storage_mut(&mut self) -> &mut Self::Storage;

	/// Consume Serializer and return the backing storage as a
	/// `BorrowMut<Storage>`.
	fn into_storage(self) -> Self::BorrowedStorage;
}

/// Trait for writing to arbitrary locations in Serializer's storage.
///
/// `SerializerWrite` is a supertrait of `Serializer`. All serializers must
/// implement this trait.
pub trait SerializerWrite {
	/// Write a value to storage at a specific position.
	///
	/// Default implementation is a no-op, and some serializers may not need to
	/// implement a functional version of this, if they don't need/support writing
	/// to storage at arbitrary positions.
	///
	/// A Serializer can also opt to implement this method not by writing to the
	/// storage immediately, but instead storing the details of what need to be
	/// written in some other form, and to leaving it to the deserializer to
	/// perform the writes later.
	///
	/// # Safety
	///
	/// `pos + mem::size_of::<T>()` must be less than or equal to `capacity()`.
	/// i.e. `pos` must be within bounds of the currently allocated storage.
	#[allow(unused_variables)]
	#[inline(always)]
	unsafe fn write<T>(&mut self, value: &T, addr: usize) {
		// TODO: Would be better to take an `Addr`
	}

	/// Write a correction to storage.
	///
	/// An example of a "correction" is: Serializing a `Vec` which has
	/// `capacity` of 2, but `len` of 1. The correction is amending the `capacity`
	/// field to 1, to reflect that the copy of the `Vec` in serialized output
	/// only contains 1 element, and no additional capacity.
	///
	/// Default implementation is a no-op, and some serializers may not need to
	/// implement a functional version of this, if they don't need corrections.
	///
	/// Method takes a closure, so that `Serialize::serialize_data`
	/// implementations can perform operations which may have some cost in the
	/// closure, prior to performing writes. If the `Serializer` doesn't care
	/// about corrections and uses this default no-op implementation of
	/// `write_correction`, the closure will not be called and the cost of those
	/// operations is avoided. Hopefully the compiler will recognise this and
	/// remove the call to `write_correction` and the code inside the closure
	/// entirely, so it's completely zero cost unless it's used.
	///
	/// If Serializer *does* want to receive corrections, it would implement this
	/// method as:
	/// ```ignore
	/// fn write_correction<W: FnOnce(&mut Self)>(&mut self, write: W) {
	/// 	write(self);
	/// }
	/// ```
	#[allow(unused_variables)]
	#[inline(always)]
	fn write_correction<W: FnOnce(&mut Self)>(&mut self, write: W) {}
}

/// Serializers which can create their own owned `Storage` implement this trait.
pub trait InstantiableSerializer: Serializer {
	/// Create new `Serializer` without allocating any memory for output.
	/// Memory will be allocated when first value is serialized.
	///
	/// If you know, or can estimate, the amount of memory that's going to
	/// be needed in advance, allocating upfront with `with_capacity` can
	/// dramatically improve performance vs `new`.
	fn new() -> Self;

	/// Create new `Serializer` with pre-allocated storage with capacity
	/// of `capacity` bytes.
	fn with_capacity(capacity: usize) -> Self;
}

/// Serializers which can be created from a `BorrowMut` of an existing `Storage`
/// implement this trait.
pub trait BorrowingSerializer<BorrowedStorage>: Serializer
where BorrowedStorage: BorrowMut<Self::Storage>
{
	/// Create new `Serializer` using an existing `BorrowMut<Storage>`.
	fn from_storage(storage: BorrowedStorage) -> Self;
}
