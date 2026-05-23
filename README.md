<p align="center">
  <h1 align="center">Inkle</h1>
  <p align="center"><em>Lightweight · Self-hosted · Blog Engine</em></p>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.80%2B-orange?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/license-MIT-blue" alt="License">
  <img src="https://img.shields.io/badge/platform-amd64%20|%20arm64-lightgrey" alt="Platform">
  <img src="https://img.shields.io/badge/memory-%3C30MB-brightgreen" alt="Memory">
</p>

---

Inkle is a lightweight, single-user blog engine written in Rust. Built for low-end devices like Raspberry Pi (4 GB RAM), it delivers a full-featured blogging experience — Markdown editor, FTS5 search, and a Hugo/Hexo-style live theme system — in a single binary with no runtime dependencies.

## Table of Contents

- [Features](#features)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Admin Panel](#admin-panel)
- [Theme System](#theme-system)
  - [Directory Structure](#directory-structure)
  - [theme.json](#themejson)
  - [Template Variables](#template-variables)
  - [Static Assets](#static-assets)
- [API Reference](#api-reference)
- [Tech Stack](#tech-stack)
- [Project Structure](#project-structure)
- [License](#license)

## Features

- **Single-user, no user management** — One admin password. No registration flow, no roles.
- **Dual-mode Markdown editor** — Write in raw Markdown, preview the rendered HTML in real time.
- **Live theme switching** — Drop folders into `themes/`, activate from the admin panel, no restart needed.
- **FTS5 trigram search** — SQLite FTS5 with trigram tokenizer. Works for Latin and CJK text.
- **Tiny resource footprint** — < 30 MB memory, single binary, no runtime dependencies.
- **Multi-architecture** — Pre-built Docker images for `linux/amd64` and `linux/arm64`.
- **Defense in depth** — JWT auth, rate limiting, HTML sanitization, path-traversal protection baked in.

## Quick Start

### Prerequisites

- Rust 1.80+ (or Docker)
- SQLite 3

### Run from source

```bash
git clone https://github.com/example/inkle.git
cd inkle

cp .env.example .env
# Edit .env — set JWT_SECRET and ADMIN_PASSWORD (both required)

cargo run --release
```

Open:
- **Blog** — http://localhost:3000
- **Admin** — http://localhost:3000/admin

### Docker

```bash
mkdir -p data themes

docker-compose up -d
```

Or build and run manually:

```bash
docker build -t inkle .
docker run -d \
  -p 3000:3000 \
  -v "$(pwd)/themes:/app/themes" \
  -v "$(pwd)/data:/app/data" \
  -e JWT_SECRET=your-secret-key \
  -e ADMIN_PASSWORD=your-password \
  inkle
```

### Multi-arch build (Raspberry Pi)

```bash
docker buildx create --name multiarch --use
docker buildx build --platform linux/amd64,linux/arm64 -t inkle .
```

## Configuration

All configuration is set via environment variables (or a `.env` file):

| Variable | Required | Default | Description |
|---|---|---|---|
| `JWT_SECRET` | **Yes** | — | Secret key for JWT signing. The app refuses to start if missing. |
| `ADMIN_PASSWORD` | **Yes** | — | Admin login password. The app refuses to start if empty or missing. |
| `DATABASE_URL` | No | `sqlite:data.db?mode=rwc` | SQLite database path. In Docker, mount `/app/data`. |

Site settings — title, subtitle, posts per page, and social links — are configured from the admin panel at `/admin/settings`.

## Admin Panel

Available at `/admin`. Protected by password + JWT cookie authentication.

| Page | What it does |
|---|---|
| **Dashboard** | Post list with FTS5 search, edit/delete actions, pagination |
| **Editor** | Raw + Preview Markdown editor with live rendering via the preview API |
| **Settings** | Site title, subtitle, posts per page, social links |
| **Themes** | Browse installed themes, activate with one click |

## Theme System

Inkle uses a drop-in theme system inspired by Hugo and Hexo. Each theme is a self-contained folder with Tera templates and static assets.

### Directory Structure

```
themes/
└── your-theme/
    ├── theme.json          # Theme metadata (required)
    ├── templates/          # Tera HTML templates
    │   ├── index.html      # Home page (required)
    │   ├── post.html       # Single post (required)
    │   ├── category.html   # Category listing (required)
    │   ├── tag.html        # Tag listing (required)
    │   ├── search.html     # Search results (required)
    │   └── 404.html        # Not found (optional)
    └── static/             # CSS, JS, images
        └── style.css
```

### theme.json

```json
{
    "name": "my-theme",
    "title": "My Theme",
    "version": "1.0.0",
    "author": "Your Name",
    "description": "A custom blog theme",
    "thumbnail": "screenshot.png"
}
```

| Field | Required | Notes |
|---|---|---|
| `name` | **Yes** | Must match the folder name exactly. |
| `title` | **Yes** | Display name shown in the admin panel. |
| `version` | No | Semantic version. |
| `author` | No | Theme author name. |
| `description` | No | Shown on the theme card. |
| `thumbnail` | No | Path relative to the theme root. Falls back to a grey placeholder. |

### Template Variables

All templates receive a `global` context object with site configuration. Page-specific variables are documented below.

#### Global (all templates)

| Variable | Type | Description |
|---|---|---|
| `global.config.site_title` | `String` | Site title |
| `global.config.site_subtitle` | `String` | Site subtitle (may be empty) |
| `global.config.active_theme` | `String` | Current theme name |
| `global.config.posts_per_page` | `u32` | Posts per page |
| `global.config.social_links` | `Map<String, String>` | Social links (platform → URL) |
| `global.current_theme` | `String` | Same as active theme |
| `global.current_url` | `String` | Current request URL path |

#### `index.html`

| Variable | Type | Description |
|---|---|---|
| `posts` | `Vec<Post>` | Published posts for the current page |
| `pagination` | `Pagination` | Pagination info |

#### `post.html`

| Variable | Type | Description |
|---|---|---|
| `post` | `Post` | The current post |

#### `category.html` / `tag.html`

| Variable | Type | Description |
|---|---|---|
| `current_category` / `current_tag` | `String` | Category or tag name |
| `posts` | `Vec<Post>` | Posts in this category/with this tag |
| `total_posts` | `i64` | Total matching posts |
| `pagination` | `Pagination` | Pagination info |

#### `search.html`

| Variable | Type | Description |
|---|---|---|
| `query` | `String` | Search query string |
| `posts` | `Vec<Post>` | Search results (includes `snippet` field) |
| `total_results` | `i64` | Total result count |
| `pagination` | `Pagination` | Pagination info |

#### `Post` struct

| Field | Type | Description |
|---|---|---|
| `post.title` | `String` | Post title |
| `post.slug` | `String` | URL-friendly identifier |
| `post.category` | `String` | Category (may be empty) |
| `post.tags` | `Vec<String>` | Tag list |
| `post.content` | `String` | Rendered HTML (pre-sanitized) |
| `post.snippet` | `Option<String>` | Search result snippet (HTML, pre-sanitized) |
| `post.created_at` | `String` | ISO 8601 timestamp |
| `post.updated_at` | `String` | ISO 8601 timestamp |

> **Note:** `post.content` is pre-rendered and sanitized HTML. Use `{{ post.content | safe }}` to avoid double-escaping. `post.snippet` contains only `<mark>` highlight tags and is safe for `| safe` as well.

#### `Pagination` struct

| Field | Type | Description |
|---|---|---|
| `pagination.current` | `usize` | Current page (1-based) |
| `pagination.total` | `usize` | Total pages |
| `pagination.prev` | `Option<usize>` | Previous page number |
| `pagination.next` | `Option<usize>` | Next page number |

### Static Assets

Files in `themes/{name}/static/` are served at `/static/theme/`:

```
themes/my-theme/static/style.css  →  /static/theme/style.css
themes/my-theme/static/logo.png   →  /static/theme/logo.png
```

Reference them with absolute paths:

```html
<link rel="stylesheet" href="/static/theme/style.css">
<img src="/static/theme/logo.png" alt="Logo">
```

### Switching Themes

1. Place your theme folder under `themes/`
2. Go to `/admin/themes`
3. Click **Activate** on your theme
4. The blog switches instantly — no restart required

## API Reference

### Public

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/api/health` | — | Health check |
| `POST` | `/api/login` | — | Login (returns JWT cookie; rate-limited) |
| `POST` | `/api/logout` | — | Logout (clears cookie) |
| `POST` | `/api/preview` | — | Render Markdown → sanitized HTML |
| `GET` | `/api/search?q=` | — | Full-text search (published posts only) |

### Admin (JWT required)

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/admin/posts` | List posts (paginated) |
| `POST` | `/api/admin/posts` | Create post |
| `GET` | `/api/admin/posts/{id}` | Get post |
| `PUT` | `/api/admin/posts/{id}` | Update post |
| `DELETE` | `/api/admin/posts/{id}` | Delete post |
| `GET` | `/api/admin/settings` | Get site settings |
| `PUT` | `/api/admin/settings` | Update site settings |
| `GET` | `/api/admin/stats` | Category & tag statistics |
| `GET` | `/api/admin/themes` | List installed themes |
| `POST` | `/api/admin/themes/{name}/activate` | Activate a theme |

## Tech Stack

| Layer | Technology |
|---|---|
| Language | Rust (Edition 2021) |
| Web framework | [Axum](https://github.com/tokio-rs/axum) 0.8 |
| Database | SQLite via [sqlx](https://github.com/launchbadge/sqlx) 0.8 |
| Templates | [Tera](https://github.com/Keats/tera) 1.x |
| Markdown | [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark) 0.12 |
| HTML sanitization | [ammonia](https://github.com/rust-ammonia/ammonia) 4 |
| Search | SQLite FTS5 (trigram tokenizer) |
| Auth | JWT ([jsonwebtoken](https://github.com/Keats/jsonwebtoken) 9) + HttpOnly cookies |
| Admin UI | Vanilla JavaScript — no frameworks |

## Project Structure

```
inkle/
├── Cargo.toml
├── Dockerfile
├── docker-compose.yml
├── .env.example
├── README.md
├── README_zh.md
├── migrations/
│   ├── 001_init.sql           # Posts & configs tables
│   ├── 002_fts.sql            # FTS5 virtual table
│   ├── 003_unicode61.sql      # CJK tokenizer support
│   ├── 004_search_v2.sql      # Trigram tokenizer + search_logs
│   └── 005_post_tags.sql      # Normalized post_tags junction table
├── themes/
│   ├── default/               # "Soft Curve" — light glassmorphism theme
│   └── nord/                  # "Nord" — dark arctic theme
├── admin_templates/           # Admin panel Tera templates
│   ├── base.html
│   ├── login.html
│   ├── dashboard.html
│   ├── editor.html
│   ├── settings.html
│   ├── themes.html
│   └── 404.html
├── static/                    # Admin panel static assets
│   ├── admin.css
│   └── admin.js
└── src/
    ├── main.rs
    ├── config.rs
    ├── db.rs
    ├── models.rs
    ├── state.rs
    ├── handlers/
    │   ├── mod.rs
    │   ├── auth.rs
    │   ├── admin_post.rs
    │   ├── admin_pages.rs
    │   ├── frontend.rs
    │   ├── preview.rs
    │   ├── search.rs
    │   ├── settings.rs
    │   └── theme.rs
    └── services/
        ├── mod.rs
        ├── markdown.rs
        └── theme.rs
```

## License

[MIT](LICENSE)
