# RBXM Compatibility API

This project is a scaffold for an RBXM compatibility API that follows the public binary model conventions used by Rojo and the `rbx-dom` ecosystem.

Important honesty note:
- This is not a private Roblox `SerializationService` implementation.
- It is a public-format compatibility service intended to normalize model data and produce/consume RBXM-style binary data using a documented model.
- For true native `.rbxm` compatibility in Studio, the recommended path is to use the native import/export pipeline or a server-side compatibility service built around `rbx-dom`.

## Goals

- Expose a clean HTTP API for RBXM encode/decode/validate work
- Accept a neutral Roblox instance DOM payload from a client
- Return RBXM-compatible document bytes in a transport-safe form (base64 or file upload)
- Keep the implementation aligned with public Rojo model semantics rather than private Roblox internals

## API surface

- `GET /health`
- `POST /v1/rbxm/encode`
- `POST /v1/rbxm/decode`
- `POST /v1/rbxm/validate`

## Architecture

```text
Roblox Script / Studio Plugin
        │
        │ JSON DOM payload
        ▼
RBXM API Service
        │
        ├── Public model document schema
        ├── RBXM encode/decode adapter layer
        ├── Validation / referent checks
        └── Future rbx-dom integration
```

## Notes

This repository is intentionally a scaffold. The next step is to integrate with a concrete public RBXM reference implementation and add the actual binary encode/decode logic.

The server currently exposes a minimal working API contract while keeping the logic separated for future public-format compatibility work.
