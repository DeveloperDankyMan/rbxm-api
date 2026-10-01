import init, {
  rbxm_to_json,
  json_to_rbxm,
  encode_to_base64,
  decode_from_base64,
  class_schema_json,
  class_schemas_json
} from "./pkg/rbxm_api.js";

let wasmReady = false;

// Initialize WASM on page load
await init();
wasmReady = true;
console.log("WASM initialized - API routes ready!");

// Parse URL and route requests
function parseRequest() {
  const path = window.location.pathname;
  const params = new URLSearchParams(window.location.search);
  
  return { path, params };
}

// Route handler
async function handleRoute() {
  const { path, params } = parseRequest();
  const segments = path.split("/").filter(Boolean);
  
  // Match routes like /rbxm-api/encode, /rbxm-api/decode, etc.
  const route = segments[segments.length - 1];
  
  let response;

  try {
    switch (route) {
      case "health":
        response = { body: "ok", contentType: "text/plain", status: 200 };
        break;

      case "encode": {
        // POST /encode - JSON body → .rbxm bytes (base64)
        const jsonData = params.get("data") || await getPostBody();
        const b64 = encode_to_base64(jsonData);
        response = {
          body: JSON.stringify({ data: b64, bytes: b64.length }),
          contentType: "application/json",
          status: 200
        };
        break;
      }

      case "decode": {
        // POST /decode - base64 body → JSON
        const b64 = params.get("data") || await getPostBody();
        const json = decode_from_base64(b64);
        response = {
          body: json,
          contentType: "application/json",
          status: 200
        };
        break;
      }

      case "schema": {
        // GET /schema/:class
        const className = params.get("class") || segments[segments.length - 2];
        if (!className) {
          throw new Error("class parameter required");
        }
        const schema = class_schema_json(className);
        response = {
          body: schema,
          contentType: "application/json",
          status: 200
        };
        break;
      }

      case "schemas": {
        // GET /schemas?classes=Part,Model,Script
        const classes = params.get("classes");
        if (!classes) {
          throw new Error("classes parameter required");
        }
        const schemas = class_schemas_json(classes);
        response = {
          body: schemas,
          contentType: "application/json",
          status: 200
        };
        break;
      }

      default:
        response = {
          body: JSON.stringify({ error: "not found" }),
          contentType: "application/json",
          status: 404
        };
    }
  } catch (err) {
    response = {
      body: JSON.stringify({ error: String(err) }),
      contentType: "application/json",
      status: 400
    };
  }

  return response;
}

// Helper: get POST body
function getPostBody() {
  return new Promise((resolve) => {
    // For now, return empty - in a real Service Worker this would be req.text()
    resolve("");
  });
}

// Export for use in HTML
window.rbxmApi = {
  handleRoute,
  wasmReady: () => wasmReady
};
