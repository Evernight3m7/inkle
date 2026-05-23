// ── Toast System ──

function showToast(message, type, duration) {
    type = type || 'success';
    duration = duration || 3000;

    var container = document.getElementById('toast-container');
    if (!container) return;

    var toast = document.createElement('div');
    toast.className = 'toast toast-' + type;
    toast.textContent = message;
    toast.setAttribute('role', 'alert');

    // Click to dismiss immediately
    toast.onclick = function () { dismissToast(toast); };

    container.appendChild(toast);

    // Auto-dismiss after duration
    var timer = setTimeout(function () { dismissToast(toast); }, duration);
    toast._dismissTimer = timer;
}

function dismissToast(toast) {
    if (toast._dismissed) return;
    toast._dismissed = true;
    clearTimeout(toast._dismissTimer);
    toast.style.opacity = '0';
    toast.style.transform = 'translateX(100%)';
    toast.style.transition = 'all 0.2s ease';
    setTimeout(function () {
        if (toast.parentNode) toast.parentNode.removeChild(toast);
    }, 200);
}

// ── Modal System ──

var _modalCallback = null;

function confirmModal(message, onConfirm) {
    var overlay = document.getElementById('modal-overlay');
    var msg = document.getElementById('modal-message');
    var btn = document.getElementById('modal-confirm-btn');

    if (!overlay || !msg || !btn) return;

    msg.textContent = message;
    _modalCallback = onConfirm;
    btn.onclick = function () {
        closeModal();
        if (_modalCallback) _modalCallback();
    };
    overlay.style.display = 'flex';
    // Focus confirm button for keyboard accessibility
    btn.focus();
}

function closeModal() {
    var overlay = document.getElementById('modal-overlay');
    if (overlay) overlay.style.display = 'none';
    _modalCallback = null;
}

document.addEventListener('DOMContentLoaded', function () {
    var overlay = document.getElementById('modal-overlay');
    if (overlay) {
        overlay.addEventListener('click', function (e) {
            if (e.target === overlay) closeModal();
        });
    }
});

// Escape key closes modal
document.addEventListener('keydown', function (e) {
    if (e.key === 'Escape') {
        var overlay = document.getElementById('modal-overlay');
        if (overlay && overlay.style.display === 'flex') {
            closeModal();
        }
    }
});

// ── Auth ──

async function handleLogin(event) {
    event.preventDefault();
    var password = document.getElementById('password').value;

    try {
        var res = await fetch('/api/login', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ password: password })
        });

        if (res.ok) {
            window.location.href = '/admin';
        } else {
            var data = await res.json().catch(function () { return { error: 'Login failed' }; });
            showToast(data.error || 'Login failed', 'error');
        }
    } catch (err) {
        showToast('Network error', 'error');
    }
    return false;
}

async function logout() {
    await fetch('/api/logout', { method: 'POST' });
    window.location.href = '/admin/login';
}

// ── Post CRUD ──

async function savePost() {
    var title = document.getElementById('post-title').value.trim();
    if (!title) {
        showToast('Title is required', 'error');
        return;
    }

    var form = document.getElementById('post-form');
    var postId = form.dataset.postId;
    var tagsStr = document.getElementById('post-tags').value;
    var tags = tagsStr ? tagsStr.split(',').map(function (t) { return t.trim(); }).filter(Boolean) : [];

    var payload = {
        title: title,
        slug: document.getElementById('post-slug').value.trim() || null,
        category: document.getElementById('post-category').value.trim() || null,
        tags: tags.length > 0 ? tags : null,
        content: document.getElementById('post-content').value,
        status: document.getElementById('post-status').value
    };

    try {
        var url = postId ? '/api/admin/posts/' + postId : '/api/admin/posts';
        var method = postId ? 'PUT' : 'POST';
        var res = await fetch(url, {
            method: method,
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(payload)
        });

        if (res.ok) {
            var data = await res.json();
            showToast(postId ? 'Post updated' : 'Post created', 'success');
            if (!postId) {
                window.location.href = '/admin/posts/' + data.post.id + '/edit';
            } else {
                // Update data-post-id in case slug changed
                form.dataset.postId = data.post.id;
            }
        } else {
            var data = await res.json().catch(function () { return { error: 'Failed to save post' }; });
            showToast(data.error || 'Failed to save post', 'error');
        }
    } catch (err) {
        showToast('Network error', 'error');
    }
}

document.addEventListener('DOMContentLoaded', function () {
    document.addEventListener('click', function (event) {
        var btn = event.target.closest('.btn-delete-post');
        if (!btn) return;
        event.preventDefault();
        var id = btn.dataset.postId;
        var title = btn.dataset.postTitle;
        if (id && title) confirmDelete(id, title);
    });
});

function confirmDelete(id, title) {
    confirmModal('Delete "' + title + '"? This action cannot be undone.', async function () {
        try {
            var res = await fetch('/api/admin/posts/' + id, { method: 'DELETE' });
            if (res.ok) {
                showToast('Post deleted', 'success');
                setTimeout(function () { window.location.reload(); }, 500);
            } else {
                var data = await res.json().catch(function () { return { error: 'Failed to delete' }; });
                showToast(data.error || 'Failed to delete', 'error');
            }
        } catch (err) {
            showToast('Network error', 'error');
        }
    });
}

// ── Editor Dual Mode ──

function switchEditorMode(mode) {
    var rawPane = document.getElementById('editor-raw');
    var previewPane = document.getElementById('editor-preview');
    var btnRaw = document.getElementById('btn-raw');
    var btnPreview = document.getElementById('btn-preview');

    if (!rawPane || !previewPane) return;

    if (mode === 'preview') {
        loadPreview();
        rawPane.style.display = 'none';
        previewPane.style.display = 'block';
        btnRaw.className = 'btn btn-sm btn-secondary';
        btnPreview.className = 'btn btn-sm btn-primary';
    } else {
        rawPane.style.display = 'block';
        previewPane.style.display = 'none';
        btnRaw.className = 'btn btn-sm btn-primary';
        btnPreview.className = 'btn btn-sm btn-secondary';
    }
}

async function loadPreview() {
    var content = document.getElementById('post-content').value;
    var previewEl = document.getElementById('preview-content');
    if (!previewEl) return;

    try {
        var res = await fetch('/api/preview', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ markdown: content })
        });
        if (res.ok) {
            var data = await res.json();
            previewEl.innerHTML = data.html;
        } else {
            previewEl.innerHTML = '<p style="color:var(--danger)">Failed to render preview</p>';
        }
    } catch (err) {
        previewEl.innerHTML = '<p style="color:var(--danger)">Network error</p>';
    }
}

// ── Auto-slugify ──

var slugTimer = null;
document.addEventListener('DOMContentLoaded', function () {
    var titleInput = document.getElementById('post-title');
    var slugInput = document.getElementById('post-slug');
    if (!titleInput || !slugInput) return;

    var userEditedSlug = !!slugInput.value;

    titleInput.addEventListener('input', function () {
        if (userEditedSlug) return;
        clearTimeout(slugTimer);
        slugTimer = setTimeout(function () {
            var slug = titleInput.value
                .toLowerCase()
                .replace(/[^a-z0-9一-鿿]+/g, '-')
                .replace(/^-|-$/g, '')
                .substring(0, 200);
            slugInput.value = slug;
        }, 300);
    });

    slugInput.addEventListener('input', function () {
        userEditedSlug = true;
    });
});

// ── Themes ──

async function activateTheme(name) {
    try {
        var res = await fetch('/api/admin/themes/' + encodeURIComponent(name) + '/activate', { method: 'POST' });
        if (res.ok) {
            showToast('Theme "' + name + '" activated', 'success');
            setTimeout(function () { window.location.reload(); }, 500);
        } else {
            var data = await res.json().catch(function () { return { error: 'Failed to activate theme' }; });
            showToast(data.error || 'Failed to activate theme', 'error');
        }
    } catch (err) {
        showToast('Network error', 'error');
    }
}

// ── Settings ──

async function saveSettings() {
    var socialLinks = {};
    document.querySelectorAll('.social-link-row').forEach(function (row) {
        var key = row.querySelector('.social-key').value.trim();
        var value = row.querySelector('.social-value').value.trim();
        if (key && value) socialLinks[key] = value;
    });

    var payload = {
        site_title: document.getElementById('site-title').value,
        site_subtitle: document.getElementById('site-subtitle').value,
        posts_per_page: parseInt(document.getElementById('posts-per-page').value) || 10,
        social_links: socialLinks
    };

    try {
        var res = await fetch('/api/admin/settings', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(payload)
        });
        if (res.ok) {
            showToast('Settings saved', 'success');
        } else {
            var data = await res.json().catch(function () { return { error: 'Failed to save settings' }; });
            showToast(data.error || 'Failed to save settings', 'error');
        }
    } catch (err) {
        showToast('Network error', 'error');
    }
}

function addSocialLink() {
    var container = document.getElementById('social-links-container');
    if (!container) return;
    var row = document.createElement('div');
    row.className = 'social-link-row';
    row.innerHTML =
        '<input type="text" class="input social-key" placeholder="Platform"> ' +
        '<input type="text" class="input social-value" placeholder="URL"> ' +
        '<button type="button" class="btn btn-sm btn-danger" onclick="this.parentElement.remove()">&times;</button>';
    container.appendChild(row);
}

// ── Dashboard Search (AJAX dropdown) ──

var searchTimer = null;
document.addEventListener('DOMContentLoaded', function () {
    var searchInput = document.getElementById('search-input');
    var dropdown = document.getElementById('search-dropdown');
    if (!searchInput || !dropdown) return;

    searchInput.addEventListener('input', function () {
        clearTimeout(searchTimer);
        var q = this.value.trim();
        var that = this;
        searchTimer = setTimeout(async function () {
            if (q.length > 0) {
                try {
                    var res = await fetch('/api/search?q=' + encodeURIComponent(q));
                    if (res.ok) {
                        var data = await res.json();
                        renderSearchDropdown(dropdown, data.results, q);
                    }
                } catch (e) {
                    dropdown.style.display = 'none';
                }
            } else {
                dropdown.style.display = 'none';
            }
        }, 250);
    });

    // Hide dropdown when clicking outside
    document.addEventListener('click', function (e) {
        if (!searchInput.contains(e.target) && !dropdown.contains(e.target)) {
            dropdown.style.display = 'none';
        }
    });

    // Show dropdown again if input still has value on focus
    searchInput.addEventListener('focus', function () {
        var q = this.value.trim();
        if (q.length > 0 && dropdown.children.length > 0) {
            dropdown.style.display = 'block';
        }
    });

    // Keyboard navigation
    searchInput.addEventListener('keydown', function (e) {
        var items = dropdown.querySelectorAll('.search-dropdown-item');
        var active = dropdown.querySelector('.search-dropdown-item.active');
        var idx = Array.from(items).indexOf(active);

        if (e.key === 'ArrowDown') {
            e.preventDefault();
            var next = Math.min(idx + 1, items.length - 1);
            items.forEach(function (it) { it.classList.remove('active'); });
            if (items[next]) items[next].classList.add('active');
        } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            var prev = Math.max(idx - 1, 0);
            items.forEach(function (it) { it.classList.remove('active'); });
            if (items[prev]) items[prev].classList.add('active');
        } else if (e.key === 'Enter') {
            e.preventDefault();
            if (active) active.click();
        } else if (e.key === 'Escape') {
            dropdown.style.display = 'none';
            searchInput.blur();
        }
    });
});

function renderSearchDropdown(dropdown, results, query) {
    dropdown.innerHTML = '';

    if (results.length === 0) {
        dropdown.innerHTML = '<div class="search-dropdown-empty">No posts matching "' + escapeHtml(query) + '"</div>';
    } else {
        results.forEach(function (r) {
            var item = document.createElement('a');
            item.className = 'search-dropdown-item';
            item.href = '/admin/posts/' + r.id + '/edit';
            item.innerHTML =
                '<span class="search-dropdown-title">' + highlightMatch(r.title, query) + '</span>' +
                (r.snippet ? '<span class="search-dropdown-snippet">' + r.snippet + '</span>' : '');
            item.addEventListener('click', function (e) {
                // Allow normal navigation but hide dropdown
                dropdown.style.display = 'none';
            });
            dropdown.appendChild(item);
        });
        // Highlight first item for keyboard nav
        if (dropdown.firstChild) {
            dropdown.firstChild.classList.add('active');
        }
    }

    dropdown.style.display = 'block';
}

function highlightMatch(text, query) {
    // HTML-escape first, then wrap matches in <mark> tags
    var escapedText = escapeHtml(text);
    var escapedQuery = query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    var re = new RegExp('(' + escapedQuery + ')', 'gi');
    return escapedText.replace(re, '<mark>$1</mark>');
}

function escapeHtml(str) {
    return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

// ── Category Datalist Population ──

document.addEventListener('DOMContentLoaded', async function () {
    var datalist = document.getElementById('category-list');
    if (!datalist) return;
    try {
        var res = await fetch('/api/admin/stats');
        if (res.ok) {
            var data = await res.json();
            data.categories.forEach(function (cat) {
                var option = document.createElement('option');
                option.value = cat.name;
                datalist.appendChild(option);
            });
        }
    } catch (e) {
        // Silent fail — category autocomplete is optional
    }
});
