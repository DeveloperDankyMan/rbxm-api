import init, {
  rbxm_to_json,
  json_to_rbxm,
  class_schema_json,
  class_schemas_json
} from "./pkg/rbxm_api.js";

let wasmReady = false;

// Initialize WASM
await init();
wasmReady = true;

// Simple HTTP server routing
export async function handleRequest(request) {
  const url = new URL(request.url);
  const path = url.pathname;
  const method = request.method;

  // Match routes
  if (path === "/health" || path === "/rbxm-api/health") {
    return new Response("ok", { status: 200, headers: { "Content-Type": "text/plain" } });
  }

  if ((path === "/encode" || path === "/rbxm-api/encode") && method === "POST") {
    try {
      const jsonText = await request.text();
      const bytes = json_to_rbxm(jsonText);
      return new Response(JSON.stringify({ data: Buffer.from(bytes).toString("base64"), bytes: bytes.length }), {
        status: 200,
        headers: { "Content-Type": "application/json" }
      });
    } catch (err) {
      return new Response(JSON.stringify({ error: String(err) }), { status: 400, headers: { "Content-Type": "application/json" } });
    }
  }

  if ((path === "/decode" || path === "/rbxm-api/decode") && method === "POST") {
    try {
      const b64 = await request.text();
      const bytes = Buffer.from(b64, "base64");
      const json = rbxm_to_json(new Uint8Array(bytes));
      return new Response(json, { status: 200, headers: { "Content-Type": "application/json" } });
    } catch (err) {
      return new Response(JSON.stringify({ error: String(err) }), { status: 400, headers: { "Content-Type": "application/json" } });
    }
  }

  if ((path.startsWith("/schema/") || path.startsWith("/rbxm-api/schema/")) && method === "GET") {
    try {
      const className = path.split("/").pop();
      const schema = class_schema_json(className);
      return new Response(schema, { status: 200, headers: { "Content-Type": "application/json" } });
    } catch (err) {
      return new Response(JSON.stringify({ error: String(err) }), { status: 400, headers: { "Content-Type": "application/json" } });
    }
  }

  if ((path === "/schemas" || path === "/rbxm-api/schemas") && method === "GET") {
    try {
      const classes = url.searchParams.get("classes");
      if (!classes) {
        return new Response(JSON.stringify({ error: "classes parameter required" }), { status: 400, headers: { "Content-Type": "application/json" } });
      }
      const schemas = class_schemas_json(classes);
      return new Response(schemas, { status: 200, headers: { "Content-Type": "application/json" } });
    } catch (err) {
      return new Response(JSON.stringify({ error: String(err) }), { status: 400, headers: { "Content-Type": "application/json" } });
    }
  }

  // 404
  return new Response(JSON.stringify({ error: "not found" }), { status: 404, headers: { "Content-Type": "application/json" } });
}
