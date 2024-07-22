import { myFunction } from './script.js';
import './script2.js';
import mod from '@rust-jsc';
import rust from '@rust-jsc';
import { name } from '@rust-jsc';
import data from './test.json';

// import { name } from '@rust-jsc'

console.log(`Data: ${data}`);
console.log(`Virtual: ${mod.name} - ${rust.name}`);
console.log(`Name: ${name} , equal: ${mod.name === name}`);

// console.log(object);
console.log(`Filename: ${import.meta.filename}`);
console.log(`URL: ${import.meta.url}`);
console.log(`Directory: ${import.meta.dir}`);
console.log(`Main: ${import.meta.main}`);
console.log("Meta Inf :", import.meta.filename, import.meta.url, import.meta.dir);

const promise = Promise.resolve();

await promise.then(() => {
    console.log("Here :", myFunction());

    promise.then(() => {
        console.log("HereMK :", new Array().test());
        // throw new Error('Test Error');
    })
    // .catch((error) => {
    //     // console.log(error);
    // });
});

// new SharedArrayBuffer(10);

// try {
//     throw new Error(import.meta.filename);
// } catch (error) {
//     console.log(error);
// }
// console.log(myFunction());

async function nameM() {
  Promise.reject("Kevin");
}

await nameM();

// var arr = [];
(async () => {
    await new Promise((resolve, reject) => {
        promise.then(() => {
            console.setTimeout(() => {
                console.log("Testme");
                // reject('Error');
            }, 500)
        });
    });

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

console.log("Test 1: ");

throw new Error('Test Error Kevin');
// arr.push(1);
// arr.push(2);
// arr;

// export default { myFunction }
