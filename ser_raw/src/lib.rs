// Derive macros
#[cfg(feature = "derive")]
pub use ser_raw_derive::Serialize;
pub use ser_raw_derive_serializer::Serializer;

// Export Serializers, Storage, traits, and utils
mod serializer;
pub use serializer::Serializer;

mod serializer_traits;
pub use serializer_traits::{
	CompleteSerializerTrait, PosTrackingSerializer, PtrGroup, PtrSerializer, Ptrs, RelPtrSerializer,
	WritableSerializer,
};

mod serializers;
pub use serializers::{
	AlignedRelPtrSerializer, AlignedSerializer, CompleteSerializer, UnalignedSerializer,
};

mod serialize;
pub use serialize::{Serialize, SerializeWith};

pub mod pos;
pub mod storage;
pub mod util;

// `Serialize` implementations for Rust internal types
mod serialize_impls;
