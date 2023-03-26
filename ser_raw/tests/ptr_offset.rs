use std::fmt::Debug;

mod common;
use common::{generate_minecraft_data, tests, Test};
use ser_raw::{
	storage::{AlignedVec, Storage},
	util::aligned_max_capacity,
	PtrOffsetSerializer, Serialize, Serializer,
};

// NB: Cannot easily test for error if try to serialize a type with alignment
// greater than the serializer's `MAX_VALUE_ALIGNMENT`, because it's an error at
// compile time, not runtime.

const MAX_CAPACITY: usize = aligned_max_capacity(16);
type Ser = PtrOffsetSerializer<16, 16, 8, MAX_CAPACITY, AlignedVec>;

fn serialize<T: Serialize<Ser>>(value: &T) -> (usize, AlignedVec) {
	let ser = Ser::new();
	ser.serialize(value)
}

fn test_serialize<T>(input: &T, test: Test, test_num: usize)
where T: Serialize<Ser> + Debug + PartialEq {
	let (pos, storage) = serialize(input);

	// No deserializer, so can't test output. Just testing length of output for now.
	// TODO: Test for correct relative pointers.
	let expected_size = match test {
		Test::Primitives => 96,
		Test::NonZeroNumbers => 80,
		Test::Arrays => 24,
		Test::ArraysOfBoxes => 96,
		Test::Tuples => 56,
		Test::EnumFieldless => 8,
		Test::EnumWithFields => 16,
		Test::BoxedPrimitives => 272,
		Test::BoxedStructs => 64,
		Test::VecOfPrimitives => [72, 96, 112][test_num],
		Test::VecOfVecs => 232,
		Test::VecsWithZeroLenZeroCapacity => 24,
		Test::VecsWithZeroLenExcessCapacity => 24,
		Test::VecsWithZeroLenExcessCapacity2 => 24,
		Test::VecsWithExcessCapacity => 32,
		Test::VecsWithExcessCapacity2 => 40,
		Test::Strings => [32, 32, 32, 40][test_num],
		Test::StringsWithZeroLenZeroCapacity => 24,
		Test::StringsWithZeroLenExcessCapacity => 24,
		Test::StringsWithZeroLenExcessCapacity2 => 24,
		Test::StringsWithExcessCapacity => 32,
		Test::StringsWithExcessCapacity2 => 32,
		Test::Options => [72, 72, 104, 96][test_num],
		Test::BigUint => 152,
		Test::BigInt => 336,
		Test::StructureWhereStorageGrowsAfterLastPointerWritten => 136,
		Test::MinecraftData => 1375104,
	};

	assert_eq!(pos, 0);
	assert_eq!(storage.pos(), expected_size);
}

tests!(test_serialize);
