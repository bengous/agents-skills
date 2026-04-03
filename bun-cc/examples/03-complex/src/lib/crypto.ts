// Encryption utilities using Web Crypto API (AES-GCM)

// ============================================================================
// CONSTANTS
// ============================================================================

const PBKDF2_ITERATIONS = 100000;
const SALT_LENGTH = 16;
const IV_LENGTH = 12;

// ============================================================================
// PURE FUNCTIONS (exported for testing)
// ============================================================================

export function concatBuffers(...buffers: Uint8Array[]): Uint8Array {
  const result = new Uint8Array(buffers.reduce((sum, b) => sum + b.length, 0));
  let offset = 0;
  for (const buf of buffers) {
    result.set(buf, offset);
    offset += buf.length;
  }
  return result;
}

export function splitEncryptedData(data: Uint8Array) {
  return {
    salt: data.slice(0, SALT_LENGTH),
    iv: data.slice(SALT_LENGTH, SALT_LENGTH + IV_LENGTH),
    ciphertext: data.slice(SALT_LENGTH + IV_LENGTH),
  };
}

// ============================================================================
// SIDE-EFFECTING FUNCTIONS
// ============================================================================

async function deriveKey(password: string, salt: Uint8Array): Promise<CryptoKey> {
  const keyMaterial = await crypto.subtle.importKey("raw", new TextEncoder().encode(password), "PBKDF2", false, [
    "deriveKey",
  ]);
  return crypto.subtle.deriveKey(
    {
      name: "PBKDF2",
      salt: salt.buffer as ArrayBuffer,
      iterations: PBKDF2_ITERATIONS,
      hash: "SHA-256",
    },
    keyMaterial,
    { name: "AES-GCM", length: 256 },
    false,
    ["encrypt", "decrypt"],
  );
}

export async function encrypt(plaintext: string, password: string): Promise<Uint8Array> {
  const salt = crypto.getRandomValues(new Uint8Array(SALT_LENGTH));
  const iv = crypto.getRandomValues(new Uint8Array(IV_LENGTH));
  const key = await deriveKey(password, salt);
  const encrypted = await crypto.subtle.encrypt({ name: "AES-GCM", iv }, key, new TextEncoder().encode(plaintext));
  return concatBuffers(salt, iv, new Uint8Array(encrypted));
}

export async function decrypt(data: Uint8Array, password: string): Promise<string> {
  const { salt, iv, ciphertext } = splitEncryptedData(data);
  const key = await deriveKey(password, salt);
  const decrypted = await crypto.subtle.decrypt(
    { name: "AES-GCM", iv: iv.buffer as ArrayBuffer },
    key,
    ciphertext.buffer as ArrayBuffer,
  );
  return new TextDecoder().decode(decrypted);
}
