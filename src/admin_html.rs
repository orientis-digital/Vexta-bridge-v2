pub static ADMIN_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Vexta V2 Bridge — Admin Console</title>
    <style>
        :root {
            --bg-dark: #080808;
            --panel-bg: #121212;
            --accent: #39FF14;
            --accent-glow: rgba(57, 255, 20, 0.3);
            --border: rgba(57, 255, 20, 0.2);
            --text: #EEEEEE;
            --text-muted: #888888;
        }
        * { box-sizing: border-box; margin: 0; padding: 0; font-family: 'Courier New', Courier, monospace; }
        body { background-color: var(--bg-dark); color: var(--text); padding: 24px; }
        .header { display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid var(--border); padding-bottom: 16px; margin-bottom: 24px; }
        .logo { display: flex; align-items: center; gap: 12px; font-weight: bold; font-size: 20px; color: #FFF; }
        .badge { background: rgba(57, 255, 20, 0.15); border: 1px solid var(--accent); color: var(--accent); font-size: 11px; padding: 3px 8px; border-radius: 4px; }
        .auth-bar { display: flex; gap: 8px; }
        input[type="password"], input[type="text"], textarea { background: #000; border: 1px solid var(--border); color: #FFF; padding: 8px 12px; border-radius: 6px; font-size: 12px; width: 100%; }
        input:focus, textarea:focus { outline: none; border-color: var(--accent); box-shadow: 0 0 10px var(--accent-glow); }
        button { background: var(--accent); color: #000; font-weight: bold; border: none; padding: 8px 16px; border-radius: 6px; cursor: pointer; font-size: 12px; transition: all 0.2s; }
        button:hover { opacity: 0.9; box-shadow: 0 0 12px var(--accent-glow); }
        .btn-danger { background: #FF3366; color: #FFF; }
        .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 16px; margin-bottom: 24px; }
        .card { background: var(--panel-bg); border: 1px solid var(--border); border-radius: 8px; padding: 16px; }
        .card-title { font-size: 11px; color: var(--text-muted); text-transform: uppercase; margin-bottom: 8px; }
        .card-value { font-size: 24px; font-weight: bold; color: var(--accent); }
        .section-title { font-size: 14px; font-weight: bold; margin-bottom: 12px; color: var(--accent); text-transform: uppercase; }
        table { width: 100%; border-collapse: collapse; margin-top: 8px; font-size: 12px; }
        th, td { border: 1px solid var(--border); padding: 8px 12px; text-align: left; }
        th { background: #1A1A1A; color: var(--accent); }
        tr:nth-child(even) { background: #0E0E0E; }
        .pubkey { font-family: monospace; font-size: 10px; color: #88CCFF; max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    </style>
</head>
<body>
    <div class="header">
        <div class="logo">
            <span>⚡ VEXTA BRIDGE V2 ADMIN</span>
            <span class="badge">SECURE CONSOLE</span>
        </div>
        <div class="auth-bar">
            <input type="password" id="adminSecret" placeholder="Enter Admin Secret Token" style="width: 240px;">
            <button onclick="fetchAdminData()">Authenticate</button>
        </div>
    </div>

    <div class="grid">
        <div class="card">
            <div class="card-title">Active WS Sessions</div>
            <div class="card-value" id="statSessions">0</div>
        </div>
        <div class="card">
            <div class="card-title">Registered Accounts</div>
            <div class="card-value" id="statUsers">0</div>
        </div>
        <div class="card">
            <div class="card-title">Queued Offline Payload</div>
            <div class="card-value" id="statQueued">0</div>
        </div>
        <div class="card">
            <div class="card-title">Registered Devices</div>
            <div class="card-value" id="statDevices">0</div>
        </div>
    </div>

    <div style="display: grid; grid-template-columns: 2fr 1fr; gap: 24px;">
        <!-- User Management Table -->
        <div class="card">
            <div class="section-title">User Accounts Management</div>
            <table>
                <thead>
                    <tr>
                        <th>Username</th>
                        <th>Ed25519 Public Key</th>
                        <th>Created Date</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody id="userTableBody">
                    <tr><td colspan="4" style="text-align: center; color: #888;">Enter Admin Secret Token to load data</td></tr>
                </tbody>
            </table>
        </div>

        <!-- Announcements & Controls -->
        <div style="display: flex; flex-direction: column; gap: 16px;">
            <div class="card">
                <div class="section-title">Broadcast System Announcement</div>
                <textarea id="announcementText" rows="3" placeholder="Type broadcast message..." style="margin-bottom: 12px;"></textarea>
                <button onclick="postAnnouncement()">Broadcast Notice</button>
            </div>

            <div class="card">
                <div class="section-title">Active Announcements</div>
                <div id="announcementsList" style="font-size: 11px; color: #CCC; display: flex; flex-direction: column; gap: 8px;">
                    <div>No announcements</div>
                </div>
            </div>
        </div>
    </div>

    <script>
        function getSecret() {
            return document.getElementById('adminSecret').value || localStorage.getItem('vexta_admin_secret') || '';
        }

        async function fetchAdminData() {
            const secret = getSecret();
            if (!secret) {
                alert('Please enter your Admin Secret Token');
                return;
            }
            localStorage.setItem('vexta_admin_secret', secret);

            try {
                // Fetch Stats
                const resStats = await fetch('/api/admin/stats', { headers: { 'X-Admin-Secret': secret } });
                if (resStats.status === 401) { alert('Unauthorized: Invalid Admin Secret Token'); return; }
                const stats = await resStats.json();
                document.getElementById('statSessions').innerText = stats.active_ws_sessions || 0;
                document.getElementById('statUsers').innerText = stats.total_users || 0;
                document.getElementById('statQueued').innerText = stats.total_queued_offline_messages || 0;
                document.getElementById('statDevices').innerText = stats.total_registered_devices || 0;

                // Fetch Users
                const resUsers = await fetch('/api/admin/users', { headers: { 'X-Admin-Secret': secret } });
                const users = await resUsers.json();
                const tbody = document.getElementById('userTableBody');
                tbody.innerHTML = '';
                users.forEach(u => {
                    const tr = document.createElement('tr');
                    const createdStr = new Date(u.created_at * 1000).toLocaleString();
                    tr.innerHTML = `
                        <td style="font-weight: bold; color: #39FF14;">${u.username}</td>
                        <td class="pubkey" title="${u.ed25519_pubkey}">${u.ed25519_pubkey}</td>
                        <td>${createdStr}</td>
                        <td><button class="btn-danger" onclick="deleteUser('${u.username}')">Delete</button></td>
                    `;
                    tbody.appendChild(tr);
                });

                // Fetch Announcements
                fetchAnnouncements();
            } catch (err) {
                console.error(err);
            }
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
            fetchAnnouncements();
        }

        async function fetchAnnouncements() {
            const secret = getSecret();
            const res = await fetch('/api/admin/announcements', { headers: { 'X-Admin-Secret': secret } });
            const list = await res.json();
            const div = document.getElementById('announcementsList');
            div.innerHTML = '';
            list.forEach(a => {
                const item = document.createElement('div');
                item.style.padding = '8px';
                item.style.background = '#000';
                item.style.borderRadius = '4px';
                item.style.border = '1px solid #333';
                item.innerHTML = `<strong>#${a.id}</strong>: ${a.message} <button class="btn-danger" style="padding: 2px 6px; font-size: 9px; float: right;" onclick="deleteAnnouncement(${a.id})">Delete</button>`;
                div.appendChild(item);
            });
        }

        async function deleteUser(username) {
            if (!confirm(`Delete user '${username}' permanently?`)) return;
            const secret = getSecret();
            await fetch(`/api/admin/users/${username}`, {
                method: 'DELETE',
                headers: { 'X-Admin-Secret': secret }
            });
            fetchAdminData();
        }

        async function deleteAnnouncement(id) {
            const secret = getSecret();
            await fetch(`/api/admin/announcements/${id}`, {
                method: 'DELETE',
                headers: { 'X-Admin-Secret': secret }
            });
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
