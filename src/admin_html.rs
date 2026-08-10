pub static ADMIN_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Vexta V2 Bridge — Admin Console</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500;600&display=swap" rel="stylesheet">
    <style>
        :root {
            --bg-dark: #07090e;
            --bg-card: rgba(15, 20, 30, 0.75);
            --bg-input: #0b0f17;
            --accent: #39ff14;
            --accent-glow: rgba(57, 255, 20, 0.25);
            --border: rgba(255, 255, 255, 0.08);
            --border-accent: rgba(57, 255, 20, 0.3);
            --text-primary: #f0f4f8;
            --text-muted: #8a99ad;
            --danger: #ff3366;
            --danger-glow: rgba(255, 51, 102, 0.25);
        }
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            background-color: var(--bg-dark);
            color: var(--text-primary);
            font-family: 'Inter', system-ui, -apple-system, sans-serif;
            padding: 28px;
            min-height: 100vh;
            background-image: 
                radial-gradient(circle at 15% 15%, rgba(57, 255, 20, 0.04) 0%, transparent 45%),
                radial-gradient(circle at 85% 85%, rgba(0, 180, 216, 0.04) 0%, transparent 45%);
        }
        .header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            border-bottom: 1px solid var(--border);
            padding-bottom: 20px;
            margin-bottom: 28px;
            gap: 16px;
            flex-wrap: wrap;
        }
        .logo-group { display: flex; align-items: center; gap: 14px; }
        .logo-title {
            font-size: 22px;
            font-weight: 800;
            letter-spacing: -0.5px;
            color: #fff;
            display: flex;
            align-items: center;
            gap: 10px;
        }
        .logo-title span { color: var(--accent); }
        .badge {
            background: rgba(57, 255, 20, 0.12);
            border: 1px solid var(--border-accent);
            color: var(--accent);
            font-size: 11px;
            font-weight: 700;
            padding: 4px 10px;
            border-radius: 20px;
            letter-spacing: 0.5px;
            text-transform: uppercase;
            display: inline-flex;
            align-items: center;
            gap: 6px;
        }
        .badge-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 8px var(--accent); }
        .auth-bar { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
        input[type="password"], input[type="text"], textarea {
            background: var(--bg-input);
            border: 1px solid var(--border);
            color: var(--text-primary);
            padding: 10px 14px;
            border-radius: 10px;
            font-size: 13px;
            font-family: inherit;
            transition: all 0.2s ease;
        }
        input:focus, textarea:focus {
            outline: none;
            border-color: var(--accent);
            box-shadow: 0 0 14px var(--accent-glow);
        }
        button {
            background: var(--accent);
            color: #050a02;
            font-weight: 700;
            font-size: 13px;
            border: none;
            padding: 10px 18px;
            border-radius: 10px;
            cursor: pointer;
            transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
            display: inline-flex;
            align-items: center;
            gap: 8px;
        }
        button:hover {
            transform: translateY(-1px);
            box-shadow: 0 4px 20px var(--accent-glow);
        }
        button:active { transform: translateY(0); }
        .btn-danger {
            background: rgba(255, 51, 102, 0.15);
            border: 1px solid rgba(255, 51, 102, 0.3);
            color: #ff527b;
            padding: 6px 12px;
            font-size: 12px;
        }
        .btn-danger:hover {
            background: var(--danger);
            color: #fff;
            box-shadow: 0 4px 16px var(--danger-glow);
        }
        .btn-secondary {
            background: rgba(255, 255, 255, 0.06);
            border: 1px solid var(--border);
            color: var(--text-primary);
        }
        .btn-secondary:hover { background: rgba(255, 255, 255, 0.1); }
        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
            gap: 18px;
            margin-bottom: 28px;
        }
        .stat-card {
            background: var(--bg-card);
            border: 1px solid var(--border);
            border-radius: 16px;
            padding: 20px;
            backdrop-filter: blur(12px);
            transition: all 0.2s ease;
        }
        .stat-card:hover {
            border-color: var(--border-accent);
            transform: translateY(-2px);
        }
        .stat-title {
            font-size: 12px;
            font-weight: 600;
            color: var(--text-muted);
            text-transform: uppercase;
            letter-spacing: 0.5px;
            margin-bottom: 8px;
        }
        .stat-value {
            font-size: 32px;
            font-weight: 800;
            color: var(--accent);
            letter-spacing: -1px;
            font-family: 'JetBrains Mono', monospace;
        }
        .main-layout {
            display: grid;
            grid-template-columns: 2fr 1fr;
            gap: 24px;
        }
        @media (max-width: 960px) {
            .main-layout { grid-template-columns: 1fr; }
        }
        .panel {
            background: var(--bg-card);
            border: 1px solid var(--border);
            border-radius: 18px;
            padding: 24px;
            backdrop-filter: blur(12px);
        }
        .panel-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            margin-bottom: 18px;
            gap: 12px;
        }
        .panel-title {
            font-size: 15px;
            font-weight: 700;
            color: #fff;
            display: flex;
            align-items: center;
            gap: 10px;
        }
        .panel-title span { color: var(--accent); }
        table {
            width: 100%;
            border-collapse: collapse;
            font-size: 13px;
        }
        th, td {
            padding: 12px 14px;
            text-align: left;
            border-bottom: 1px solid rgba(255, 255, 255, 0.05);
        }
        th {
            background: rgba(255, 255, 255, 0.03);
            color: var(--text-muted);
            font-weight: 600;
            font-size: 11px;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }
        tr:hover td { background: rgba(255, 255, 255, 0.02); }
        .user-cell { font-weight: 700; color: var(--accent); font-family: 'JetBrains Mono', monospace; }
        .pubkey-cell {
            font-family: 'JetBrains Mono', monospace;
            font-size: 11px;
            color: #88ccff;
            max-width: 220px;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
            cursor: pointer;
        }
        .pubkey-cell:hover { text-decoration: underline; color: #b3e0ff; }
        .announcement-item {
            background: var(--bg-input);
            border: 1px solid var(--border);
            border-radius: 12px;
            padding: 14px;
            display: flex;
            flex-direction: column;
            gap: 8px;
            transition: all 0.2s ease;
        }
        .announcement-item:hover { border-color: rgba(255, 255, 255, 0.15); }
        .announcement-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            font-size: 11px;
            color: var(--text-muted);
        }
        .announcement-text { font-size: 13px; color: var(--text-primary); line-height: 1.4; }
        .template-chip {
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid var(--border);
            color: var(--text-muted);
            font-size: 11px;
            padding: 4px 8px;
            border-radius: 6px;
            cursor: pointer;
            transition: all 0.2s ease;
        }
        .template-chip:hover {
            border-color: var(--accent);
            color: var(--accent);
            background: rgba(57, 255, 20, 0.08);
        }
        .toast {
            position: fixed;
            bottom: 24px;
            right: 24px;
            background: #121924;
            border: 1px solid var(--accent);
            color: #fff;
            padding: 12px 20px;
            border-radius: 12px;
            font-size: 13px;
            font-weight: 500;
            box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
            transform: translateY(100px);
            opacity: 0;
            transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
            z-index: 1000;
        }
        .toast.show { transform: translateY(0); opacity: 1; }
    </style>
</head>
<body>
    <div class="header">
        <div class="logo-group">
            <div class="logo-title">⚡ <span>VEXTA</span> BRIDGE V2</div>
            <div class="badge"><span class="badge-dot"></span> SECURE RELAY CONSOLE</div>
        </div>
        <div class="auth-bar">
            <input type="password" id="adminSecret" placeholder="Enter Admin Secret Token" style="width: 240px;">
            <button onclick="fetchAdminData()">Authenticate</button>
            <button class="btn-secondary" onclick="toggleAutoRefresh()" id="btnAutoRefresh">Auto-Refresh: Off</button>
        </div>
    </div>

    <div class="stats-grid">
        <div class="stat-card">
            <div class="stat-title">Active WS Sessions</div>
            <div class="stat-value" id="statSessions">0</div>
        </div>
        <div class="stat-card">
            <div class="stat-title">Registered Accounts</div>
            <div class="stat-value" id="statUsers">0</div>
        </div>
        <div class="stat-card">
            <div class="stat-title">Queued Offline Payload</div>
            <div class="stat-value" id="statQueued">0</div>
        </div>
        <div class="stat-card">
            <div class="stat-title">Registered Devices</div>
            <div class="stat-value" id="statDevices">0</div>
        </div>
    </div>

    <div class="main-layout">
        <!-- User Accounts Management Table -->
        <div class="panel">
            <div class="panel-header">
                <div class="panel-title"><span>👥</span> User Accounts</div>
                <input type="text" id="userSearch" placeholder="Filter users..." oninput="filterUsers()" style="width: 180px; padding: 6px 12px; font-size: 12px;">
            </div>
            <table>
                <thead>
                    <tr>
                        <th>Username</th>
                        <th>Ed25519 Public Key</th>
                        <th>Registered Date</th>
                        <th style="text-align: right;">Action</th>
                    </tr>
                </thead>
                <tbody id="userTableBody">
                    <tr><td colspan="4" style="text-align: center; color: var(--text-muted); padding: 24px;">Enter Admin Secret Token to load real-time database records.</td></tr>
                </tbody>
            </table>
        </div>

        <!-- System Announcements Panel -->
        <div style="display: flex; flex-direction: column; gap: 20px;">
            <div class="panel">
                <div class="panel-title" style="margin-bottom: 14px;"><span>📢</span> Broadcast System Announcement</div>
                <div style="display: flex; gap: 6px; margin-bottom: 10px; flex-wrap: wrap;">
                    <span class="template-chip" onclick="applyTemplate('[RELEASE] Vexta Client v0.0.9 is live! Please update.')">Release Notice</span>
                    <span class="template-chip" onclick="applyTemplate('[MAINTENANCE] Scheduled bridge maintenance in 15 mins.')">Maintenance</span>
                    <span class="template-chip" onclick="applyTemplate('[SECURITY] Key rotation notice: verify server public key.')">Security Alert</span>
                </div>
                <textarea id="announcementText" rows="3" placeholder="Type system-wide broadcast message..." style="width: 100%; margin-bottom: 12px;"></textarea>
                <button onclick="postAnnouncement()" style="width: 100%; justify-content: center;">Broadcast Notice</button>
            </div>

            <div class="panel">
                <div class="panel-header">
                    <div class="panel-title"><span>📋</span> Active Announcements</div>
                </div>
                <div id="announcementsList" style="display: flex; flex-direction: column; gap: 10px;">
                    <div style="color: var(--text-muted); font-size: 12px;">No active announcements</div>
                </div>
            </div>
        </div>
    </div>

    <div id="toast" class="toast">Action performed successfully</div>

    <script>
        let rawUsers = [];
        let autoRefreshInterval = null;

        function showToast(msg) {
            const toast = document.getElementById('toast');
            toast.innerText = msg;
            toast.classList.add('show');
            setTimeout(() => toast.classList.remove('show'), 3000);
        }

        function getSecret() {
            return document.getElementById('adminSecret').value || localStorage.getItem('vexta_admin_secret') || '';
        }

        function toggleAutoRefresh() {
            const btn = document.getElementById('btnAutoRefresh');
            if (autoRefreshInterval) {
                clearInterval(autoRefreshInterval);
                autoRefreshInterval = null;
                btn.innerText = 'Auto-Refresh: Off';
                btn.classList.remove('btn-secondary');
                showToast('Auto-refresh disabled');
            } else {
                autoRefreshInterval = setInterval(fetchAdminData, 5000);
                btn.innerText = 'Auto-Refresh: 5s';
                showToast('Auto-refresh set to every 5s');
            }
        }

        function applyTemplate(text) {
            document.getElementById('announcementText').value = text;
        }

        function copyToClipboard(text) {
            navigator.clipboard.writeText(text);
            showToast('Copied to clipboard!');
        }

        async function fetchAdminData() {
            const secret = getSecret();
            if (!secret) {
                showToast('Please enter your Admin Secret Token');
                return;
            }
            localStorage.setItem('vexta_admin_secret', secret);

            try {
                // Fetch Stats
                const resStats = await fetch('/api/admin/stats', { headers: { 'X-Admin-Secret': secret } });
                if (resStats.status === 401) { showToast('Unauthorized: Invalid Admin Secret Token'); return; }
                const stats = await resStats.json();
                document.getElementById('statSessions').innerText = stats.active_ws_sessions || 0;
                document.getElementById('statUsers').innerText = stats.total_users || 0;
                document.getElementById('statQueued').innerText = stats.total_queued_offline_messages || 0;
                document.getElementById('statDevices').innerText = stats.total_registered_devices || 0;

                // Fetch Users
                const resUsers = await fetch('/api/admin/users', { headers: { 'X-Admin-Secret': secret } });
                rawUsers = await resUsers.json();
                renderUsers(rawUsers);

                // Fetch Announcements
                fetchAnnouncements();
            } catch (err) {
                console.error(err);
            }
        }

        function renderUsers(users) {
            const tbody = document.getElementById('userTableBody');
            tbody.innerHTML = '';
            if (users.length === 0) {
                tbody.innerHTML = '<tr><td colspan="4" style="text-align: center; color: var(--text-muted); padding: 18px;">No users found</td></tr>';
                return;
            }
            users.forEach(u => {
                const tr = document.createElement('tr');
                const createdStr = new Date(u.created_at * 1000).toLocaleString();
                tr.innerHTML = `
                    <td class="user-cell">@${u.username}</td>
                    <td class="pubkey-cell" title="Click to copy public key" onclick="copyToClipboard('${u.ed25519_pubkey}')">${u.ed25519_pubkey}</td>
                    <td style="color: var(--text-muted); font-size: 12px;">${createdStr}</td>
                    <td style="text-align: right;"><button class="btn-danger" onclick="deleteUser('${u.username}')">Delete</button></td>
                `;
                tbody.appendChild(tr);
            });
        }

        function filterUsers() {
            const q = document.getElementById('userSearch').value.toLowerCase().trim();
            const filtered = rawUsers.filter(u => u.username.toLowerCase().includes(q) || u.ed25519_pubkey.toLowerCase().includes(q));
            renderUsers(filtered);
        }

        async function postAnnouncement() {
            const secret = getSecret();
            const message = document.getElementById('announcementText').value;
            if (!message.trim()) return;

            await fetch('/api/admin/announcements', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json', 'X-Admin-Secret': secret },
                body: JSON.stringify({ message })
            });
            document.getElementById('announcementText').value = '';
            showToast('Announcement broadcast successfully');
            fetchAnnouncements();
        }

        async function fetchAnnouncements() {
            const secret = getSecret();
            const res = await fetch('/api/admin/announcements', { headers: { 'X-Admin-Secret': secret } });
            const list = await res.json();
            const div = document.getElementById('announcementsList');
            div.innerHTML = '';
            if (list.length === 0) {
                div.innerHTML = '<div style="color: var(--text-muted); font-size: 12px; padding: 10px;">No active announcements</div>';
                return;
            }
            list.forEach(a => {
                const item = document.createElement('div');
                item.className = 'announcement-item';
                item.innerHTML = `
                    <div class="announcement-header">
                        <span>#${a.id} • ${a.created_at || 'Just now'}</span>
                        <button class="btn-danger" style="padding: 3px 8px; font-size: 10px;" onclick="deleteAnnouncement(${a.id})">Delete</button>
                    </div>
                    <div class="announcement-text">${a.message}</div>
                `;
                div.appendChild(item);
            });
        }

        async function deleteUser(username) {
            if (!confirm(`Delete user '@${username}' permanently?`)) return;
            const secret = getSecret();
            await fetch(`/api/admin/users/${username}`, {
                method: 'DELETE',
                headers: { 'X-Admin-Secret': secret }
            });
            showToast(`User @${username} deleted`);
            fetchAdminData();
        }

        async function deleteAnnouncement(id) {
            const secret = getSecret();
            await fetch(`/api/admin/announcements/${id}`, {
                method: 'DELETE',
                headers: { 'X-Admin-Secret': secret }
            });
            showToast(`Announcement #${id} deleted`);
            fetchAnnouncements();
        }

        window.onload = () => {
            const saved = localStorage.getItem('vexta_admin_secret');
            if (saved) {
                document.getElementById('adminSecret').value = saved;
                fetchAdminData();
            }
        };
    </script>
</body>
</html>
"#;
