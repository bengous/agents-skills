# HTTP Server and WebSockets

Use `Bun.serve()` for Bun-native HTTP servers. For new Bun versions, prefer the
`routes` object for simple routing and `fetch` as fallback or when request handling
is highly dynamic.

## Routes

```typescript
const server = Bun.serve({
  port: 3000,
  routes: {
    "/api/health": new Response("OK"),

    "/users/:id": (req) => {
      return Response.json({ id: req.params.id });
    },

    "/api/posts": {
      GET: () => Response.json({ posts: [] }),
      POST: async (req) => {
        const body = await req.json();
        return Response.json({ created: true, ...body }, { status: 201 });
      },
    },

    "/api/*": Response.json({ message: "Not found" }, { status: 404 }),
    "/favicon.ico": new Response(Bun.file("./favicon.ico")),
  },

  fetch() {
    return new Response("Not Found", { status: 404 });
  },

  error(error) {
    console.error(error);
    return new Response("Internal Server Error", { status: 500 });
  },
});

console.log(`Listening on ${server.url}`);
```

Use `fetch(req, server)` instead of `routes` when you need custom dispatch,
streaming setup, or compatibility with older Bun versions.

## Server Lifecycle

```typescript
await server.stop();      // graceful
await server.stop(true);  // close active connections

server.reload({
  routes: {
    "/api/version": Response.json({ version: "2.0.0" }),
  },
});

server.unref(); // allow process exit if this is the only active handle
server.ref();
```

Useful properties and methods:

- `server.url`, `server.port`, `server.hostname`
- `server.pendingRequests`, `server.pendingWebSockets`
- `server.requestIP(req)`
- `server.timeout(req, seconds)` for per-request idle timeout

Default idle timeout is short enough to affect long-lived streams. For SSE or
slow streaming responses, call `server.timeout(req, 0)`.

## WebSocket Upgrade

```typescript
type WsData = { username: string; joinedAt: number };

const server = Bun.serve<WsData>({
  fetch(req, server) {
    const url = new URL(req.url);
    if (url.pathname === "/ws") {
      const ok = server.upgrade(req, {
        data: {
          username: url.searchParams.get("user") ?? "anon",
          joinedAt: Date.now(),
        },
      });
      return ok ? undefined : new Response("Upgrade failed", { status: 500 });
    }
    return new Response("Hello");
  },

  websocket: {
    open(ws) {
      ws.subscribe("chat");
      server.publish("chat", `${ws.data.username} joined`);
    },
    message(ws, message) {
      server.publish("chat", `${ws.data.username}: ${message}`);
    },
    close(ws) {
      ws.unsubscribe("chat");
    },
    drain() {
      // backpressure relieved
    },

    idleTimeout: 120,
    maxPayloadLength: 1024 * 1024,
    perMessageDeflate: true,
    sendPings: true,
  },
});
```

Gotchas:

- On successful `server.upgrade()`, return `undefined`, not a `Response`.
- Type WebSocket `data` with `Bun.serve<WsData>()` or `satisfies`.
- `server.publish()` returns bytes sent, `0` if dropped, and `-1` on backpressure.

## Static and File Responses

```typescript
routes: {
  "/health": new Response("OK"),
  "/download.zip": new Response(Bun.file("./download.zip")),
}
```

Static `Response` objects are cached for the server lifetime. File responses using
`Bun.file()` read from disk per request and support filesystem-driven behavior such
as missing-file 404s and range requests.

## Official Docs

- `https://bun.sh/docs/api/http`
- `https://bun.sh/docs/runtime/http/routing`
- `https://bun.sh/docs/api/websockets`
