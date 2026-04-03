import { describe, expect, test } from "bun:test";
import {
  concatBuffers,
  decrypt,
  encrypt,
  splitEncryptedData,
} from "../src/lib/crypto";

describe("concatBuffers", () => {
  test("concatenates multiple buffers", () => {
    const a = new Uint8Array([1, 2]);
    const b = new Uint8Array([3, 4, 5]);
    const c = new Uint8Array([6]);
    const result = concatBuffers(a, b, c);
    expect(result).toEqual(new Uint8Array([1, 2, 3, 4, 5, 6]));
  });

  test("handles empty buffers", () => {
    const a = new Uint8Array([1, 2]);
    const b = new Uint8Array([]);
    const result = concatBuffers(a, b);
    expect(result).toEqual(new Uint8Array([1, 2]));
  });
});

describe("splitEncryptedData", () => {
  test("splits data into salt, iv, and ciphertext", () => {
    const data = new Uint8Array(50);
    for (let i = 0; i < 50; i++) data[i] = i;

    const { salt, iv, ciphertext } = splitEncryptedData(data);
    expect(salt.length).toBe(16);
    expect(iv.length).toBe(12);
    expect(ciphertext.length).toBe(22);
    expect(salt[0]).toBe(0);
    expect(iv[0]).toBe(16);
    expect(ciphertext[0]).toBe(28);
  });
});

describe("encrypt/decrypt roundtrip", () => {
  test("encrypts and decrypts text correctly", async () => {
    const plaintext = "SECRET_KEY=abc123\nAPI_TOKEN=xyz789";
    const password = "testpassword";

    const encrypted = await encrypt(plaintext, password);
    expect(encrypted.length).toBeGreaterThan(plaintext.length);

    const decrypted = await decrypt(encrypted, password);
    expect(decrypted).toBe(plaintext);
  });

  test("fails with wrong password", async () => {
    const plaintext = "SECRET=value";
    const encrypted = await encrypt(plaintext, "correct");

    await expect(decrypt(encrypted, "wrong")).rejects.toThrow();
  });

  test("handles empty string", async () => {
    const encrypted = await encrypt("", "pass");
    const decrypted = await decrypt(encrypted, "pass");
    expect(decrypted).toBe("");
  });

  test("handles unicode content", async () => {
    const plaintext =
      "EMOJI=\u{1F680}\nJAPANESE=\u3053\u3093\u306B\u3061\u306F";
    const encrypted = await encrypt(plaintext, "pass");
    const decrypted = await decrypt(encrypted, "pass");
    expect(decrypted).toBe(plaintext);
  });
});
