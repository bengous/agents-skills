# SQLite

Bun's built-in SQLite driver via `bun:sqlite`. Synchronous, zero-dependency.

## Open a Database

```typescript
import { Database } from "bun:sqlite";

const db = new Database("mydb.sqlite");
const memoryDb = new Database(":memory:");
const readonlyDb = new Database("mydb.sqlite", { readonly: true });
const strictDb = new Database(":memory:", { strict: true }); // bind without $ prefix
```

Enable WAL mode for concurrent reads:

```typescript
db.run("PRAGMA journal_mode = WAL;");
```

## Queries

### `db.run(sql)` — DDL/DML without results

```typescript
db.run(`
  CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    email TEXT UNIQUE
  )
`);
```

### `db.query(sql)` — Prepared statement (cached)

Returns a reusable `Statement` with these methods:

```typescript
const stmt = db.query("SELECT * FROM users WHERE id = ?");

stmt.get(1);       // single row object or undefined
stmt.all();        // array of row objects
stmt.values();     // array of arrays (no column names)
stmt.run(args);    // execute, return { changes, lastInsertRowid }

// Iterate large result sets without loading all into memory
for (const row of stmt.iterate()) {
  console.log(row.name);
}
```

### `db.prepare(sql)` — Same as `db.query()` (alias)

## Parameter Binding

Positional (`?`) or named (`$param`, `:param`, `@param`):

```typescript
// Positional
db.query("SELECT * FROM users WHERE name = ? AND age > ?").all("Alice", 18);

// Named (default — prefix required)
db.query("INSERT INTO users (name, email) VALUES ($name, $email)")
  .run({ $name: "Bob", $email: "bob@example.com" });

// Named (strict mode — no prefix needed)
const strictDb = new Database(":memory:", { strict: true });
strictDb.query("SELECT $message").all({ message: "Hello" });
```

## Insert Results

```typescript
const result = db.query("INSERT INTO users (name) VALUES (?)").run("Alice");
result.lastInsertRowid;  // number — ID of inserted row
result.changes;          // number — rows affected
```

## Transactions

Atomic: all statements succeed or all roll back.

```typescript
const insert = db.prepare("INSERT INTO users (name) VALUES ($name)");

const insertMany = db.transaction((users: { $name: string }[]) => {
  for (const user of users) insert.run(user);
  return users.length;
});

const count = insertMany([
  { $name: "Alice" },
  { $name: "Bob" },
]);

// Transaction modes
insertMany.deferred(users);   // BEGIN DEFERRED (default)
insertMany.immediate(users);  // BEGIN IMMEDIATE
insertMany.exclusive(users);  // BEGIN EXCLUSIVE
```

## Map to Class Instances

```typescript
class User {
  id!: number;
  name!: string;
  get displayName() { return `User: ${this.name}`; }
}

const users = db.query("SELECT * FROM users").as(User).all();
users[0].displayName; // "User: Alice"
```

## Cleanup

```typescript
db.close();
```

## Gotchas

1. **All operations are synchronous** — `db.query().all()` blocks. For async, use
   workers or wrap in `Effect.sync`.

2. **`strict: true` changes binding** — without it, named params need `$`/`:`/`@` prefix
   in the object keys.

3. **WAL is not default** — you must explicitly set `PRAGMA journal_mode = WAL`.

## Official Docs

- [SQLite](https://bun.sh/docs/api/sqlite)
