import init, {
  rbxm_to_json,
  json_to_rbxm,
  class_schema_json,
  class_schemas_json
} from "./web/pkg/rbxm_api.js";

let wasmReady = false;

// Initialize WASM on worker startup
await init().then(() => {
  wasmReady = true;
  console.log("WASM initialized");
});

export default {
  async fetch(request) {
    if (!wasmReady) {
      return new Response(JSON.stringify({ error: "WASM not initialized" }), {
        status: 500,
        headers: { "Content-Type": "application/json" }
      });
    }

    const url = new URL(request.url);
    const path = url.pathname;
    const method = request.method;

    // CORS headers
    const corsHeaders = {
      "Access-Control-Allow-Origin": "*",
      "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
      "Access-Control-Allow-Headers": "Content-Type"
    };

    // Handle CORS preflight
    if (method === "OPTIONS") {
      return new Response(null, { headers: corsHeaders });
    }

    try {
      // /health
      if (path === "/health") {
        return new Response("ok", {
          status: 200,
          headers: { ...corsHeaders, "Content-Type": "text/plain" }
        });
      }

      // POST /encode - JSON to .rbxm
      if (path === "/encode" && method === "POST") {
        const jsonText = await request.text();
        const bytes = json_to_rbxm(jsonText);
        const b64 = btoa(String.fromCharCode(...bytes));
        return new Response(JSON.stringify({ data: b64, bytes: bytes.length }), {
          status: 200,
          headers: { ...corsHeaders, "Content-Type": "application/json" }
        });
      }

      // POST /decode - .rbxm to JSON
      if (path === "/decode" && method === "POST") {
        const b64 = await request.text();
        const bytes = Uint8Array.from(atob(b64), c => c.charCodeAt(0));
        const json = rbxm_to_json(bytes);
        return new Response(json, {
          status: 200,
          headers: { ...corsHeaders, "Content-Type": "application/json" }
        });
      }

      // GET /schema/:class
      if (path.startsWith("/schema/") && method === "GET") {
        const className = path.split("/").pop();
        const schema = class_schema_json(className);
        return new Response(schema, {
          status: 200,
          headers: { ...corsHeaders, "Content-Type": "application/json" }
        });
      }

      // GET /schemas?classes=Part,Model
      if (path === "/schemas" && method === "GET") {
        const classes = url.searchParams.get("classes");
        if (!classes) {
          return new Response(JSON.stringify({ error: "classes parameter required" }), {
            status: 400,
            headers: { ...corsHeaders, "Content-Type": "application/json" }
          });
        }
        const schemas = class_schemas_json(classes);
        return new Response(schemas, {
          status: 200,
          headers: { ...corsHeaders, "Content-Type": "application/json" }
        });
      }

      // 404
      return new Response(JSON.stringify({ error: "not found" }), {
        status: 404,
        headers: { ...corsHeaders, "Content-Type": "application/json" }
      });
    } catch (err) {
      return new Response(JSON.stringify({ error: String(err) }), {
        status: 400,
        headers: { ...corsHeaders, "Content-Type": "application/json" }
      });
    }
  }
};
