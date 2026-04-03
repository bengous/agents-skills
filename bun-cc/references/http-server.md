# HTTP Server & WebSockets

Bun's built-in HTTP server via `Bun.serve()`. No external dependencies needed.

## Basic HTTP Server

```typescript
const server = Bun.serve({
  port: 3000,             // default: 3000 or $PORT
  hostname: "0.0.0.0",    // default: "0.0.0.0"

  async fetch(req) {
    const url = new URL(req.url);

    if (url.pathname === "/api/health") {
      return Response.json({ ok: true });
    }

    if (req.method === "POST" && url.pathname === "/api/data") {
      const body = await req.json();
      return Response.json({ received: body }, { status: 201 });
    }

    return new Response("Not Found", { status: 404 });
  },

  error(error) {
    return new Response(`Error: ${error.message}`, { status: 500 });
  },
});

console.log(`Listening on ${server.url}`);
```

## Server Interface

Key properties and methods on the returned `server` object:

```typescript
server.port;             // number — port listening on
server.hostname;         // string
server.url;              // URL — full URL including protocol
server.development;      // boolean

server.stop();           // stop accepting connections (returns Promise)
server.reload(options);  // update fetch/error handlers without restart
server.ref();            // keep process alive while server runs
server.unref();          // allow process to exit

server.pendingRequests;  // number of in-flight HTTP requests
server.pendingWebSockets; // number of active WebSocket connections
server.requestIP(req);   // SocketAddress | null
```

## TLS / HTTPS

```typescript
Bun.serve({
  port: 443,
  tls: {
    cert: Bun.file("cert.pem"),
    key: Bun.file("key.pem"),
    // ca: Bun.file("ca.pem"),  // optional CA chain
    // passphrase: "...",        // optional key passphrase
  },
  fetch(req) { /* ... */ },
});
```

## WebSocket Upgrade

Upgrade HTTP requests to WebSocket inside the `fetch` handler:

```typescript
type WsData = { username: string; joinedAt: number };

const server = Bun.serve({
  fetch(req, server) {
    const url = new URL(req.url);
    if (url.pathname === "/ws") {
      const username = url.searchParams.get("user") || "anon";
      const ok = server.upgrade(req, {
        data: { username, joinedAt: Date.now() } satisfies WsData,
        headers: { "Set-Cookie": "session=abc" }, // optional
      });
      return ok ? undefined : new Response("Upgrade failed", { status: 500 });
    }
    return new Response("Hello");
  },

  websocket: {
    open(ws) {
      console.log(`${ws.data.username} connected`);
    },
    message(ws, message) {
      ws.send(`Echo: ${message}`);
    },
    close(ws, code, reason) {
      console.log(`${ws.data.username} left (${code})`);
    },
    drain(ws) {
      // called when backpressure is relieved
    },

    // Configuration
    idleTimeout: 120,              // seconds (default 120)
    maxPayloadLength: 1024 * 1024, // bytes (default 16 MB)
    perMessageDeflate: true,       // compression
    sendPings: true,               // keep-alive pings
    backpressureLimit: 1024 * 1024,
    closeOnBackpressureLimit: false,
    publishToSelf: false,
  },
});
```

## WebSocket Pub/Sub

Built-in topic-based pub/sub — no Redis or external broker needed:

```typescript
websocket: {
  open(ws) {
    ws.subscribe("chat");                    // join topic
    server.publish("chat", "Someone joined"); // broadcast
  },
  message(ws, msg) {
    server.publish("chat", `${ws.data.username}: ${msg}`);
  },
  close(ws) {
    ws.unsubscribe("chat");
    server.publish("chat", `${ws.data.username} left`);
  },
}

// Publish from outside handlers
server.publish("chat", "Server announcement!");
server.subscriberCount("chat"); // number of subscribers
```

## Hot Reload

Update handlers without dropping connections:

```typescript
server.reload({
  fetch(req) { /* new handler */ },
  error(err) { /* new error handler */ },
});
```

## Gotchas

1. **`server.upgrade()` returns boolean** — return `undefined` (not `new Response()`)
   on successful upgrade. Returning a Response after upgrade causes an error.

2. **WebSocket `data` typing** — use `satisfies` or generic `Bun.serve<WsData>()`
   to type `ws.data` across all handlers.

3. **`publish` return value** — returns bytes sent, `0` if dropped, `-1` on backpressure.

## Official Docs

- [HTTP Server](https://bun.sh/docs/api/http)
- [WebSockets](https://bun.sh/docs/api/websockets)
