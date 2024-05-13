// import { myFunction } from './script.js';
// import './script2.js';
// import * as data from './test.json';

console.log("Meta Inf :", import.meta.filename, import.meta.path, import.meta.main);

const promise = Promise.resolve();

promise.then(() => {
    // console.log("Here :", myFunction());

    promise.then(() => {
        console.log("Here :", new Array().test());
    })
    .catch((error) => {
        console.log(error);
    });
});

try {
    throw new Error(import.meta.filename);
} catch (error) {
    console.log(error);
}
// console.log(myFunction());

// var arr = [];
// (async () => {
//     try {
//         await import('./script.mjs').then(module => { module.myFunction(); })
//     } catch (error) {
//         errorMessage = String(error);
//         console.log(errorMessage);
//     }
//     await 1;
//     arr.push(3);
// })();
// arr.push(1);
// arr.push(2);
// arr;