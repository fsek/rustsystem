Building wasm (requires `wasm-pack`):
```bash
RUSTFLAGS='--cfg getrandom_backend="wasm_js"' wasm-pack build --target web
```
