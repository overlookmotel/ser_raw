#![allow(dead_code)]

use crate::{pos::PosMapping, Serializer};

/// Trait for serializers which track position in output.
///
/// Implement the trait on a serializer, and then use macro
/// `impl_pos_tracking_serializer!()` to implement `Serialize`.
///
/// `serialize_data` functions can then get the position of any value's
/// serialized representation in output with `serializer.pos_for(&value)`.
///
/// # Example
///
/// ```ignore
/// use ser_raw::{impl_pos_tracking_serializer, PosTrackingSerializer, SerializerStorage};
///
/// struct MySerializer {}
///
/// impl PosTrackingSerializer for MySerializer {}
/// impl_pos_tracking_serializer!(MySerializer);
///
/// impl SerializerStorage for MySerializer {
/// 	// ...
/// }
/// ```
pub trait PosTrackingSerializer: Serializer {
	/// Get current position mapping
	fn pos_mapping(&self) -> &PosMapping;

	/// Set current position mapping
	fn set_pos_mapping(&mut self, pos_mapping: PosMapping) -> ();

	/// Get position for a value
	#[inline]
	fn pos_for<T>(&self, value: &T) -> usize {
		self.pos_mapping().pos_for(value)
	}
}

/// Macro to create `Serializer` implementation for serializers implementing
/// `PosTrackingSerializer`.
///
/// See `impl_serializer` for syntax rules.
#[macro_export]
macro_rules! impl_pos_tracking_serializer {
	($($type_def:tt)*) => {
		$crate::impl_serializer!(
			PosTrackingSerializer,
			{
				/// `PosTrackingSerializer` serializers do not record pointers,
				/// so have no need for a working `Addr`.
				type Addr = $crate::pos::NoopAddr;

				fn serialize_value<T: $crate::Serialize<Self>>(&mut self, value: &T) {
					use $crate::pos::PosMapping;

					// Align storage, ready to write value
					self.storage_mut().align_for::<T>();

					// Record position mapping for this value
					self.set_pos_mapping(PosMapping::new(value as *const T as usize, self.pos()));

					// Push value to storage.
					// `push_slice_unaligned`'s requirements are satisfied by `align_for::<T>()` and
					// `align_after::<T>()`.
					let slice = ::std::slice::from_ref(value);
					unsafe { self.storage_mut().push_slice_unaligned(slice) };
					self.storage_mut().align_after::<T>();

					// Serialize value (which may use the pos mapping we set)
					value.serialize_data(self);
				}

				// Skip recording position when no further processing for a slice
				#[inline]
				fn push_slice<T>(&mut self, slice: &[T], _ptr_addr: Self::Addr) {
					self.push_raw_slice(slice);
				}

				#[inline]
				fn push_and_process_slice<T, P: FnOnce(&mut Self)>(
					&mut self,
					slice: &[T],
					_ptr_addr: Self::Addr,
					process: P
				) {
					use $crate::pos::PosMapping;

					// Get position mapping before processing this
					let pos_mapping_before = *self.pos_mapping();

					// Align storage, ready to write slice
					self.storage_mut().align_for::<T>();

					// Record position mapping for this slice
					self.set_pos_mapping(PosMapping::new(slice.as_ptr() as usize, self.pos()));

					// Push slice to storage.
					// `push_slice_unaligned`'s requirements are satisfied by `align_for::<T>()` and
					// `align_after::<T>()`.
					unsafe { self.storage_mut().push_slice_unaligned(slice) };
					self.storage_mut().align_after::<T>();

					// Call `process` function (which may use the pos mapping we set)
					process(self);

					// Reset position mapping back to as it was
					self.set_pos_mapping(pos_mapping_before);
				}
			},
			$($type_def)*
		);
	};
}
