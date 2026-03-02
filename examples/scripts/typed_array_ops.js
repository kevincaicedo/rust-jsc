// TypedArray operations
function fillTypedArray(size) {
    const arr = new Uint8Array(size);
    for (let i = 0; i < size; i++) {
        arr[i] = i % 256;
    }
    return arr;
}

function transformFloat32(input) {
    const float = new Float32Array(input.length);
    for (let i = 0; i < input.length; i++) {
        float[i] = input[i] * 1.5 + 0.5;
    }
    return float;
}

function sumFloat64(count) {
    const arr = new Float64Array(count);
    for (let i = 0; i < count; i++) {
        arr[i] = Math.sin(i) * Math.cos(i);
    }
    let sum = 0;
    for (let i = 0; i < arr.length; i++) {
        sum += arr[i];
    }
    return sum;
}

function bufferCopy(size) {
    const src = new ArrayBuffer(size);
    const srcView = new Uint8Array(src);
    for (let i = 0; i < size; i++) {
        srcView[i] = i % 256;
    }

    const dst = new ArrayBuffer(size);
    const dstView = new Uint8Array(dst);
    dstView.set(srcView);

    return dstView.reduce((a, b) => a + b, 0);
}

const u8Result = fillTypedArray(10000);
const f32Result = transformFloat32(u8Result);
const f64Sum = sumFloat64(5000);
const copySum = bufferCopy(4096);

export default {
    u8Length: u8Result.length,
    f32Length: f32Result.length,
    f64Sum: f64Sum,
    copySum: copySum
};
