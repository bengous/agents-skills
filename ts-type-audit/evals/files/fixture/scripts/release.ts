import { parseCommand, run } from "../src/commands.js";

console.log(run(parseCommand(process.argv.slice(2))));
