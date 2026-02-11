import {
  initialized,
  echoBool, negateBool,
  echoI8, echoU8,
  echoI16, echoU16,
  echoI32, addI32, echoU32,
  echoI64, echoU64,
  echoF32, addF32,
  echoF64, addF64,
  echoString, concatStrings, stringLength,
  echoPoint, makePoint, addPoints, pointDistance,
} from './dist/wasm/pkg/node.js';

await initialized;
console.log('Module initialized via node.js loader\n');

function assert(condition, msg) {
  if (!condition) throw new Error(msg);
}

console.log('Testing bool...');
assert(echoBool(true) === true, 'echoBool(true)');
assert(echoBool(false) === false, 'echoBool(false)');
assert(negateBool(true) === false, 'negateBool(true)');
assert(negateBool(false) === true, 'negateBool(false)');

console.log('Testing i8/u8...');
assert(echoI8(127) === 127, 'echoI8(127)');
assert(echoI8(-128) === -128, 'echoI8(-128)');
assert(echoU8(255) === 255, 'echoU8(255)');
assert(echoU8(0) === 0, 'echoU8(0)');

console.log('Testing i16/u16...');
assert(echoI16(32767) === 32767, 'echoI16(32767)');
assert(echoI16(-32768) === -32768, 'echoI16(-32768)');
assert(echoU16(65535) === 65535, 'echoU16(65535)');

console.log('Testing i32/u32...');
assert(echoI32(2147483647) === 2147483647, 'echoI32(max)');
assert(echoI32(-2147483648) === -2147483648, 'echoI32(min)');
assert(addI32(2, 3) === 5, 'addI32(2, 3)');
assert(echoU32(2147483647) === 2147483647, 'echoU32(below signed max)');

console.log('Testing i64/u64...');
assert(echoI64(9007199254740991n) === 9007199254740991n, 'echoI64(safe max)');
assert(echoI64(-9007199254740991n) === -9007199254740991n, 'echoI64(safe min)');
assert(echoU64(9007199254740991n) === 9007199254740991n, 'echoU64(safe max)');

console.log('Testing f32...');
assert(Math.abs(echoF32(3.14) - 3.14) < 0.001, 'echoF32(3.14)');
assert(Math.abs(addF32(1.5, 2.5) - 4.0) < 0.001, 'addF32(1.5, 2.5)');

console.log('Testing f64...');
assert(echoF64(3.141592653589793) === 3.141592653589793, 'echoF64(pi)');
assert(addF64(1.1, 2.2) === 3.3000000000000003, 'addF64(1.1, 2.2)');

console.log('Testing string...');
assert(echoString('hello') === 'hello', 'echoString(hello)');
assert(concatStrings('foo', 'bar') === 'foobar', 'concatStrings');
assert(stringLength('test') === 4, 'stringLength(test)');

console.log('Testing records...');
const p1 = makePoint(3.0, 4.0);
assert(p1.x === 3.0, 'makePoint x');
assert(p1.y === 4.0, 'makePoint y');

const p2 = echoPoint({ x: 1.5, y: 2.5 });
assert(p2.x === 1.5, 'echoPoint x');
assert(p2.y === 2.5, 'echoPoint y');

const p3 = addPoints({ x: 1.0, y: 2.0 }, { x: 3.0, y: 4.0 });
assert(p3.x === 4.0, 'addPoints x');
assert(p3.y === 6.0, 'addPoints y');

const dist = pointDistance({ x: 3.0, y: 4.0 });
assert(Math.abs(dist - 5.0) < 0.0001, 'pointDistance');

console.log('\nAll tests passed!');
