![cover image](./static/og-image.png)

**B{ook}log** is a self-hosted, multi-user book tracking platform for avid readers. Each user
maintains a personal library and wishlist, with per-user readings, stats, and timeline.

Booklog features an LLM-powered extraction feature, which enables it to automatically fill
book and author information using a photo of a book cover.

Booklog ships as a single Rust binary that serves a web UI, a REST API, and a CLI client. The
application uses SQLite as a backend, and will automatically create and migrate the database on
start-up.

> [!NOTE]
> This application was built almost entirely with Claude Code. I used this project as an
> excuse to explore the current state of the art in agentic coding tools. I've reviewed
> much of the code, and I'm largely responsible for the rules and the layout of the repository
> but nonetheless the vast majority of the code was written by Claude and Opus 4.6.

## Quick Start (Demo)

Optionally, sign up for [OpenRouter](https://openrouter.ai) to enable AI-powered book cover
extraction. Then create a `docker.env` file:

```env
# Optional: API key from OpenRouter for AI extraction
BOOKLOG_OPENROUTER_API_KEY=sk-or-...
# I've had good results with Gemini models, but you can try 'openrouter/free' to experiment
BOOKLOG_OPENROUTER_MODEL=google/gemini-3-flash-preview
```

See the full list of configuration options [below](#configuration). Once the `.env` file is
ready, start the container using the environment file:

```bash
# Create a data directory to store the database
mkdir data
# Run the container
docker run \
  --rm \
  -p 3000 \
  --env-file docker.env \
  -v $PWD/data:/data \
  ghcr.io/jnsgruk/booklog:latest
```

On first start with an empty database the server prints a one-time registration URL:

```
No users found. Register the first user at:
  http://localhost:3000/register/abc123...
This link expires in 1 hour.
```

Open that URL, choose a display name, and register a passkey. This creates an admin account
and signs in automatically. Additional users can be invited from the admin page.

### Install from Git

To build and install from source, a working Rust toolchain is required:

```bash
cargo install --locked --git https://github.com/jnsgruk/booklog.git
```

Then create a `.env` file and start the server:

```bash
booklog serve
```

### CLI Authentication

To use the CLI or API for write operations, create a token via browser hand-off:

```bash
booklog token create --name "my-cli-token"
# Browser opens → authenticate with a passkey → token printed once

export BOOKLOG_URL="http://localhost:3000"
export BOOKLOG_TOKEN="<token from above>"

# Create data from the CLI
booklog author add --name "George Orwell" --nationality "British"
```

Run `booklog --help` for the full command reference.

### Maintenance

Rebuild timeline event snapshots (e.g. after editing entities whose changes need to propagate):

```bash
booklog timeline rebuild
```

## Configuration

All settings are read from environment variables or CLI flags. A `.env` file in the working
directory is loaded automatically via [dotenvy](https://crates.io/crates/dotenvy).

### Server (`booklog serve`)

| Variable                   | Purpose                                                                | Default                 |
| -------------------------- | ---------------------------------------------------------------------- | ----------------------- |
| `BOOKLOG_RP_ID`            | WebAuthn Relying Party ID (server domain)                              | `localhost`             |
| `BOOKLOG_RP_ORIGIN`        | WebAuthn Relying Party origin (full URL)                               | `http://localhost:3000` |
| `BOOKLOG_DATABASE_URL`     | Database connection string                                             | `sqlite://booklog.db`   |
| `BOOKLOG_BIND_ADDRESS`     | Server bind address                                                    | `127.0.0.1:3000`        |
| `BOOKLOG_INSECURE_COOKIES` | Disable the `Secure` cookie flag (auto-enabled for localhost defaults) | `false`                 |
| `RUST_LOG`                 | Log level filter                                                       | `info`                  |
| `RUST_LOG_FORMAT`          | Set to `json` for structured log output                                | —                       |

### CLI Client

| Variable        | Purpose                               | Default                 |
| --------------- | ------------------------------------- | ----------------------- |
| `BOOKLOG_URL`   | Server URL                            | `http://localhost:3000` |
| `BOOKLOG_TOKEN` | API bearer token for write operations | —                       |

### Integrations

| Variable                     | Purpose                                                        | Default           |
| ---------------------------- | -------------------------------------------------------------- | ----------------- |
| `BOOKLOG_OPENROUTER_API_KEY` | [OpenRouter](https://openrouter.ai/) API key for AI extraction | (optional)        |
| `BOOKLOG_OPENROUTER_MODEL`   | LLM model for AI extraction                                    | `openrouter/free` |

## Contributing

```bash
cargo build                           # Build
cargo clippy --allow-dirty --fix      # Lint
cargo fmt                             # Format
cargo test                            # Test
```

See [CLAUDE.md](CLAUDE.md) for architecture, code patterns, and development conventions.

## License

[Apache License 2.0](LICENSE)
