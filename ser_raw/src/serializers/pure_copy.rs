use crate::Serializer;

/// Trait for the most basic serializers which purely copy types.
pub trait PureCopySerializer: Serializer {}
impl<Ser: PureCopySerializer> Serializer for Ser {}
