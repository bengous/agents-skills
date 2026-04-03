# Crypto & Password Hashing

Bun's built-in password hashing and cryptographic hashing utilities.

## Password Hashing (`Bun.password`)

Argon2id by default, with bcrypt support. Automatic salt generation.

```typescript
// Hash (async — default Argon2id)
const hash = await Bun.password.hash("my-password");
// $argon2id$v=19$m=65536,t=2,p=1$...

// Verify (auto-detects algorithm from hash)
const ok = await Bun.password.verify("my-password", hash);

// Sync versions
const hashSync = Bun.password.hashSync("my-password");
const okSync = Bun.password.verifySync("my-password", hashSync);
```

### Bcrypt

```typescript
const hash = await Bun.password.hash("my-password", {
  algorithm: "bcrypt",
  cost: 12,  // work factor 4-31, default 10
});
```

### Argon2 Variants

```typescript
const hash = await Bun.password.hash("my-password", {
  algorithm: "argon2id",  // or "argon2i", "argon2d"
  memoryCost: 65536,      // KB (default 65536)
  timeCost: 3,            // iterations (default 2)
});
```

## Cryptographic Hashing (`Bun.CryptoHasher`)

Streaming hash computation for arbitrary data:

```typescript
const hasher = new Bun.CryptoHasher("sha256");
hasher.update("hello ");
hasher.update("world");
const digest = hasher.digest("hex");
// "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
```

### HMAC

```typescript
const hasher = new Bun.CryptoHasher("sha256", "secret-key");
hasher.update("hello world");
hasher.digest("hex");
// "095d5a21fe6d0646db223fdf3de6436bb8dfb2fab0b51677ecf6441fcf5f2a67"
```

### Digest formats

```typescript
hasher.digest("hex");        // hex string
hasher.digest("base64");     // base64 string
hasher.digest();             // Uint8Array
```

## Non-Cryptographic Hashing (`Bun.hash`)

Fast hashing for hash tables, checksums, etc. Not for security.

```typescript
Bun.hash("data");                 // bigint — Wyhash 64-bit (default)
Bun.hash("data", 1234n);         // with seed

// Named algorithms
Bun.hash.crc32("data");
Bun.hash.xxHash32("data");
Bun.hash.xxHash64("data");
Bun.hash.cityHash64("data");
Bun.hash.murmur32v3("data");

// Accepts: string | TypedArray | DataView | ArrayBuffer
```

## Official Docs

- [Hashing](https://bun.sh/docs/api/hashing)
