import express from "express";
import cors from "cors";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Import WASM functions
const wasmModule = await import("./web/pkg/rbxm_api.js").then(m => m.default || m);
let rbxm_to_json, json_to_rbxm, class_schema_json, class_schemas_json;

// Initialize WASM
async function initWasm() {
  const mod = await wasmModule();
  rbxm_to_json = mod.rbxm_to_json;
  json_to_rbxm = mod.json_to_rbxm;
  class_schema_json = mod.class_schema_json;
  class_schemas_json = mod.class_schemas_json;
}

const app = express();

// Middleware
app.use(cors());
app.use(express.text({ type: "application/octet-stream", limit: "50mb" }));
app.use(express.json({ limit: "50mb" }));
app.use(express.raw({ type: "application/octet-stream", limit: "50mb" }));

// Serve static files from web folder
app.use(express.static(path.join(__dirname, "web")));

// Routes

// Health check
app.get("/health", (req, res) => {
  res.send("ok");
});

// Decode: .rbxm bytes → JSON
app.post("/decode", (req, res) => {
  try {
    let bytes;
    
    // Handle raw binary or base64
    if (typeof req.body === "string") {
      // Base64 input
      bytes = Buffer.from(req.body, "base64");
    } else if (Buffer.isBuffer(req.body)) {
      bytes = req.body;
    } else {
      bytes = Buffer.from(req.body);
    }

    const json = rbxm_to_json(new Uint8Array(bytes));
    res.json(JSON.parse(json));
  } catch (err) {
    res.status(400).json({ error: String(err) });
  }
});

// Encode: JSON → .rbxm bytes
app.post("/encode", (req, res) => {
  try {
    const jsonText = typeof req.body === "string" ? req.body : JSON.stringify(req.body);
    const bytes = json_to_rbxm(jsonText);
    const b64 = Buffer.from(bytes).toString("base64");
    res.json({ data: b64, bytes: bytes.length });
  } catch (err) {
    res.status(400).json({ error: String(err) });
  }
});

// Schema: single class
app.get("/schema/:class", (req, res) => {
  try {
    const schema = class_schema_json(req.params.class);
    res.json(JSON.parse(schema));
  } catch (err) {
    res.status(400).json({ error: String(err) });
  }
});

// Schemas: batch
app.get("/schemas", (req, res) => {
  try {
    const classes = req.query.classes;
    if (!classes) {
      return res.status(400).json({ error: "classes parameter required" });
    }
    const schemas = class_schemas_json(classes);
    res.json(JSON.parse(schemas));
  } catch (err) {
    res.status(400).json({ error: String(err) });
  }
});

// Serve index.html for all other routes (SPA fallback)
app.get("*", (req, res) => {
  res.sendFile(path.join(__dirname, "web", "index.html"));
});

// Start server
const PORT = process.env.PORT || 3000;

initWasm().then(() => {
  app.listen(PORT, () => {
    console.log(`Server running on http://localhost:${PORT}`);
  });
}).catch(err => {
  console.error("Failed to initialize WASM:", err);
  process.exit(1);
});
