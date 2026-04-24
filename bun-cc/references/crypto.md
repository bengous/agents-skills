# Crypto and Hashing

Bun exposes password hashing, cryptographic hashing, HMAC, and fast
non-cryptographic hashes. Use Web Crypto or `node:crypto` when a project already
standardizes on those APIs.

## Password Hashing

`Bun.password` generates salts automatically and stores the algorithm in the hash.
`verify()` auto-detects PHC/MCF formats.

```typescript
const hash = await Bun.password.hash("my-password");
const ok = await Bun.password.verify("my-password", hash);
```

Configure algorithms deliberately:

```typescript
const argonHash = await Bun.password.hash("my-password", {
  algorithm: "argon2id",
  memoryCost: 65536,
  timeCost: 3,
});

const bcryptHash = await Bun.password.hash("my-password", {
  algorithm: "bcrypt",
  cost: 12,
});
```

Sync variants exist, but they are CPU-expensive and block the runtime:

```typescript
const hash = Bun.password.hashSync("my-password");
const ok = Bun.password.verifySync("my-password", hash);
```

Use sync hashing only in CLIs, migrations, tests, or startup paths where blocking
is acceptable.

## Cryptographic Hashing

```typescript
const hasher = new Bun.CryptoHasher("sha256");
hasher.update("hello ");
hasher.update("world");
const digest = hasher.digest("hex");
```

`update()` accepts strings, typed arrays, and array buffers. String updates can pass
an encoding as the second argument.

HMAC:

```typescript
const hmac = new Bun.CryptoHasher("sha256", "secret-key");
hmac.update("hello world");
console.log(hmac.digest("hex"));
```

Do not reuse an HMAC hasher after `digest()`. Use `.copy()` before digesting when
you need variants.

## Non-Cryptographic Hashing

`Bun.hash` is fast and not security-safe.

```typescript
Bun.hash("data");
Bun.hash("data", 1234n);
Bun.hash.crc32("data");
Bun.hash.xxHash64("data");
Bun.hash.rapidhash("data");
```

Use this for sharding, cache keys, or checksums where collision resistance is not a
security property.

## Guardrails

- Never use `Bun.hash` for passwords, tokens, signatures, or integrity checks against
  attackers.
- Prefer async password hashing in servers.
- Store password hashes exactly as returned; do not split out salts manually.
- Use `crypto.randomUUID()` or Web Crypto for random values, not hashes of predictable data.

## Official Docs

- `https://bun.sh/docs/api/hashing`
- `https://bun.sh/docs/runtime/hashing`
