<p align="center">
  <h1 align="center">Inkle</h1>
  <p align="center"><em>轻量级 · 私有化部署 · 博客引擎</em></p>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.80%2B-orange?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/license-MIT-blue" alt="License">
  <img src="https://img.shields.io/badge/platform-amd64%20|%20arm64-lightgrey" alt="Platform">
  <img src="https://img.shields.io/badge/%E5%86%85%E5%AD%98-%3C30MB-brightgreen" alt="Memory">
</p>

---

Inkle 是一个用 Rust 编写的轻量级单用户博客引擎。专为树莓派等低功耗设备（4 GB 内存）设计，提供 Markdown 编辑器、FTS5 全文搜索、Hugo/Hexo 风格的热切换主题系统等完整功能，单二进制文件运行，无运行时依赖。

## 目录

- [功能特性](#功能特性)
- [快速开始](#快速开始)
- [配置说明](#配置说明)
- [管理后台](#管理后台)
- [主题系统](#主题系统)
  - [目录结构](#目录结构)
  - [theme.json](#themejson)
  - [模板变量](#模板变量)
  - [静态资源](#静态资源)
- [API 参考](#api-参考)
- [技术栈](#技术栈)
- [项目结构](#项目结构)
- [开源协议](#开源协议)

## 功能特性

- **单用户博客** — 一个管理员密码，无注册流程、无角色管理。
- **Markdown 双模式编辑器** — Raw 纯文本 + Preview 实时预览，一键切换。
- **主题热切换** — 将主题文件夹放入 `themes/`，后台激活即生效，无需重启服务。
- **FTS5 trigram 全文搜索** — SQLite FTS5 trigram 分词器，同时支持拉丁语系和中文/日文/韩文。
- **极低资源占用** — < 30 MB 运行内存，单二进制文件，零运行时依赖。
- **多架构支持** — Docker 镜像兼容 `linux/amd64` 和 `linux/arm64`。
- **纵深安全防御** — JWT 认证、登录速率限制、HTML 内容过滤、路径遍历防护，均为内置功能。

## 快速开始

### 环境要求

- Rust 1.80+（使用 Docker 则无需安装）
- SQLite 3

### 从源码运行

```bash
git clone https://github.com/example/inkle.git
cd inkle

cp .env.example .env
# 编辑 .env — JWT_SECRET 和 ADMIN_PASSWORD 均为必填项

cargo run --release
```

访问：
- **博客前台** — http://localhost:3000
- **管理后台** — http://localhost:3000/admin

### Docker 部署

```bash
mkdir -p data themes

docker-compose up -d
```

或手动构建运行：

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

### 多架构构建（适用于树莓派）

```bash
docker buildx create --name multiarch --use
docker buildx build --platform linux/amd64,linux/arm64 -t inkle .
```

## 配置说明

所有配置通过环境变量（或 `.env` 文件）设置：

| 变量 | 必填 | 默认值 | 说明 |
|---|---|---|---|
| `JWT_SECRET` | **是** | — | JWT 签名密钥。未设置时应用拒绝启动。 |
| `ADMIN_PASSWORD` | **是** | — | 管理员登录密码。为空或未设置时应用拒绝启动。 |
| `DATABASE_URL` | 否 | `sqlite:data.db?mode=rwc` | SQLite 数据库路径。Docker 中请挂载 `/app/data`。 |

站点标题、副标题、每页文章数、社交链接等在后台 `/admin/settings` 页面配置。

## 管理后台

后台入口 `/admin`，使用密码 + JWT Cookie 认证。

| 页面 | 功能 |
|---|---|
| **仪表盘** | 文章列表，支持 FTS5 搜索、编辑、删除、分页 |
| **编辑器** | Raw + Preview 双模式 Markdown 编辑器，通过 API 实时渲染 |
| **站点设置** | 站点标题、副标题、每页文章数、社交链接 |
| **主题管理** | 浏览已安装主题，一键激活 |

## 主题系统

Inkle 采用 Hugo/Hexo 风格的文件夹即主题设计。每个主题是一个包含 Tera 模板和静态资源的独立文件夹。

### 目录结构

```
themes/
└── your-theme/
    ├── theme.json          # 主题元数据（必填）
    ├── templates/          # Tera HTML 模板
    │   ├── index.html      # 首页（必填）
    │   ├── post.html       # 文章详情页（必填）
    │   ├── category.html   # 分类列表页（必填）
    │   ├── tag.html        # 标签列表页（必填）
    │   ├── search.html     # 搜索结果页（必填）
    │   └── 404.html        # 404 页面（可选）
    └── static/             # 静态资源：CSS、JS、图片
        └── style.css
```

### theme.json

```json
{
    "name": "my-theme",
    "title": "我的主题",
    "version": "1.0.0",
    "author": "作者名",
    "description": "一个自定义博客主题",
    "thumbnail": "screenshot.png"
}
```

| 字段 | 必填 | 说明 |
|---|---|---|
| `name` | **是** | 主题唯一标识，必须与文件夹名称一致。 |
| `title` | **是** | 显示在后台主题卡片上的展示名称。 |
| `version` | 否 | 语义版本号。 |
| `author` | 否 | 主题作者。 |
| `description` | 否 | 主题简介。 |
| `thumbnail` | 否 | 缩略图路径（相对主题根目录）。缺失时显示灰色占位符。 |

### 模板变量

所有模板均接收 `global` 全局上下文对象。各页面专属变量如下。

#### 全局上下文（所有模板可用）

| 变量 | 类型 | 说明 |
|---|---|---|
| `global.config.site_title` | `String` | 站点标题 |
| `global.config.site_subtitle` | `String` | 站点副标题（可能为空） |
| `global.config.active_theme` | `String` | 当前主题名称 |
| `global.config.posts_per_page` | `u32` | 每页文章数 |
| `global.config.social_links` | `Map<String, String>` | 社交链接（平台名 → URL） |
| `global.current_theme` | `String` | 同 active_theme |
| `global.current_url` | `String` | 当前请求的 URL 路径 |

#### `index.html`（首页）

| 变量 | 类型 | 说明 |
|---|---|---|
| `posts` | `Vec<Post>` | 当前页已发布文章列表 |
| `pagination` | `Pagination` | 分页信息 |

#### `post.html`（文章详情页）

| 变量 | 类型 | 说明 |
|---|---|---|
| `post` | `Post` | 当前文章 |

#### `category.html` / `tag.html`

| 变量 | 类型 | 说明 |
|---|---|---|
| `current_category` / `current_tag` | `String` | 分类 / 标签名称 |
| `posts` | `Vec<Post>` | 该分类 / 标签下的文章列表 |
| `total_posts` | `i64` | 符合条件的文章总数 |
| `pagination` | `Pagination` | 分页信息 |

#### `search.html`（搜索结果页）

| 变量 | 类型 | 说明 |
|---|---|---|
| `query` | `String` | 搜索关键词 |
| `posts` | `Vec<Post>` | 搜索结果（含 `snippet` 字段） |
| `total_results` | `i64` | 搜索结果总数 |
| `pagination` | `Pagination` | 分页信息 |

#### `Post` 数据结构

| 字段 | 类型 | 说明 |
|---|---|---|
| `post.title` | `String` | 文章标题 |
| `post.slug` | `String` | URL 友好标识符 |
| `post.category` | `String` | 分类名称（可能为空） |
| `post.tags` | `Vec<String>` | 标签列表 |
| `post.content` | `String` | 已渲染并过滤的 HTML |
| `post.snippet` | `Option<String>` | 搜索结果摘要（HTML，已过滤，仅含 `<mark>` 高亮标签） |
| `post.created_at` | `String` | ISO 8601 创建时间 |
| `post.updated_at` | `String` | ISO 8601 更新时间 |

> **注意：** `post.content` 已渲染并过滤为安全 HTML，模板中使用 `{{ post.content | safe }}` 避免二次转义。`post.snippet` 仅包含 `<mark>` 高亮标签，可安全使用 `| safe`。

#### `Pagination` 数据结构

| 字段 | 类型 | 说明 |
|---|---|---|
| `pagination.current` | `usize` | 当前页码（从 1 开始） |
| `pagination.total` | `usize` | 总页数 |
| `pagination.prev` | `Option<usize>` | 上一页页码 |
| `pagination.next` | `Option<usize>` | 下一页页码 |

### 静态资源

`themes/{name}/static/` 下的文件通过 `/static/theme/` 对外访问：

```
themes/my-theme/static/style.css  →  /static/theme/style.css
themes/my-theme/static/logo.png   →  /static/theme/logo.png
```

在模板中使用绝对路径引用：

```html
<link rel="stylesheet" href="/static/theme/style.css">
<img src="/static/theme/logo.png" alt="Logo">
```

### 切换主题

1. 将主题文件夹放入 `themes/` 目录
2. 进入后台 `/admin/themes`
3. 点击目标主题的 **Activate** 按钮
4. 博客即时切换，无需重启

## API 参考

### 公开接口

| 方法 | 路径 | 鉴权 | 说明 |
|---|---|---|---|
| `GET` | `/api/health` | — | 健康检查 |
| `POST` | `/api/login` | — | 登录（返回 JWT Cookie；含速率限制） |
| `POST` | `/api/logout` | — | 登出（清除 Cookie） |
| `POST` | `/api/preview` | — | Markdown → 过滤后的安全 HTML |
| `GET` | `/api/search?q=` | — | 全文搜索（仅已发布文章） |

### 后台接口（需 JWT）

| 方法 | 路径 | 说明 |
|---|---|---|
| `GET` | `/api/admin/posts` | 文章列表（分页） |
| `POST` | `/api/admin/posts` | 创建文章 |
| `GET` | `/api/admin/posts/{id}` | 获取文章 |
| `PUT` | `/api/admin/posts/{id}` | 更新文章 |
| `DELETE` | `/api/admin/posts/{id}` | 删除文章 |
| `GET` | `/api/admin/settings` | 获取站点设置 |
| `PUT` | `/api/admin/settings` | 更新站点设置 |
| `GET` | `/api/admin/stats` | 分类与标签统计 |
| `GET` | `/api/admin/themes` | 列出已安装主题 |
| `POST` | `/api/admin/themes/{name}/activate` | 激活主题 |

## 技术栈

| 层级 | 技术 |
|---|---|
| 语言 | Rust (Edition 2021) |
| Web 框架 | [Axum](https://github.com/tokio-rs/axum) 0.8 |
| 数据库 | SQLite + [sqlx](https://github.com/launchbadge/sqlx) 0.8 异步驱动 |
| 模板引擎 | [Tera](https://github.com/Keats/tera) 1.x |
| Markdown 解析 | [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark) 0.12 |
| HTML 过滤 | [ammonia](https://github.com/rust-ammonia/ammonia) 4 |
| 全文搜索 | SQLite FTS5（trigram 分词器） |
| 认证 | JWT（[jsonwebtoken](https://github.com/Keats/jsonwebtoken) 9）+ HttpOnly Cookie |
| 后台 UI | 原生 JavaScript（无框架） |

## 项目结构

```
inkle/
├── Cargo.toml
├── Dockerfile
├── docker-compose.yml
├── .env.example
├── README.md
├── README_zh.md
├── migrations/
│   ├── 001_init.sql           # 文章表 & 配置表
│   ├── 002_fts.sql            # FTS5 虚拟表
│   ├── 003_unicode61.sql      # CJK 分词器支持
│   ├── 004_search_v2.sql      # Trigram 分词器 + 搜索日志
│   └── 005_post_tags.sql      # post_tags 关联表（规范化标签）
├── themes/
│   ├── default/               # "Soft Curve" — 浅色毛玻璃主题
│   └── nord/                  # "Nord" — 深色北欧极光主题
├── admin_templates/           # 后台管理 Tera 模板
│   ├── base.html
│   ├── login.html
│   ├── dashboard.html
│   ├── editor.html
│   ├── settings.html
│   ├── themes.html
│   └── 404.html
├── static/                    # 后台管理静态资源
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

## 开源协议

[MIT](LICENSE)
