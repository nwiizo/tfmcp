FROM rust:1.88.0-slim-bullseye AS builder

ARG TERRAFORM_VERSION="1.15.8"

# Install dependencies
RUN apt-get update && apt-get install -y \
    curl \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# Install Terraform using direct download method (works for any architecture)
RUN ARCH=$(uname -m) && \
    if [ "$ARCH" = "x86_64" ]; then TERRAFORM_ARCH="amd64"; \
    elif [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then TERRAFORM_ARCH="arm64"; \
    else TERRAFORM_ARCH="$ARCH"; fi && \
    curl -fsSL "https://releases.hashicorp.com/terraform/${TERRAFORM_VERSION}/terraform_${TERRAFORM_VERSION}_linux_${TERRAFORM_ARCH}.zip" -o terraform.zip && \
    unzip terraform.zip && \
    mv terraform /usr/local/bin/ && \
    rm terraform.zip

WORKDIR /app

# Copy only the files needed for dependencies first to leverage Docker cache
COPY Cargo.toml Cargo.lock rust-toolchain.toml build.rs ./

# Create dummy src files to build dependencies (lib + bin structure)
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    echo "" > src/lib.rs && \
    cargo build --release --locked && \
    rm -rf src

# Copy the actual source code
COPY src/ src/
COPY example/ example/

# Rebuild with the actual source code
RUN cargo build --release --locked

# Create the runtime image
FROM debian:bullseye-slim

ARG TERRAFORM_VERSION="1.15.8"
ARG TFMCP_VERSION="0.2.2"
ARG TFMCP_REVISION="unknown"

LABEL io.modelcontextprotocol.server.name="io.github.nwiizo/tfmcp" \
    org.opencontainers.image.title="tfmcp" \
    org.opencontainers.image.description="Local-first Terraform MCP server with Registry lookup, Terraform CLI workflows, plan/state analysis, module health checks, and safety gates." \
    org.opencontainers.image.source="https://github.com/nwiizo/tfmcp" \
    org.opencontainers.image.version="${TFMCP_VERSION}" \
    org.opencontainers.image.revision="${TFMCP_REVISION}"

# Install dependencies for runtime
RUN apt-get update && apt-get install -y \
    curl \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# Install Terraform using direct download method (works for any architecture)
RUN ARCH=$(uname -m) && \
    if [ "$ARCH" = "x86_64" ]; then TERRAFORM_ARCH="amd64"; \
    elif [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then TERRAFORM_ARCH="arm64"; \
    else TERRAFORM_ARCH="$ARCH"; fi && \
    curl -fsSL "https://releases.hashicorp.com/terraform/${TERRAFORM_VERSION}/terraform_${TERRAFORM_VERSION}_linux_${TERRAFORM_ARCH}.zip" -o terraform.zip && \
    unzip terraform.zip && \
    mv terraform /usr/local/bin/ && \
    rm terraform.zip

WORKDIR /app

# Copy the built binary from the builder stage
COPY --from=builder /app/target/release/tfmcp /usr/local/bin/tfmcp
COPY --from=builder /app/example /app/example

# Set environment variables
ENV RUST_LOG=info

# Set the entrypoint
ENTRYPOINT ["tfmcp"]
CMD ["mcp"] 
