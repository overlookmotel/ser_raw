use std::{borrow::BorrowMut, slice};

use crate::{storage::Storage, Serialize};

/// Serializers implement this trait.
///
/// Implementations only need to provide the following methods at minimum:
/// * `storage`
/// * `storage_mut`
/// * `from_storage`
/// * `into_storage`
///
/// Default implementation forwards all other method calls to the underlying
/// `Storage`.
pub trait Serializer<Store, BorrowedStore>: Sized
where
	Store: Storage,
	BorrowedStore: BorrowMut<Store>,
{
	/// Get immutable ref to `Storage` backing this `Serializer`.
	fn storage(&self) -> &Store;

	/// Get mutable ref to `Storage` backing this `Serializer`.
	fn storage_mut(&mut self) -> &mut Store;

	/// Create new `Serializer` using an existing `BorrowMut<Storage>`.
	fn from_storage(storage: BorrowedStore) -> Self;

	/// Consume Serializer and return the backing storage as a
	/// `BorrowMut<Storage>`.
	fn into_storage(self) -> BorrowedStore;

	/// Serialize a value and all its dependencies.
	///
	/// The entry point for serializing, which user will call.
	// Serialize<Ser, Store, BorrowedStore>
	fn serialize_value<T: Serialize<Self, Store, BorrowedStore>>(&mut self, t: &T) {
		self.push_raw(t);
		t.serialize_data(self);
	}

	/// Push a value to output.
	///
	/// This is a value in a separate allocation, reached by a pointer
	/// (e.g. `Box<T>`), where `T` does not need further serialization.
	/// If `T` does need further serialization, use `push_and_process` instead.
	///
	/// Some Serializers may record/overwrite the pointer address.
	#[inline]
	fn push<T>(&mut self, value: &T) {
		self.push_slice(slice::from_ref(value));
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
	fn push_slice<T>(&mut self, slice: &[T]) {
		self.push_and_process_slice(slice, |_| {});
	}

	/// Push a value to output and continue processing the value.
	///
	/// This is a value in a separate allocation, reached by a pointer
	/// (e.g. `Box<T>`).
	///
	/// Some Serializers may record/overwrite the pointer address.
	#[inline]
	fn push_and_process<T, P: FnOnce(&mut Self)>(&mut self, t: &T, process: P) {
		self.push_and_process_slice(slice::from_ref(t), process);
	}

	/// Push a slice of values to output and continue processing content of the
	/// slice.
	///
	/// This is a slice in a separate allocation, reached by a pointer
	/// (e.g. `Vec<T>`).
	///
	/// Some Serializers may record/overwrite the pointer address.
	#[inline]
	fn push_and_process_slice<T, P: FnOnce(&mut Self)>(&mut self, slice: &[T], process: P) {
		self.push_raw_slice(slice);
		process(self);
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
	/// ```
	/// struct MyStringProxy;
	/// impl<Ser, Store, BorrowedStore> SerializeWith<MyString, Ser, Store, BorrowedStore>
	/// 	for MyStringProxy
	/// where Ser: Serializer<Store, BorrowedStore>,
	/// 	Store: Storage,
	/// 	BorrowedStore: BorrowMut<Store>
	/// {
	///   fn serialize_data_with(my_str: &MyString, serializer: &mut Ser) {
	///     // Serializer may record pointer to this
	///     serializer.push(&my_str.len());
	///     // No need to record pointer to this, as it's deductible from pointer to `len`
	///     serializer.push_bytes(my_str.as_slice());
	///   }
	/// }
	/// ```
	#[inline]
	fn push_bytes(&mut self, bytes: &[u8]) {
		self.storage_mut().push_bytes(bytes);
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

/// Serializers which can create their own `Storage` implement this trait.
pub trait InstantiableSerializer<Store>: Serializer<Store, Store>
where Store: Storage
{
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
