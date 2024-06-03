import { myFunction } from './script.js';
import data from './test.json';
import rust from '@rust-jsc';

console.log(`Virtual: ${rust.name}`);
console.log(`Filename: ${data}`);

Array.prototype.test = function() {
    myFunction();
    console.log('test');
}