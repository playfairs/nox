const scores = [82, 91, 76, 88];
const total = scores.reduce((sum, score) => sum + score, 0);

console.log(`scores: ${scores.join(" ")}`);
console.log(`average: ${total / scores.length}`);
