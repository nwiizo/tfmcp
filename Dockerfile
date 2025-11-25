FROM rust:1.84.0-slim-bullseye AS builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    curl \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# Install Terraform using direct download method (works for any architecture)
RUN TERRAFORM_VERSION="1.11.1" && \
    ARCH=$(uname -m) && \
    if [ "$ARCH" = "x86_64" ]; then TERRAFORM_ARCH="amd64"; \
    elif [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then TERRAFORM_ARCH="arm64"; \
    else TERRAFORM_ARCH="$ARCH"; fi && \
    curl -fsSL "https://releases.hashicorp.com/terraform/${TERRAFORM_VERSION}/terraform_${TERRAFORM_VERSION}_linux_${TERRAFORM_ARCH}.zip" -o terraform.zip && \
    unzip terraform.zip && \
    mv terraform /usr/local/bin/ && \
    rm terraform.zip

WORKDIR /app

# Copy only the files needed for dependencies first to leverage Docker cache
COPY Cargo.toml rust-toolchain.toml build.rs ./

# Create dummy src files to build dependencies (lib + bin structure)
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    echo "" > src/lib.rs && \
    cargo build --release && \
    rm -rf src

# Copy the actual source code
COPY src/ src/
COPY example/ example/

# Rebuild with the actual source code
RUN cargo build --release

# Create the runtime image
FROM debian:bullseye-slim

# Install dependencies for runtime
RUN apt-get update && apt-get install -y \
    curl \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# Install Terraform using direct download method (works for any architecture)
RUN TERRAFORM_VERSION="1.11.1" && \
    ARCH=$(uname -m) && \
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
