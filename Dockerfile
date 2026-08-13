FROM rust:1.97.1-slim-trixie@sha256:8e8cf8f7fd54a2d23d5a743b3a03f56e26b6c774276c33fa0595111704ebb15c AS builder

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

FROM debian:trixie-slim@sha256:3a39a0592364683e6bab97937b72cad5a8fa6dcbbee90edb3bb48c7f8e94f258 AS terraform

ARG TARGETARCH
ARG TERRAFORM_VERSION="1.15.8"

# Package versions are fixed by the immutable base-image digest; allow security
# updates when Dependabot refreshes that digest.
# hadolint ignore=DL3008
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    unzip \
    && case "${TARGETARCH}" in amd64|arm64) ;; *) echo "unsupported architecture: ${TARGETARCH}" >&2; exit 1 ;; esac \
    && curl -fsSL "https://releases.hashicorp.com/terraform/${TERRAFORM_VERSION}/terraform_${TERRAFORM_VERSION}_linux_${TARGETARCH}.zip" -o terraform.zip \
    && unzip terraform.zip -d /opt/terraform \
    && rm terraform.zip \
    && rm -rf /var/lib/apt/lists/*

# Create the runtime image
FROM debian:trixie-slim@sha256:3a39a0592364683e6bab97937b72cad5a8fa6dcbbee90edb3bb48c7f8e94f258

ARG TFMCP_VERSION="0.2.2"
ARG TFMCP_REVISION="unknown"

LABEL io.modelcontextprotocol.server.name="io.github.nwiizo/tfmcp" \
    org.opencontainers.image.title="tfmcp" \
    org.opencontainers.image.description="Local-first Terraform MCP server with Registry lookup, Terraform CLI workflows, plan/state analysis, module health checks, and safety gates." \
    org.opencontainers.image.source="https://github.com/nwiizo/tfmcp" \
    org.opencontainers.image.version="${TFMCP_VERSION}" \
    org.opencontainers.image.revision="${TFMCP_REVISION}"

# Keep only the runtime trust store; download tools stay in the Terraform stage.
# hadolint ignore=DL3008
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary from the builder stage
COPY --from=builder /app/target/release/tfmcp /usr/local/bin/tfmcp
COPY --from=builder /app/example /app/example
COPY --from=terraform /opt/terraform/terraform /usr/local/bin/terraform

# Set environment variables
ENV RUST_LOG=info

# Set the entrypoint
ENTRYPOINT ["tfmcp"]
CMD ["mcp"]
