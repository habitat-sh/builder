const faker = require('faker');
const util = {};

util.numberStringByLength = (length) => {
  let output = '';
  for (var i=0; i < length; i++) {
    output += faker.random.number(9);
  }
  return output;
}

util.randomArrayLengthOf = (callback, min = 1, max = 10) => {
  return [...Array(faker.random.number({min, max})).keys()].map(callback)
}


module.exports = util;