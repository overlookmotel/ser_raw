use std::{fmt::Debug, mem};

mod common;
use common::{generate_minecraft_data, tests, Test};
use ser_raw::{
	storage::{AlignedVec, ContiguousStorage},
	util::aligned_max_capacity,
	CompleteSerializer, Serialize, Serializer,
};

// NB: Cannot easily test for error if try to serialize a type with alignment
// greater than the serializer's `MAX_VALUE_ALIGNMENT`, because it's an error at
// compile time, not runtime.

const MAX_CAPACITY: usize = aligned_max_capacity(16);
const PTR_SIZE: usize = mem::size_of::<usize>();

type AlVec = AlignedVec<16, 16, 8, MAX_CAPACITY>;
type Ser = CompleteSerializer<16, 16, 8, MAX_CAPACITY, AlVec>;

fn serialize<T: Serialize<Ser>>(value: &T) -> AlVec {
	let ser = Ser::new();
	ser.serialize(value)
}

fn deserialize<T>(storage: &AlVec) -> &T {
	unsafe { &*storage.as_ptr().cast() }
}

fn test_serialize<T>(input: &T, _test: Test, _test_num: usize)
where T: Serialize<Ser> + Debug + PartialEq {
	let storage = serialize(input);
	let output: &T = deserialize(&storage);
	assert_eq!(input, output);
}

tests!(test_serialize);

#[test]
fn vecs_with_zero_len_represented_correctly() {
	// Zero length vecs are reduced to 0 capacity and dangling pointer

	// 0 capacity originally
	let input = Vec::<u8>::new();
	assert_eq!(input.capacity(), 0);
	let storage = serialize(&input);

	assert_eq!(storage.as_slice().len(), PTR_SIZE * 3);
	let parts: &[usize; 3] = unsafe { &*storage.as_ptr().cast() };
	assert_eq!(parts, &[0, 1, 0]);

	let output: &Vec<u8> = deserialize(&storage);
	assert_eq!(&input, output);
	assert_eq!(output.len(), 0);
	assert_eq!(output.capacity(), 0);

	// Excess capacity
	let input = Vec::<u8>::with_capacity(5);
	assert!(input.capacity() >= 5);
	let storage = serialize(&input);

	assert_eq!(storage.as_slice().len(), PTR_SIZE * 3);
	let parts: &[usize; 3] = unsafe { &*storage.as_ptr().cast() };
	assert_eq!(parts, &[0, 1, 0]);

	let output: &Vec<u8> = deserialize(&storage);
	assert_eq!(&input, output);
	assert_eq!(output.len(), 0);
	assert_eq!(output.capacity(), 0);

	// Excess capacity
	let mut input = Vec::<u32>::new();
	input.push(1);
	input.pop();
	let storage = serialize(&input);

	assert_eq!(storage.as_slice().len(), PTR_SIZE * 3);
	let parts: &[usize; 3] = unsafe { &*storage.as_ptr().cast() };
	assert_eq!(parts, &[0, 4, 0]);

	let output: &Vec<u32> = deserialize(&storage);
	assert_eq!(&input, output);
	assert_eq!(output.len(), 0);
	assert_eq!(output.capacity(), 0);
}

#[test]
fn vecs_with_excess_capacity_represented_correctly() {
	// Vecs with spare capacity are shrunk to fit
	let mut input = Vec::<u8>::with_capacity(5);
	input.push(1);
	let storage = serialize(&input);

	let output: &Vec<u8> = deserialize(&storage);
	assert_eq!(&input, output);
	assert_eq!(output.len(), 1);
	assert_eq!(output.capacity(), 1);

	let mut input = vec![0x01020304, 0x05060708, 0x090a0b0c, 0x0d0e0f10];
	input.pop();
	assert!(input.capacity() > 3);
	let storage = serialize(&input);

	let output: &Vec<u32> = deserialize(&storage);
	assert_eq!(&input, output);
	assert_eq!(output.len(), 3);
	assert_eq!(output.capacity(), 3);
}

#[test]
fn strings_with_zero_len_represented_correctly() {
	// Zero length strings are reduced to 0 capacity and dangling pointer

	// 0 capacity originally
	let input = "".to_string();
	assert_eq!(input.capacity(), 0);
	let storage = serialize(&input);

	assert_eq!(storage.as_slice().len(), PTR_SIZE * 3);
	let parts: &[usize; 3] = unsafe { &*storage.as_ptr().cast() };
	assert_eq!(parts, &[0, 1, 0]);

	let output: &String = deserialize(&storage);
	assert_eq!(&input, output);
	assert_eq!(output.len(), 0);
	assert_eq!(output.capacity(), 0);

	// Excess capacity
	let input = String::with_capacity(5);
	assert!(input.capacity() >= 5);
	let storage = serialize(&input);

	assert_eq!(storage.as_slice().len(), PTR_SIZE * 3);
	let parts: &[usize; 3] = unsafe { &*storage.as_ptr().cast() };
	assert_eq!(parts, &[0, 1, 0]);

	let output: &String = deserialize(&storage);
	assert_eq!(&input, output);
	assert_eq!(output.len(), 0);
	assert_eq!(output.capacity(), 0);

	// Excess capacity
	let mut input = "x".to_string();
	input.pop();
	assert!(input.capacity() >= 1);
	let storage = serialize(&input);

	assert_eq!(storage.as_slice().len(), PTR_SIZE * 3);
	let parts: &[usize; 3] = unsafe { &*storage.as_ptr().cast() };
	assert_eq!(parts, &[0, 1, 0]);

	let output: &String = deserialize(&storage);
	assert_eq!(&input, output);
	assert_eq!(output.len(), 0);
	assert_eq!(output.capacity(), 0);
}

#[test]
fn strings_with_excess_capacity_represented_correctly() {
	// Strings with spare capacity are shrunk to fit
	let mut input = String::with_capacity(5);
	input.push('x');
	assert!(input.capacity() > 1);
	let storage = serialize(&input);

	let output: &String = deserialize(&storage);
	assert_eq!(&input, output);
	assert_eq!(output.len(), 1);
	assert_eq!(output.capacity(), 1);

	let mut input = "abcd".to_string();
	input.pop();
	assert!(input.capacity() > 3);
	let storage = serialize(&input);

	let output: &String = deserialize(&storage);
	assert_eq!(&input, output);
	assert_eq!(output.len(), 3);
	assert_eq!(output.capacity(), 3);
}
