# === Go CLI ビルド ===
FROM golang:1.24-alpine AS cli-builder
ENV GOTOOLCHAIN=auto
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY cmd/ cmd/
COPY internal/ internal/
RUN go build -o /syllabus-cli ./cmd/syllabus-cli

# === フロントエンドビルド ===
FROM oven/bun:latest AS web-builder
WORKDIR /app
COPY web/package.json web/bun.lock* ./
RUN bun install --frozen-lockfile 2>/dev/null || bun install
COPY web/ .
RUN bun run build

# === CLI 実行用イメージ ===
FROM alpine:latest AS cli
COPY --from=cli-builder /syllabus-cli /usr/local/bin/syllabus-cli
WORKDIR /data
ENTRYPOINT ["syllabus-cli"]

# === Web サーバー (本番用: 静的ファイル配信) ===
FROM nginx:alpine AS web
COPY --from=web-builder /app/dist /usr/share/nginx/html
EXPOSE 80
