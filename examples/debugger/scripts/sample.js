// Sample JavaScript file for debugging demonstration
// This file showcases various debugging scenarios

const VERSION = "1.0.0";
const DEBUG = true;

// Simple function for basic breakpoint testing
function add(a, b) {
    let result = a + b;
    return result;
}

// Function with multiple local variables
function calculateStats(numbers) {
    let sum = 0;
    let min = Infinity;
    let max = -Infinity;
    let count = numbers.length;

    for (let i = 0; i < count; i++) {
        let num = numbers[i];
        sum += num;
        if (num < min) min = num;
        if (num > max) max = num;
    }

    let average = sum / count;

    return {
        sum: sum,
        min: min,
        max: max,
        average: average,
        count: count
    };
}

// Recursive function for step-into testing
function factorial(n) {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

// Function with closure for scope testing
function createCounter(initialValue) {
    let count = initialValue || 0;

    return {
        increment: function() {
            count++;
            return count;
        },
        decrement: function() {
            count--;
            return count;
        },
        getValue: function() {
            return count;
        }
    };
}

// Async-like function (using callbacks)
function delayedAdd(a, b, callback) {
    let result = a + b;
    // Simulate async behavior
    callback(result);
}

// Function that throws an exception
function divide(a, b) {
    if (b === 0) {
        throw new Error("Division by zero is not allowed");
    }
    return a / b;
}

// Function with debugger statement
function debugMe(value) {
    let doubled = value * 2;
    debugger; // Execution will pause here
    let tripled = value * 3;
    return { doubled: doubled, tripled: tripled };
}

// Complex nested object for variable inspection
function createPerson(name, age) {
    return {
        name: name,
        age: age,
        address: {
            street: "123 Main St",
            city: "Anytown",
            country: "USA"
        },
        hobbies: ["reading", "coding", "gaming"],
        isAdult: function() {
            return this.age >= 18;
        },
        greet: function() {
            return "Hello, my name is " + this.name;
        }
    };
}

// Class-like structure
function Animal(name, species) {
    this.name = name;
    this.species = species;
    this.energy = 100;
}

Animal.prototype.eat = function() {
    this.energy += 10;
    return this.energy;
};

Animal.prototype.sleep = function() {
    this.energy += 30;
    return this.energy;
};

Animal.prototype.play = function() {
    if (this.energy >= 20) {
        this.energy -= 20;
        return true;
    }
    return false;
};

// Array manipulation for watching values change
function processArray(arr) {
    let step1 = arr.map(x => x * 2);
    let step2 = step1.filter(x => x > 5);
    let step3 = step2.reduce((acc, x) => acc + x, 0);
    return {
        original: arr,
        doubled: step1,
        filtered: step2,
        sum: step3
    };
}

// Conditional logic for stepping through
function gradeCalculator(score) {
    let grade;
    let message;

    if (score >= 90) {
        grade = 'A';
        message = 'Excellent!';
    } else if (score >= 80) {
        grade = 'B';
        message = 'Good job!';
    } else if (score >= 70) {
        grade = 'C';
        message = 'Not bad';
    } else if (score >= 60) {
        grade = 'D';
        message = 'Needs improvement';
    } else {
        grade = 'F';
        message = 'Please try again';
    }

    return { score: score, grade: grade, message: message };
}

// Export all functions for testing
export {
    VERSION,
    DEBUG,
    add,
    calculateStats,
    factorial,
    createCounter,
    delayedAdd,
    divide,
    debugMe,
    createPerson,
    Animal,
    processArray,
    gradeCalculator
};
