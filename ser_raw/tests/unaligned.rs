use std::fmt::Debug;

mod common;
use common::{generate_minecraft_data, tests, Test};
use ser_raw::{
	storage::{Storage, UnalignedVec},
	Serialize, Serializer, UnalignedSerializer,
};

// NB: Cannot easily test for error if try to serialize a type with alignment
// greater than the serializer's `MAX_VALUE_ALIGNMENT`, because it's an error at
// compile time, not runtime.

type Ser = UnalignedSerializer<UnalignedVec>;

fn serialize<T: Serialize<Ser>>(value: &T) -> UnalignedVec {
	let mut ser = Ser::new();
	ser.serialize_value(value);
	ser.into_storage()
}

fn test_serialize<T>(input: &T, test: Test, test_num: usize)
where T: Serialize<Ser> + Debug + PartialEq {
	let storage = serialize(input);

	// No deserializer, so can't test output. Just testing length of output for now.
	let expected_size = match test {
		Test::Primitives => 96,
		Test::NonZeroNumbers => 80,
		Test::Arrays => 20,
		Test::ArraysOfBoxes => 65,
		Test::Tuples => 39,
		Test::EnumFieldless => 1,
		Test::EnumWithFields => 12,
		Test::BoxedPrimitives => 223,
		Test::BoxedStructs => 56,
		Test::VecOfPrimitives => [72, 79, 97][test_num],
		Test::VecOfVecs => 208,
		Test::VecsWithZeroLenZeroCapacity => 24,
		Test::VecsWithZeroLenExcessCapacity => 24,
		Test::VecsWithZeroLenExcessCapacity2 => 24,
		Test::VecsWithExcessCapacity => 25,
		Test::VecsWithExcessCapacity2 => 36,
		Test::Strings => [27, 25, 32, 38][test_num],
		Test::StringsWithZeroLenZeroCapacity => 24,
		Test::StringsWithZeroLenExcessCapacity => 24,
		Test::StringsWithZeroLenExcessCapacity2 => 24,
		Test::StringsWithExcessCapacity => 25,
		Test::StringsWithExcessCapacity2 => 27,
		Test::Options => [72, 72, 104, 91][test_num],
		Test::StructureWhereStorageGrowsAfterLastPointerWritten => 120,
		Test::MinecraftData => 1290592,
	};

	assert_eq!(storage.len(), expected_size);
}

tests!(test_serialize);
