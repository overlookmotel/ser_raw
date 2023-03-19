// Derive macros
#[cfg(feature = "derive")]
pub use ser_raw_derive::Serialize;
pub use ser_raw_derive_serializer::Serializer;

// Export Serializers, Storage, traits, and utils
mod serializer;
pub use serializer::Serializer;

mod serializers;
pub use serializers::{
	AlignedRelPtrSerializer, CompleteSerializer, PureCopySerializer, UnalignedSerializer,
};

mod serializer_traits;
pub mod ser_traits {
	pub use super::serializer_traits::*;
}

mod serialize;
pub use serialize::{Serialize, SerializeWith};

pub mod pos;
pub mod storage;
pub mod util;

// `Serialize` implementations for Rust internal types
mod serialize_impls;
