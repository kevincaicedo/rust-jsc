import { myFunction } from './script.js';
import './script2.js';
// import * as data from './test.json';

console.log("Meta Inf :", import.meta.filename, import.meta.path, import.meta.main);

const promise = Promise.resolve();

promise.then(() => {
    console.log("Here :", myFunction());
    
    for (let i = 0; i < 100000; i++) {
        console.log("Here :", i);
    }

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
