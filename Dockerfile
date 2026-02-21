ARG API_ENDPOINT=https://rosta.fsektionen.se
# NOTE: SALT_HEX is different from the value in .env. This value is not secret, only used to maintain uniqueness. It's purposefully different from the development value in order to maintain uniqueness in production.
ARG SALT_HEX=fa592f8bf54e9e6710f9e63699651c9d 
ARG KEYGEN_ITERATIONS=200000

# Stage 2: Build the frontend
FROM node:24-bullseye AS frontend-builder
RUN npm install -g pnpm
WORKDIR /app/frontend
COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY frontend/ .
ARG API_ENDPOINT
ARG SALT_HEX
ARG KEYGEN_ITERATIONS
RUN pnpm run build
RUN ls

# Stage 3: Build the Rust backend
FROM rust:1.91-bullseye AS backend-builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY rustsystem-server/ ./rustsystem-server/
COPY rustsystem-server-api/ ./rustsystem-server-api/
COPY decrypt-tally/ ./decrypt-tally/
ARG API_ENDPOINT
ARG SALT_HEX
ARG KEYGEN_ITERATIONS
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
