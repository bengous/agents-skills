# SQLite

Bun ships a synchronous SQLite driver as `bun:sqlite`. Use it for local CLIs,
embedded data stores, tests, and small services where blocking the event loop is
acceptable.

## Open a Database

```typescript
import { Database } from "bun:sqlite";

const db = new Database("app.sqlite");
const memoryDb = new Database(":memory:");
const readonlyDb = new Database("app.sqlite", { readonly: true });
const strictDb = new Database(":memory:", { strict: true });
```

Set journal mode deliberately instead of assuming defaults:

```typescript
db.run("PRAGMA journal_mode = WAL;");
```

## Statements

```typescript
db.run(`
  CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    email TEXT UNIQUE
  )
`);

const byId = db.query("SELECT * FROM users WHERE id = ?");
const user = byId.get(1);
const allUsers = db.query("SELECT * FROM users").all();
const names = db.query("SELECT name FROM users").values();
```

Use `iterate()` for large result sets:

```typescript
for (const row of db.query("SELECT * FROM users").iterate()) {
  console.log(row);
}
```

## Parameters

Use parameters for all user-provided values.

```typescript
db.query("SELECT * FROM users WHERE name = ? AND age > ?").all("Alice", 18);

db.query("INSERT INTO users (name, email) VALUES ($name, $email)").run({
  $name: "Bob",
  $email: "bob@example.com",
});
```

With `{ strict: true }`, named parameter object keys omit the prefix:

```typescript
const db = new Database(":memory:", { strict: true });
db.query("SELECT $message").all({ message: "hello" });
```

## Writes and Transactions

```typescript
const result = db.query("INSERT INTO users (name) VALUES (?)").run("Alice");
console.log(result.lastInsertRowid, result.changes);
```

```typescript
const insert = db.prepare("INSERT INTO users (name) VALUES ($name)");

const insertMany = db.transaction((users: { $name: string }[]) => {
  for (const user of users) insert.run(user);
  return users.length;
});

insertMany([{ $name: "Alice" }, { $name: "Bob" }]);
insertMany.immediate([{ $name: "Carol" }]);
insertMany.exclusive([{ $name: "Dana" }]);
```

## Mapping and Cleanup

```typescript
class User {
  id!: number;
  name!: string;
  get displayName() {
    return `User: ${this.name}`;
  }
}

const users = db.query("SELECT * FROM users").as(User).all();
db.close();
```

## Guardrails

- All database operations are synchronous. Keep heavy queries out of latency-sensitive
  request paths or isolate them in workers/processes.
- Do not concatenate SQL. Bind parameters.
- Choose `strict: true` deliberately; it changes named parameter object keys.
- Close databases owned by the current process, especially in tests.
- Test transaction behavior with a real database or `:memory:`, not mocked statements.

## Official Docs

- `https://bun.sh/docs/runtime/sqlite`
