// Recursive fibonacci
function fibonacci(n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

// Iterative fibonacci for comparison
function fibonacciIterative(n) {
    let a = 0, b = 1;
    for (let i = 0; i < n; i++) {
        [a, b] = [b, a + b];
    }
    return a;
}

// Run both and export results
const recursive_result = fibonacci(30);
const iterative_result = fibonacciIterative(30);

export default { recursive_result, iterative_result };
