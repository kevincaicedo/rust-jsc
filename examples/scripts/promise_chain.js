// Promise chain test
function delay(val) {
    return Promise.resolve(val);
}

async function pipeline(input) {
    let result = await delay(input);
    result = await delay(result * 2);
    result = await delay(result + 10);
    result = await delay(result * 3);
    return result;
}

// Multiple parallel promises
async function parallelWork(count) {
    const promises = [];
    for (let i = 0; i < count; i++) {
        promises.push(pipeline(i));
    }
    const results = await Promise.all(promises);
    return results.reduce((a, b) => a + b, 0);
}

// Race condition test
async function raceTest() {
    const fast = delay(42);
    const slow = new Promise(resolve => {
        // Simulate slow by chaining
        delay(1).then(() => delay(2)).then(() => resolve(99));
    });
    return Promise.race([fast, slow]);
}

const total = await parallelWork(100);
const raceWinner = await raceTest();

export default { total, raceWinner };
