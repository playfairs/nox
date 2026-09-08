const argumentsForProgram = process.argv.slice(2);

console.log(`received ${argumentsForProgram.length} arguments`);
argumentsForProgram.forEach((argument, index) => {
  console.log(`[${index + 1}] ${argument}`);
});
