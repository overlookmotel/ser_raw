'use strict';

const PTR_SIZE = 8;
const MAX_CAPACITY = 1n << BigInt(PTR_SIZE);
const MIN_CAPACITY = 2n;

let size = MIN_CAPACITY;
const blockSizes = [size, size];
while (size < MAX_CAPACITY / 2n) {
	size *= 2n;
	blockSizes.push(size);
}

console.log('PTR_SIZE:', PTR_SIZE);
console.log('MAX_CAPACITY:', MAX_CAPACITY);
console.log('MIN_CAPACITY:', MIN_CAPACITY);
console.log('blockSizes:', blockSizes);
console.log('blockSizes.length:', blockSizes.length);
console.log('total size:', blockSizes.reduce((a, s) => a + s, 0n));
console.log('total size === MAX_CAPACITY:', blockSizes.reduce((a, s) => a + s, 0n) === MAX_CAPACITY);
