const printJson = (data: unknown): void => {
  process.stdout.write(`${JSON.stringify(data, null, 2)}\n`);
};

const printError = (msg: string): void => {
  process.stderr.write(`${msg}\n`);
};

const die = (msg: string, code = 1): never => {
  printError(msg);
  process.exit(code);
};

export { die, printError, printJson };
