// Derive macro for `Serialize`
#[cfg(feature = "derive")]
pub use ser_raw_derive::Serialize;

// Export Serializers, Storage, traits, and utils
mod serializer;
pub use serializer::Serializer;

mod serializers;
pub use serializers::{
	CompleteSerializerTrait, PosTrackingSerializer, PtrGroup, PtrSerializer, PtrsRecord,
	PureCopySerializer, WritableSerializer,
};

mod serializer_impls;
pub use serializer_impls::{
	AlignedRelPtrSerializer, AlignedSerializer, CompleteSerializer, UnalignedSerializer,
};

mod serialize;
pub use serialize::{Serialize, SerializeWith};

pub mod pos;
pub mod storage;
pub mod util;

// `Serialize` implementations for Rust internal types
mod serialize_impls;
