ARG API_ENDPOINT=https://rosta.fsektionen.se

# Stage 1: Build the WebAssembly client
FROM rust:1.91-bullseye AS wasm-builder
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
WORKDIR /app
COPY . .
WORKDIR /app/frontend/rustsystem-client
ARG API_ENDPOINT
RUN wasm-pack build --target web -d ../src/pkg

# Stage 2: Build the frontend
FROM node:24-bullseye AS frontend-builder
RUN npm install -g pnpm
WORKDIR /app/frontend
COPY frontend/package.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml ./
COPY frontend/rustsystem-client/Cargo.toml ./rustsystem-client/
RUN pnpm install --frozen-lockfile
COPY frontend/ .
COPY --from=wasm-builder /app/frontend/src/pkg ./src/pkg
ARG API_ENDPOINT
RUN pnpm run build
RUN ls
# Stage 3: Build the Rust backend
FROM rust:1.91-bullseye AS backend-builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY rustsystem-server/ ./rustsystem-server/
COPY rustsystem-proof/ ./rustsystem-proof/
COPY rustsystem-server-api/ ./rustsystem-server-api/
COPY frontend/rustsystem-client/ ./frontend/rustsystem-client/
ARG API_ENDPOINT
RUN cargo build --release --bin rustsystem-server

# Stage 4: Runtime image
FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built backend binary
COPY --from=backend-builder /app/target/release/rustsystem-server ./rustsystem-server

# Copy the built frontend
COPY --from=frontend-builder /app/frontend/dist ./frontend/dist

# Create a non-root user
RUN useradd -m -u 1000 appuser && chown -R appuser:appuser /app
USER appuser

EXPOSE 3000

CMD ["./rustsystem-server"]
