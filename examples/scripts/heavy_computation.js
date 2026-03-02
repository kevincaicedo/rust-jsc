// Heavy computation: matrix multiplication, sorting, string operations
function matrixMultiply(a, b) {
    const rows = a.length;
    const cols = b[0].length;
    const n = b.length;
    const result = Array.from({ length: rows }, () => new Array(cols).fill(0));

    for (let i = 0; i < rows; i++) {
        for (let j = 0; j < cols; j++) {
            for (let k = 0; k < n; k++) {
                result[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    return result;
}

function createMatrix(size) {
    return Array.from({ length: size }, (_, i) =>
        Array.from({ length: size }, (_, j) => (i * size + j) % 100)
    );
}

function quickSort(arr) {
    if (arr.length <= 1) return arr;
    const pivot = arr[Math.floor(arr.length / 2)];
    const left = arr.filter(x => x < pivot);
    const mid = arr.filter(x => x === pivot);
    const right = arr.filter(x => x > pivot);
    return [...quickSort(left), ...mid, ...quickSort(right)];
}

function stringOps(count) {
    let result = '';
    for (let i = 0; i < count; i++) {
        result += String.fromCharCode(65 + (i % 26));
        if (i % 100 === 0) {
            result = result.split('').reverse().join('');
        }
    }
    return result.length;
}

// Run all
const matA = createMatrix(50);
const matB = createMatrix(50);
const matResult = matrixMultiply(matA, matB);

const unsorted = Array.from({ length: 10000 }, () => Math.floor(Math.random() * 100000));
const sorted = quickSort(unsorted);

const strLen = stringOps(5000);

export default {
    matrixSize: matResult.length,
    sortedLength: sorted.length,
    stringLength: strLen
};
