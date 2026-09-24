# rbxm-api

HTTP service that turns instance trees into real `.rbxm` files (and back) using
[rbx-dom](https://github.com/rojo-rbx/rbx-dom), plus a Luau `RBXM` ModuleScript that talks to it.
Lets normal (non-plugin) server scripts do what `SerializationService` can't.

```
Luau (HttpService)  --JSON tree-->  POST /encode  --> .rbxm bytes  --> buffer
Luau (HttpService)  --.rbxm bytes-> POST /decode  --> JSON tree     --> Instances
```

## Run the server

Needs Rust 1.85+ (`rustup update`).

```
RBXM_API_KEY=some-long-random-string cargo run --release
```

| env var | default | meaning |
|---|---|---|
| `RBXM_API_KEY` | *(unset = no auth!)* | required in the `x-api-key` header |
| `PORT` | 8080 | listen port |

Put it behind a reverse proxy (Caddy / nginx) for HTTPS and **rate limiting** — the server itself
doesn't rate limit. Roblox allows 500 HttpService requests/min per game server.

## Luau side

1. Enable *Game Settings > Security > Allow HTTP Requests*.
2. Put `luau/RBXM.lua` in a ModuleScript, use it from **server** scripts only. Never ship the API key to clients.
3. See `luau/Example.server.lua`.

## Endpoints

| | |
|---|---|
| `GET /health` | `ok` |
| `GET /schema/:class` | serializable + script-readable properties of a class and their types (from `rbx_reflection_database`) |
| `GET /schemas?classes=A,B,C` | batch version of the above |
| `POST /encode` | body: JSON tree → response: `.rbxm` bytes (`application/octet-stream`) |
| `POST /decode` | body: `.rbxm` bytes → response: JSON tree |

Add `?b64=1` to `/encode` / `/decode` to send/receive base64 text instead of raw bytes
(`Base64 = true` in the Luau config) if raw binary bodies ever get mangled in transit.
Errors are `{"error": "..."}` with 4xx status.

## Wire format

```json
{ "instances": [
  { "id": "1", "class": "Model", "name": "Car", "parent": null, "properties": {} },
  { "id": "2", "class": "Part",  "name": "Body", "parent": "1", "properties": {
      "Size":     { "t": "Vector3", "v": [4, 1, 2] },
      "Material": { "t": "Enum",    "v": 272 }
  } }
] }
```

`parent: null` = top-level in the file. `Ref` values are instance `id`s (or `null`).
On encode the reflection database decides each property's real type (`t` is only used for
properties it doesn't know). Property names are the normal API names (`Size`, `Color`, …).

| type | `v` |
|---|---|
| Bool / Int32 / Int64 / Float32 / Float64 / String | JSON primitive |
| BinaryString / SharedString | base64 string |
| Vector2 / Vector3 / Vector2int16 / Vector3int16 | `[x,y]` / `[x,y,z]` |
| CFrame | 12 numbers, same order as `CFrame:GetComponents()` |
| Color3 / Color3uint8 | `[r,g,b]` (0–1 / 0–255) |
| BrickColor | palette number |
| UDim / UDim2 / Rect / Ray / NumberRange | `[scale,offset]` / `[xs,xo,ys,yo]` / `[minx,miny,maxx,maxy]` / 6 numbers / `[min,max]` |
| NumberSequence / ColorSequence | `[[t,v,env],…]` / `[[t,r,g,b],…]` |
| PhysicalProperties | `null` (default) or `[density,friction,elasticity,fWeight,eWeight]` |
| Enum | number (decode also sends `"e": "EnumName"`) |
| Faces / Axes | bitmask |
| Font | `{family, weight, style}` |
| Content / ContentId | string |
| Tags | `["a","b"]` |
| Attributes | `{ name: { "t": "...", "v": ... } }` |
| Ref | id string or `null` |

## Known limits

* `Script.Source` can't be read or written by non-plugin code in Roblox, so encoded scripts have no source unless you pass it
  via `Encode(..., { Properties = { [script] = { Source = "..." } } })`, and decoded scripts come out empty.
* Not carried by the wire format yet: `UniqueId`, `MaterialColors`, `SecurityCapabilities`, `Region3*`, `NetAssetRef`.
* Decoded properties that Roblox doesn't let scripts set are skipped (set `Verbose = true` to list them).
* `Color` decodes from the file's `Color3uint8` (8-bit precision), like any .rbxm.
* Newer Roblox properties/classes appear when the `rbx_reflection_database` crate is updated (`cargo update`).
