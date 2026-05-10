FROM debian:bookworm-slim

ARG TARGETARCH=amd64

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY gws-${TARGETARCH} /usr/local/bin/gws

RUN useradd -m -u 1000 gws

USER gws

# File-based keyring avoids D-Bus dependency in headless containers.
ENV GOOGLE_WORKSPACE_CLI_KEYRING_BACKEND=file

LABEL org.opencontainers.image.source="https://github.com/shigechika/gws-mcp"
LABEL org.opencontainers.image.description="Google Workspace CLI with MCP server support"
LABEL org.opencontainers.image.licenses="Apache-2.0"
LABEL io.modelcontextprotocol.server.name="io.github.shigechika/gws-mcp"

ENTRYPOINT ["gws"]
CMD ["mcp", "-s", "gmail,drive,calendar", "--helpers"]
