import { myFunction } from './script.js';
import './script2.js';
import mod from '@rust-jsc';
import rust from '@rust-jsc';
import data from './test.json';

// import { name } from '@rust-jsc'

console.log(`Data: ${data}`);
console.log(`Virtual: ${mod.name} - ${rust.name}`);

// console.log(object);
console.log(`Filename: ${import.meta.filename}`);
console.log(`URL: ${import.meta.url}`);
console.log(`Directory: ${import.meta.dir}`);
console.log(`Main: ${import.meta.main}`);
console.log("Meta Inf :", import.meta.filename, import.meta.url, import.meta.dir);

const promise = Promise.resolve();

promise.then(() => {
    console.log("Here :", myFunction());

    promise.then(() => {
        console.log("Here :", new Array().test());
    })
    .catch((error) => {
        console.log(error);
    });
});

// try {
//     throw new Error(import.meta.filename);
// } catch (error) {
//     console.log(error);
// }
// console.log(myFunction());

// var arr = [];
(async () => {
    try {
        // const module = await import('./test.json');
        // console.log(`Before Import: ${module.myFunction()}`);
        // console.log("After Import: " + module.default.name);
    } catch (error) {
        let errorMessage = String(error);
        console.log(errorMessage);
    }
    // await 1;
    // arr.push(3);
})();
// arr.push(1);
// arr.push(2);
// arr;

// export default { myFunction }
