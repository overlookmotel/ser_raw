use crate::Serializer;

/// Trait for the most basic serializers which purely copy types.
///
/// Implement the trait on a serializer, and then use macro
/// `impl_pure_copy_serializer!()` to implement `Serialize`.
///
/// # Example
///
/// ```
/// use ser_raw::{impl_pure_copy_serializer, PureCopySerializer, SerializerStorage};
///
/// struct MySerializer {}
///
/// impl PureCopySerializer for MySerializer {}
/// impl_pure_copy_serializer!(MySerializer);
///
/// impl SerializerStorage for MySerializer {
/// 	// ...
/// }
/// ```
pub trait PureCopySerializer: Serializer {}

/// Macro to create `Serializer` implementation for serializers implementing
/// `PureCopySerializer`.
///
/// See `impl_serializer` for syntax rules.
#[macro_export]
macro_rules! impl_pure_copy_serializer {
	($($type_def:tt)*) => {
		$crate::impl_serializer!(PureCopySerializer, {}, $($type_def)*);
	};
}
