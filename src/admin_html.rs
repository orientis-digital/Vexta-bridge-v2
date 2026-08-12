pub static ADMIN_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Vexta Bridge V2 - v0.0.1 — Admin Console</title>
    <meta name="description" content="Vexta Bridge V2 secure relay administration console. Monitor sessions, manage users, broadcast announcements.">
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=IBM+Plex+Sans:wght@300;400;500;600;700&family=IBM+Plex+Mono:wght@400;500;600&display=swap" rel="stylesheet">
    <style>
        :root {
            --surface:       #09090b;
            --surface-2:     #111116;
            --surface-3:     #18181f;
            --panel:         rgba(18, 18, 28, 0.82);
            --panel-border:  rgba(255, 255, 255, 0.06);
            --panel-hover:   rgba(255, 255, 255, 0.03);
            --accent:        #39ff14;
            --accent-dim:    rgba(57, 255, 20, 0.15);
            --accent-glow:   rgba(57, 255, 20, 0.22);
            --accent-border: rgba(57, 255, 20, 0.28);
            --blue:          #3b82f6;
            --blue-dim:      rgba(59, 130, 246, 0.15);
            --purple:        #a855f7;
            --purple-dim:    rgba(168, 85, 247, 0.15);
            --amber:         #f59e0b;
            --amber-dim:     rgba(245, 158, 11, 0.15);
            --danger:        #ef4444;
            --danger-dim:    rgba(239, 68, 68, 0.12);
            --danger-glow:   rgba(239, 68, 68, 0.22);
            --success:       #10b981;
            --success-dim:   rgba(16, 185, 129, 0.12);
            --text-1:        #fafafa;
            --text-2:        #a1a1aa;
            --text-3:        #71717a;
            --radius-sm:     8px;
            --radius-md:     12px;
            --radius-lg:     18px;
            --radius-xl:     24px;
        }

        *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

        body {
            background-color: var(--surface);
            color: var(--text-1);
            font-family: 'IBM Plex Sans', system-ui, sans-serif;
            font-size: 14px;
            line-height: 1.5;
            min-height: 100vh;
            background-image:
                radial-gradient(ellipse 60% 40% at 10% 0%, rgba(57,255,20,0.05) 0%, transparent 50%),
                radial-gradient(ellipse 50% 50% at 90% 100%, rgba(59,130,246,0.04) 0%, transparent 50%);
        }

        /* ───── Layout ───── */
        .shell { display: flex; flex-direction: column; min-height: 100vh; }

        /* ───── Top Bar ───── */
        .topbar {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 14px 28px;
            border-bottom: 1px solid var(--panel-border);
            background: rgba(9,9,11,0.85);
            backdrop-filter: blur(16px);
            position: sticky;
            top: 0;
            z-index: 100;
            gap: 16px;
            flex-wrap: wrap;
        }

        .brand {
            display: flex;
            align-items: center;
            gap: 12px;
        }

        .brand-icon {
            width: 36px;
            height: 36px;
            background: linear-gradient(135deg, var(--accent) 0%, #00d4aa 100%);
            border-radius: 10px;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 18px;
            flex-shrink: 0;
            box-shadow: 0 0 20px var(--accent-glow);
        }

        .brand-name {
            font-size: 16px;
            font-weight: 700;
            letter-spacing: -0.3px;
            color: var(--text-1);
        }

        .brand-name span { color: var(--accent); }

        .brand-sub {
            font-size: 11px;
            color: var(--text-3);
            font-family: 'IBM Plex Mono', monospace;
            letter-spacing: 0.3px;
        }

        .status-badge {
            display: inline-flex;
            align-items: center;
            gap: 6px;
            background: var(--success-dim);
            border: 1px solid rgba(16, 185, 129, 0.25);
            color: var(--success);
            font-size: 11px;
            font-weight: 600;
            padding: 4px 10px;
            border-radius: 20px;
            letter-spacing: 0.4px;
            text-transform: uppercase;
        }

        .status-dot {
            width: 6px;
            height: 6px;
            border-radius: 50%;
            background: var(--success);
            box-shadow: 0 0 8px var(--success);
            animation: pulse 2s infinite;
        }

        @keyframes pulse {
            0%, 100% { opacity: 1; }
            50% { opacity: 0.4; }
        }

        .auth-section {
            display: flex;
            align-items: center;
            gap: 10px;
            flex-wrap: wrap;
        }

        /* ───── Inputs ───── */
        input[type="password"],
        input[type="text"],
        textarea,
        select {
            background: var(--surface-3);
            border: 1px solid var(--panel-border);
            color: var(--text-1);
            padding: 9px 13px;
            border-radius: var(--radius-sm);
            font-size: 13px;
            font-family: inherit;
            transition: border-color 0.2s, box-shadow 0.2s;
            outline: none;
        }

        input:focus, textarea:focus, select:focus {
            border-color: var(--accent-border);
            box-shadow: 0 0 0 3px var(--accent-dim);
        }

        select { cursor: pointer; }

        /* ───── Buttons ───── */
        button, .btn {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            gap: 7px;
            font-family: inherit;
            font-size: 13px;
            font-weight: 600;
            padding: 9px 16px;
            border-radius: var(--radius-sm);
            border: none;
            cursor: pointer;
            transition: all 0.18s cubic-bezier(0.16, 1, 0.3, 1);
            white-space: nowrap;
        }

        .btn-primary {
            background: var(--accent);
            color: #030d00;
        }
        .btn-primary:hover {
            transform: translateY(-1px);
            box-shadow: 0 6px 24px var(--accent-glow);
            filter: brightness(1.08);
        }
        .btn-primary:active { transform: translateY(0); }

        .btn-secondary {
            background: var(--surface-3);
            border: 1px solid var(--panel-border);
            color: var(--text-2);
        }
        .btn-secondary:hover {
            background: rgba(255,255,255,0.07);
            color: var(--text-1);
        }

        .btn-danger {
            background: var(--danger-dim);
            border: 1px solid rgba(239,68,68,0.22);
            color: #f87171;
            padding: 6px 12px;
            font-size: 12px;
        }
        .btn-danger:hover {
            background: var(--danger);
            color: #fff;
            box-shadow: 0 4px 16px var(--danger-glow);
        }

        .btn-ghost {
            background: transparent;
            border: 1px solid var(--panel-border);
            color: var(--text-2);
            padding: 7px 13px;
        }
        .btn-ghost:hover { background: var(--panel-hover); color: var(--text-1); }

        .btn-icon {
            padding: 7px;
            border-radius: var(--radius-sm);
        }

        button:disabled, .btn:disabled {
            opacity: 0.4;
            cursor: not-allowed;
            transform: none !important;
        }

        /* ───── Main Content ───── */
        .main { flex: 1; padding: 28px; max-width: 1600px; margin: 0 auto; width: 100%; }

        /* ───── Stats Grid ───── */
        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 16px;
            margin-bottom: 28px;
        }

        .stat-card {
            background: var(--panel);
            border: 1px solid var(--panel-border);
            border-radius: var(--radius-lg);
            padding: 20px 22px;
            backdrop-filter: blur(16px);
            transition: border-color 0.2s, transform 0.2s;
            position: relative;
            overflow: hidden;
        }

        .stat-card::before {
            content: '';
            position: absolute;
            inset: 0;
            border-radius: inherit;
            opacity: 0;
            transition: opacity 0.2s;
            pointer-events: none;
        }

        .stat-card:hover { transform: translateY(-2px); }
        .stat-card:hover::before { opacity: 1; }

        .stat-card.green::before { background: radial-gradient(circle at top left, var(--accent-dim) 0%, transparent 60%); }
        .stat-card.blue::before  { background: radial-gradient(circle at top left, var(--blue-dim) 0%, transparent 60%); }
        .stat-card.purple::before { background: radial-gradient(circle at top left, var(--purple-dim) 0%, transparent 60%); }
        .stat-card.amber::before  { background: radial-gradient(circle at top left, var(--amber-dim) 0%, transparent 60%); }

        .stat-card:hover.green  { border-color: var(--accent-border); }
        .stat-card:hover.blue   { border-color: rgba(59,130,246,0.3); }
        .stat-card:hover.purple { border-color: rgba(168,85,247,0.3); }
        .stat-card:hover.amber  { border-color: rgba(245,158,11,0.3); }

        .stat-icon {
            width: 38px;
            height: 38px;
            border-radius: var(--radius-sm);
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 18px;
            margin-bottom: 14px;
        }

        .stat-icon.green  { background: var(--accent-dim); }
        .stat-icon.blue   { background: var(--blue-dim); }
        .stat-icon.purple { background: var(--purple-dim); }
        .stat-icon.amber  { background: var(--amber-dim); }

        .stat-label {
            font-size: 11px;
            font-weight: 600;
            color: var(--text-3);
            text-transform: uppercase;
            letter-spacing: 0.6px;
            margin-bottom: 6px;
        }

        .stat-value {
            font-size: 36px;
            font-weight: 700;
            letter-spacing: -1.5px;
            font-family: 'IBM Plex Mono', monospace;
            line-height: 1;
        }

        .stat-value.green  { color: var(--accent); }
        .stat-value.blue   { color: var(--blue); }
        .stat-value.purple { color: var(--purple); }
        .stat-value.amber  { color: var(--amber); }

        .stat-sub {
            font-size: 11px;
            color: var(--text-3);
            margin-top: 6px;
        }

        /* ───── Tabs ───── */
        .tabs {
            display: flex;
            gap: 4px;
            border-bottom: 1px solid var(--panel-border);
            margin-bottom: 24px;
            overflow-x: auto;
            scrollbar-width: none;
            padding-bottom: 0;
        }

        .tab-btn {
            display: inline-flex;
            align-items: center;
            gap: 8px;
            padding: 10px 16px;
            font-size: 13px;
            font-weight: 500;
            color: var(--text-3);
            background: transparent;
            border: none;
            border-bottom: 2px solid transparent;
            border-radius: 0;
            cursor: pointer;
            transition: color 0.18s, border-color 0.18s;
            white-space: nowrap;
            margin-bottom: -1px;
        }

        .tab-btn:hover { color: var(--text-2); background: transparent; transform: none; }

        .tab-btn.active {
            color: var(--accent);
            border-bottom-color: var(--accent);
            font-weight: 600;
        }

        .tab-count {
            background: var(--surface-3);
            border-radius: 10px;
            padding: 1px 7px;
            font-size: 10px;
            font-weight: 700;
            font-family: 'IBM Plex Mono', monospace;
        }

        .tab-panel { display: none; }
        .tab-panel.active { display: block; }

        /* ───── Panel ───── */
        .panel {
            background: var(--panel);
            border: 1px solid var(--panel-border);
            border-radius: var(--radius-lg);
            backdrop-filter: blur(16px);
            overflow: hidden;
        }

        .panel-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 18px 22px;
            border-bottom: 1px solid var(--panel-border);
            gap: 14px;
            flex-wrap: wrap;
        }

        .panel-title {
            display: flex;
            align-items: center;
            gap: 9px;
            font-size: 14px;
            font-weight: 600;
            color: var(--text-1);
        }

        .panel-title-icon {
            width: 30px;
            height: 30px;
            border-radius: 8px;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 15px;
            background: var(--surface-3);
        }

        .panel-actions {
            display: flex;
            align-items: center;
            gap: 8px;
            flex-wrap: wrap;
        }

        .panel-body { padding: 22px; }

        /* ───── Tables ───── */
        .table-wrap { overflow-x: auto; }

        table {
            width: 100%;
            border-collapse: collapse;
            font-size: 13px;
        }

        thead th {
            padding: 10px 14px;
            text-align: left;
            font-size: 11px;
            font-weight: 600;
            color: var(--text-3);
            text-transform: uppercase;
            letter-spacing: 0.5px;
            background: rgba(255,255,255,0.02);
            border-bottom: 1px solid var(--panel-border);
            white-space: nowrap;
        }

        tbody td {
            padding: 13px 14px;
            border-bottom: 1px solid rgba(255,255,255,0.04);
            vertical-align: middle;
        }

        tbody tr:last-child td { border-bottom: none; }
        tbody tr:hover td { background: var(--panel-hover); }

        .td-mono {
            font-family: 'IBM Plex Mono', monospace;
            font-size: 12px;
        }

        .td-muted { color: var(--text-3); font-size: 12px; }

        /* ───── Badges / Chips ───── */
        .chip {
            display: inline-flex;
            align-items: center;
            gap: 5px;
            font-size: 11px;
            font-weight: 600;
            padding: 3px 9px;
            border-radius: 20px;
            white-space: nowrap;
        }

        .chip-green  { background: var(--accent-dim);  color: var(--accent);  border: 1px solid var(--accent-border); }
        .chip-blue   { background: var(--blue-dim);    color: var(--blue);    border: 1px solid rgba(59,130,246,0.28); }
        .chip-purple { background: var(--purple-dim);  color: var(--purple);  border: 1px solid rgba(168,85,247,0.28); }
        .chip-amber  { background: var(--amber-dim);   color: var(--amber);   border: 1px solid rgba(245,158,11,0.28); }
        .chip-danger { background: var(--danger-dim);  color: var(--danger);  border: 1px solid rgba(239,68,68,0.28); }
        .chip-neutral{ background: rgba(255,255,255,0.06); color: var(--text-2); border: 1px solid var(--panel-border); }

        .chip-dot {
            width: 5px;
            height: 5px;
            border-radius: 50%;
            background: currentColor;
        }

        /* ───── Username Cell ───── */
        .user-cell {
            font-family: 'IBM Plex Mono', monospace;
            font-weight: 600;
            color: var(--accent);
            font-size: 13px;
        }

        /* ───── Pubkey cell ───── */
        .pubkey-cell {
            font-family: 'IBM Plex Mono', monospace;
            font-size: 11px;
            color: var(--blue);
            max-width: 200px;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
            cursor: pointer;
            transition: color 0.15s;
        }
        .pubkey-cell:hover { color: #93c5fd; text-decoration: underline; }

        /* ───── Empty State ───── */
        .empty-state {
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            padding: 60px 20px;
            color: var(--text-3);
            gap: 10px;
            text-align: center;
        }

        .empty-state-icon { font-size: 36px; opacity: 0.5; }
        .empty-state-text { font-size: 14px; color: var(--text-2); }
        .empty-state-sub  { font-size: 12px; color: var(--text-3); }

        /* ───── Overview two-col layout ───── */
        .overview-grid {
            display: grid;
            grid-template-columns: 1fr 360px;
            gap: 22px;
        }
        @media (max-width: 1024px) {
            .overview-grid { grid-template-columns: 1fr; }
        }

        /* ───── Announcement Form ───── */
        .announcement-form {
            display: flex;
            flex-direction: column;
            gap: 12px;
        }

        .template-chips {
            display: flex;
            gap: 6px;
            flex-wrap: wrap;
        }

        .tpl-chip {
            background: var(--surface-3);
            border: 1px solid var(--panel-border);
            color: var(--text-2);
            font-size: 11px;
            font-weight: 500;
            padding: 5px 11px;
            border-radius: 20px;
            cursor: pointer;
            transition: all 0.15s;
            font-family: inherit;
        }

        .tpl-chip:hover {
            border-color: var(--accent-border);
            color: var(--accent);
            background: var(--accent-dim);
            transform: none;
        }

        textarea {
            resize: vertical;
            min-height: 88px;
            width: 100%;
        }

        /* ───── Announcement List ───── */
        .announcement-list {
            display: flex;
            flex-direction: column;
            gap: 10px;
        }

        .ann-item {
            background: var(--surface-3);
            border: 1px solid var(--panel-border);
            border-radius: var(--radius-md);
            padding: 14px 16px;
            transition: border-color 0.2s;
        }

        .ann-item:hover { border-color: rgba(255,255,255,0.12); }

        .ann-meta {
            display: flex;
            align-items: center;
            justify-content: space-between;
            margin-bottom: 8px;
            gap: 10px;
        }

        .ann-id {
            font-family: 'IBM Plex Mono', monospace;
            font-size: 11px;
            color: var(--text-3);
        }

        .ann-text {
            font-size: 13px;
            color: var(--text-1);
            line-height: 1.5;
        }

        /* ───── Search ───── */
        .search-wrap {
            position: relative;
        }

        .search-wrap input {
            padding-left: 34px;
            width: 200px;
        }

        .search-icon {
            position: absolute;
            left: 10px;
            top: 50%;
            transform: translateY(-50%);
            color: var(--text-3);
            font-size: 14px;
            pointer-events: none;
        }

        /* ───── Session list ───── */
        .session-list {
            display: flex;
            flex-direction: column;
            gap: 8px;
        }

        .session-item {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 12px 16px;
            background: var(--surface-3);
            border: 1px solid var(--panel-border);
            border-radius: var(--radius-md);
            gap: 10px;
        }

        .session-user {
            font-family: 'IBM Plex Mono', monospace;
            font-size: 13px;
            font-weight: 600;
            color: var(--accent);
        }

        /* ───── Toast ───── */
        #toast {
            position: fixed;
            bottom: 28px;
            right: 28px;
            display: flex;
            align-items: center;
            gap: 10px;
            padding: 13px 18px;
            border-radius: var(--radius-md);
            font-size: 13px;
            font-weight: 500;
            box-shadow: 0 12px 40px rgba(0,0,0,0.6);
            transform: translateY(120px);
            opacity: 0;
            transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
            z-index: 9999;
            max-width: 360px;
            pointer-events: none;
        }
        #toast.show { transform: translateY(0); opacity: 1; }
        #toast.success { background: #0b1f0e; border: 1px solid rgba(16,185,129,0.4); color: #6ee7b7; }
        #toast.error   { background: #1f0b0b; border: 1px solid rgba(239,68,68,0.4);  color: #fca5a5; }
        #toast.info    { background: #0b1020; border: 1px solid rgba(59,130,246,0.4); color: #93c5fd; }

        /* ───── Auto-refresh countdown ───── */
        .refresh-info {
            font-size: 11px;
            color: var(--text-3);
            font-family: 'IBM Plex Mono', monospace;
            min-width: 60px;
            text-align: center;
        }

        /* ───── Divider ───── */
        .divider { height: 1px; background: var(--panel-border); margin: 20px 0; }

        /* ───── Last updated bar ───── */
        .last-updated {
            display: flex;
            align-items: center;
            gap: 8px;
            font-size: 11px;
            color: var(--text-3);
            font-family: 'IBM Plex Mono', monospace;
        }

        /* ───── Responsive ───── */
        @media (max-width: 768px) {
            .topbar { padding: 12px 16px; }
            .main   { padding: 16px; }
            .stats-grid { grid-template-columns: 1fr 1fr; }
            .tab-btn { padding: 8px 12px; font-size: 12px; }
        }

        @media (max-width: 480px) {
            .stats-grid { grid-template-columns: 1fr; }
        }

        /* ───── Scrollbar ───── */
        ::-webkit-scrollbar { width: 6px; height: 6px; }
        ::-webkit-scrollbar-track { background: transparent; }
        ::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.1); border-radius: 4px; }
        ::-webkit-scrollbar-thumb:hover { background: rgba(255,255,255,0.2); }

        /* ───── Skeleton loading ───── */
        .skeleton {
            background: linear-gradient(90deg, var(--surface-3) 25%, rgba(255,255,255,0.05) 50%, var(--surface-3) 75%);
            background-size: 200% 100%;
            animation: shimmer 1.4s infinite;
            border-radius: 4px;
            height: 14px;
        }
        @keyframes shimmer {
            0% { background-position: 200% 0; }
            100% { background-position: -200% 0; }
        }
    </style>
</head>
<body>
<div class="shell">

    <!-- ── Top Bar ── -->
    <header class="topbar">
        <div class="brand">
            <div class="brand-icon">⚡</div>
            <div>
                <div class="brand-name"><span>VEXTA</span> BRIDGE V2</div>
                <div class="brand-sub">Vexta Bridge V2 - v0.0.1</div>
            </div>
            <div class="status-badge"><span class="status-dot"></span> ONLINE</div>
        </div>

        <div class="auth-section">
            <div id="lastUpdatedBar" class="last-updated" style="display:none;">
                <span>⏱</span>
                <span id="lastUpdatedText">—</span>
            </div>
            <div class="refresh-info" id="refreshCountdown" style="display:none;"></div>
            <input type="password" id="adminSecret" placeholder="Admin Secret Token" style="width: 230px;" onkeydown="if(event.key==='Enter')fetchAdminData()">
            <button class="btn-primary" onclick="fetchAdminData()" id="btnAuth">
                <span>🔐</span> Authenticate
            </button>
            <button class="btn-secondary btn-ghost" onclick="toggleAutoRefresh()" id="btnAutoRefresh">
                <span>🔄</span> Auto-Refresh
            </button>
        </div>
    </header>

    <!-- ── Main ── -->
    <main class="main">

        <!-- Stats -->
        <div class="stats-grid">
            <div class="stat-card green">
                <div class="stat-icon green">🟢</div>
                <div class="stat-label">Live WS Sessions</div>
                <div class="stat-value green" id="statSessions">—</div>
                <div class="stat-sub">Active connections</div>
            </div>
            <div class="stat-card blue">
                <div class="stat-icon blue">👤</div>
                <div class="stat-label">Registered Accounts</div>
                <div class="stat-value blue" id="statUsers">—</div>
                <div class="stat-sub">Total users in DB</div>
            </div>
            <div class="stat-card amber">
                <div class="stat-icon amber">📨</div>
                <div class="stat-label">Queued Offline Msgs</div>
                <div class="stat-value amber" id="statQueued">—</div>
                <div class="stat-sub">Pending delivery</div>
            </div>
            <div class="stat-card purple">
                <div class="stat-icon purple">📱</div>
                <div class="stat-label">Registered Devices</div>
                <div class="stat-value purple" id="statDevices">—</div>
                <div class="stat-sub">Across all accounts</div>
            </div>
            <div class="stat-card amber">
                <div class="stat-icon amber">📢</div>
                <div class="stat-label">Active Announcements</div>
                <div class="stat-value amber" id="statAnnouncements">—</div>
                <div class="stat-sub">System broadcasts</div>
            </div>
        </div>

        <!-- Tabs -->
        <div class="tabs" role="tablist">
            <button class="tab-btn active" onclick="switchTab('overview')" id="tab-overview" role="tab" aria-selected="true">
                📊 Overview
            </button>
            <button class="tab-btn" onclick="switchTab('users')" id="tab-users" role="tab" aria-selected="false">
                👥 Users <span class="tab-count" id="tabCountUsers">0</span>
            </button>
            <button class="tab-btn" onclick="switchTab('devices')" id="tab-devices" role="tab" aria-selected="false">
                📱 Devices <span class="tab-count" id="tabCountDevices">0</span>
            </button>
            <button class="tab-btn" onclick="switchTab('announcements')" id="tab-announcements" role="tab" aria-selected="false">
                📢 Announcements
            </button>
            <button class="tab-btn" onclick="switchTab('firewall')" id="tab-firewall" role="tab" aria-selected="false">
                🛡️ IP Firewall
            </button>
            <button class="tab-btn" onclick="switchTab('maintenance')" id="tab-maintenance" role="tab" aria-selected="false">
                🚨 Maintenance
            </button>
            <button class="tab-btn" onclick="switchTab('audit')" id="tab-audit" role="tab" aria-selected="false">
                📜 Audit Logs
            </button>
            <button class="tab-btn" onclick="switchTab('analytics')" id="tab-analytics" role="tab" aria-selected="false">
                📊 Traffic Leaderboard
            </button>
            <button class="tab-btn" onclick="switchTab('vacuum')" id="tab-vacuum" role="tab" aria-selected="false">
                🧹 DB Vacuum
            </button>
        </div>

        <!-- ── Tab: Overview ── -->
        <div class="tab-panel active" id="panel-overview">
            <div class="overview-grid">

                <!-- Active Sessions -->
                <div class="panel">
                    <div class="panel-header">
                        <div class="panel-title">
                            <div class="panel-title-icon">🟢</div>
                            Live Sessions
                        </div>
                        <button class="btn-ghost btn-icon" onclick="fetchAdminData()" title="Refresh">↻</button>
                    </div>
                    <div class="panel-body">
                        <div id="sessionList">
                            <div class="empty-state">
                                <div class="empty-state-icon">🔌</div>
                                <div class="empty-state-text">No active sessions</div>
                                <div class="empty-state-sub">Authenticate to view live data</div>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- Quick Broadcast -->
                <div style="display:flex;flex-direction:column;gap:20px;">
                    <div class="panel">
                        <div class="panel-header">
                            <div class="panel-title">
                                <div class="panel-title-icon">📢</div>
                                Quick Broadcast
                            </div>
                        </div>
                        <div class="panel-body">
                            <div class="announcement-form">
                                <div class="template-chips">
                                    <button class="tpl-chip" onclick="applyTemplate('[RELEASE] Vexta Client v0.1.0 is live. Please update.')">🚀 Release</button>
                                    <button class="tpl-chip" onclick="applyTemplate('[MAINTENANCE] Bridge maintenance scheduled in 15 mins.')">🔧 Maintenance</button>
                                    <button class="tpl-chip" onclick="applyTemplate('[SECURITY] Rotate your keys and verify server fingerprint.')">🔒 Security</button>
                                    <button class="tpl-chip" onclick="applyTemplate('[INFO] Service update in progress. Expect brief disruption.')">ℹ️ Info</button>
                                </div>
                                <textarea id="announcementText" placeholder="Type a system-wide broadcast message…"></textarea>
                                <button class="btn-primary" onclick="postAnnouncement()" style="width:100%;">
                                    <span>📡</span> Broadcast to All Users
                                </button>
                            </div>
                        </div>
                    </div>

                    <!-- Server Info -->
                    <div class="panel">
                        <div class="panel-header">
                            <div class="panel-title">
                                <div class="panel-title-icon">⚙️</div>
                                Server Info
                            </div>
                        </div>
                        <div class="panel-body" style="display:flex;flex-direction:column;gap:10px;">
                            <div style="display:flex;justify-content:space-between;align-items:center;font-size:12px;">
                                <span style="color:var(--text-3);">WS Endpoint</span>
                                <span class="td-mono" style="color:var(--blue);font-size:11px;">/ws/chat</span>
                            </div>
                            <div style="display:flex;justify-content:space-between;align-items:center;font-size:12px;">
                                <span style="color:var(--text-3);">Admin API</span>
                                <span class="td-mono" style="color:var(--blue);font-size:11px;">/api/admin/*</span>
                            </div>
                            <div style="display:flex;justify-content:space-between;align-items:center;font-size:12px;">
                                <span style="color:var(--text-3);">Public API</span>
                                <span class="td-mono" style="color:var(--blue);font-size:11px;">/api/announcements</span>
                            </div>
                            <div class="divider"></div>
                            <div style="display:flex;justify-content:space-between;align-items:center;font-size:12px;">
                                <span style="color:var(--text-3);">Console Version</span>
                                <span class="chip chip-green" style="font-size:10px;">v2.1</span>
                            </div>
                        </div>
                    </div>
                </div>

            </div>
        </div>

        <!-- ── Tab: Users ── -->
        <div class="tab-panel" id="panel-users">
            <div class="panel">
                <div class="panel-header">
                    <div class="panel-title">
                        <div class="panel-title-icon">👥</div>
                        User Accounts
                    </div>
                    <div class="panel-actions">
                        <div class="search-wrap">
                            <span class="search-icon">🔍</span>
                            <input type="text" id="userSearch" placeholder="Search users…" oninput="filterUsers()" style="padding-left:34px;width:200px;">
                        </div>
                        <button class="btn-ghost" onclick="fetchAdminData()"><span>↻</span> Refresh</button>
                    </div>
                </div>
                <div class="table-wrap">
                    <table id="userTable">
                        <thead>
                            <tr>
                                <th>Username</th>
                                <th>Ed25519 Public Key</th>
                                <th>Status</th>
                                <th>Auth Attempts</th>
                                <th>Provisioned</th>
                                <th>Registered</th>
                                <th style="text-align:right;">Action</th>
                            </tr>
                        </thead>
                        <tbody id="userTableBody">
                            <tr>
                                <td colspan="7">
                                    <div class="empty-state">
                                        <div class="empty-state-icon">🔑</div>
                                        <div class="empty-state-text">Authentication required</div>
                                        <div class="empty-state-sub">Enter your Admin Secret Token and authenticate to view user data.</div>
                                    </div>
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>
        </div>

        <!-- ── Tab: Devices ── -->
        <div class="tab-panel" id="panel-devices">
            <div class="panel">
                <div class="panel-header">
                    <div class="panel-title">
                        <div class="panel-title-icon">📱</div>
                        Registered Devices
                    </div>
                    <div class="panel-actions">
                        <div class="search-wrap">
                            <span class="search-icon">🔍</span>
                            <input type="text" id="deviceSearch" placeholder="Search devices…" oninput="filterDevices()" style="padding-left:34px;width:200px;">
                        </div>
                        <button class="btn-ghost" onclick="fetchAdminData()"><span>↻</span> Refresh</button>
                    </div>
                </div>
                <div class="table-wrap">
                    <table>
                        <thead>
                            <tr>
                                <th>#</th>
                                <th>Owner</th>
                                <th>Device Name</th>
                                <th>Type</th>
                                <th>Hardware Hash</th>
                                <th>Registered</th>
                                <th>Last Active</th>
                            </tr>
                        </thead>
                        <tbody id="deviceTableBody">
                            <tr>
                                <td colspan="7">
                                    <div class="empty-state">
                                        <div class="empty-state-icon">📵</div>
                                        <div class="empty-state-text">Authentication required</div>
                                        <div class="empty-state-sub">Enter your Admin Secret Token and authenticate to view device data.</div>
                                    </div>
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>
        </div>

        <!-- ── Tab: Announcements ── -->
        <div class="tab-panel" id="panel-announcements">
            <div style="display:grid;grid-template-columns:1fr 380px;gap:22px;">

                <!-- Active Announcements -->
                <div class="panel">
                    <div class="panel-header">
                        <div class="panel-title">
                            <div class="panel-title-icon">📋</div>
                            Active Announcements
                        </div>
                        <button class="btn-ghost" onclick="fetchAnnouncements()"><span>↻</span> Refresh</button>
                    </div>
                    <div class="panel-body">
                        <div id="announcementsList" class="announcement-list">
                            <div class="empty-state">
                                <div class="empty-state-icon">📭</div>
                                <div class="empty-state-text">No active announcements</div>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- Broadcast Form (full) -->
                <div class="panel">
                    <div class="panel-header">
                        <div class="panel-title">
                            <div class="panel-title-icon">📡</div>
                            Broadcast Message
                        </div>
                    </div>
                    <div class="panel-body">
                        <div class="announcement-form">
                            <div style="font-size:12px;color:var(--text-3);margin-bottom:4px;">Quick templates:</div>
                            <div class="template-chips">
                                <button class="tpl-chip" onclick="applyTemplate('[RELEASE] Vexta Client v0.1.0 is live. Please update.')">🚀 Release</button>
                                <button class="tpl-chip" onclick="applyTemplate('[MAINTENANCE] Scheduled bridge maintenance in 15 mins.')">🔧 Maintenance</button>
                                <button class="tpl-chip" onclick="applyTemplate('[SECURITY] Rotate your keys and verify server fingerprint.')">🔒 Security</button>
                                <button class="tpl-chip" onclick="applyTemplate('[NOTICE] The relay server will be restarted shortly.')">📌 Notice</button>
                                <button class="tpl-chip" onclick="applyTemplate('[INFO] Service restored. All systems operational.')">✅ Resolved</button>
                            </div>
                            <textarea id="announcementText2" rows="5" placeholder="Type system-wide broadcast message…" style="width:100%;"></textarea>
                            <button class="btn-primary" onclick="postAnnouncement2()" style="width:100%;">
                                <span>📡</span> Broadcast to All Users
                            </button>
                            <div style="font-size:11px;color:var(--text-3);line-height:1.5;">
                                Messages are delivered to all connected clients over WebSocket and persisted in the database for offline users.
                            </div>
                        </div>
                    </div>
                </div>

            </div>
        </div>

    </main>
</div>

<!-- Toast -->
<div id="toast" role="alert" aria-live="polite"></div>

<script>
    // ── State ──
    let rawUsers   = [];
    let rawDevices = [];
    let autoRefreshInterval = null;
    let autoRefreshCountdown = 0;
    let countdownTimer = null;
    let currentTab = 'overview';
    let isAuthenticated = false;

    // ── Helpers ──
    function getSecret() {
        return document.getElementById('adminSecret').value.trim()
            || localStorage.getItem('vexta_admin_secret')
            || '';
    }

    function showToast(msg, type = 'success') {
        const el = document.getElementById('toast');
        el.className = `show ${type}`;
        el.innerHTML = (type === 'success' ? '✓ ' : type === 'error' ? '✕ ' : 'ℹ ') + msg;
        clearTimeout(el._timer);
        el._timer = setTimeout(() => el.classList.remove('show'), 3500);
    }

    function fmtDate(ts) {
        if (!ts) return '—';
        const d = new Date(ts * 1000);
        return d.toLocaleDateString(undefined, { year:'numeric', month:'short', day:'numeric' })
             + ' ' + d.toLocaleTimeString(undefined, { hour:'2-digit', minute:'2-digit' });
    }

    function fmtRelative(ts) {
        if (!ts) return '—';
        const now = Date.now() / 1000;
        const diff = now - ts;
        if (diff < 60)   return 'Just now';
        if (diff < 3600) return Math.floor(diff / 60) + 'm ago';
        if (diff < 86400) return Math.floor(diff / 3600) + 'h ago';
        return Math.floor(diff / 86400) + 'd ago';
    }

    function truncate(str, n) {
        return str && str.length > n ? str.slice(0, n) + '…' : str || '—';
    }

    function switchTab(name) {
        currentTab = name;
        document.querySelectorAll('.tab-btn').forEach(b => {
            b.classList.toggle('active', b.id === 'tab-' + name);
            b.setAttribute('aria-selected', b.id === 'tab-' + name);
        });
        document.querySelectorAll('.tab-panel').forEach(p => {
            p.classList.toggle('active', p.id === 'panel-' + name);
        });
    }

    function applyTemplate(text) {
        document.getElementById('announcementText').value = text;
        document.getElementById('announcementText2').value = text;
    }

    function copyToClipboard(text) {
        navigator.clipboard.writeText(text).then(() => showToast('Copied to clipboard!', 'info'));
    }

    // ── Auto Refresh ──
    function toggleAutoRefresh() {
        const btn = document.getElementById('btnAutoRefresh');
        const cd  = document.getElementById('refreshCountdown');
        if (autoRefreshInterval) {
            clearInterval(autoRefreshInterval);
            clearInterval(countdownTimer);
            autoRefreshInterval = null;
            countdownTimer = null;
            btn.innerHTML = '<span>🔄</span> Auto-Refresh';
            cd.style.display = 'none';
            showToast('Auto-refresh disabled', 'info');
        } else {
            if (!getSecret()) { showToast('Authenticate first', 'error'); return; }
            autoRefreshCountdown = 10;
            cd.style.display = 'block';
            cd.textContent = `↻ ${autoRefreshCountdown}s`;
            autoRefreshInterval = setInterval(fetchAdminData, 10000);
            countdownTimer = setInterval(() => {
                autoRefreshCountdown = Math.max(0, autoRefreshCountdown - 1);
                if (autoRefreshCountdown === 0) autoRefreshCountdown = 10;
                cd.textContent = `↻ ${autoRefreshCountdown}s`;
            }, 1000);
            btn.innerHTML = '<span>⏹</span> Stop Refresh';
            showToast('Auto-refresh every 10s', 'info');
        }
    // ── Realtime SSE Stream ──
    let sseSource = null;

    function connectSseStream(secret) {
        if (sseSource) {
            sseSource.close();
        }
        sseSource = new EventSource('/api/admin/events?token=' + encodeURIComponent(secret));
        sseSource.onmessage = function(e) {
            try {
                const data = JSON.parse(e.data);
                console.log('[Admin SSE] Live event:', data);
                if (data.event === 'session_connected' || data.event === 'session_disconnected') {
                    if (data.active_count !== undefined) {
                        document.getElementById('statSessions').textContent = data.active_count;
                        renderSessionList(data.active_count);
                    }
                    showToast(`⚡ Live event: Session ${data.event === 'session_connected' ? 'connected (' + data.username + ')' : 'disconnected (' + data.username + ')'}`, 'info');
                } else if (data.event === 'traffic_recorded') {
                    if (data.total_messages !== undefined) {
                        document.getElementById('statQueued').textContent = data.total_messages;
                    }
                } else if (data.event === 'announcement_created') {
                    fetchAnnouncements();
                    showToast('⚡ Live event: New announcement broadcasted', 'info');
                }
                const bar = document.getElementById('lastUpdatedBar');
                bar.style.display = 'flex';
                document.getElementById('lastUpdatedText').textContent = '⚡ Realtime SSE Stream Active • ' + new Date().toLocaleTimeString();
            } catch (err) {
                console.warn('[Admin SSE] Parse error:', err);
            }
        };
        sseSource.onerror = function() {
            console.warn('[Admin SSE] Connection lost — reconnecting automatically...');
        };
    }

    // ── Fetch All Data ──
    async function fetchAdminData() {
        const secret = getSecret();
        if (!secret) { showToast('Enter your Admin Secret Token first', 'error'); return; }
        localStorage.setItem('vexta_admin_secret', secret);

        const btn = document.getElementById('btnAuth');
        btn.disabled = true;
        btn.innerHTML = '<span>⏳</span> Loading…';

        try {
            // Stats
            const resStats = await fetch('/api/admin/stats', { headers: { 'X-Admin-Secret': secret } });
            if (resStats.status === 401) {
                showToast('Unauthorized: Invalid Admin Secret Token', 'error');
                btn.disabled = false;
                btn.innerHTML = '<span>🔐</span> Authenticate';
                return;
            }
            const stats = await resStats.json();
            document.getElementById('statSessions').textContent      = stats.active_ws_sessions ?? 0;
            document.getElementById('statUsers').textContent         = stats.total_users ?? 0;
            document.getElementById('statQueued').textContent        = stats.total_queued_offline_messages ?? 0;
            document.getElementById('statDevices').textContent       = stats.total_registered_devices ?? 0;
            document.getElementById('statAnnouncements').textContent = stats.total_announcements ?? 0;

            isAuthenticated = true;
            connectSseStream(secret);

            // Users
            const resUsers = await fetch('/api/admin/users', { headers: { 'X-Admin-Secret': secret } });
            rawUsers = await resUsers.json();
            renderUsers(rawUsers);
            document.getElementById('tabCountUsers').textContent = rawUsers.length;

            // Devices
            const resDevices = await fetch('/api/admin/devices', { headers: { 'X-Admin-Secret': secret } });
            rawDevices = await resDevices.json();
            renderDevices(rawDevices);
            document.getElementById('tabCountDevices').textContent = rawDevices.length;

            // Sessions (derived from stats — actual list not available without further API)
            renderSessionList(stats.active_ws_sessions ?? 0);

            // Announcements
            await fetchAnnouncements();

            // Last updated
            const bar = document.getElementById('lastUpdatedBar');
            bar.style.display = 'flex';
            document.getElementById('lastUpdatedText').textContent = 'Updated ' + new Date().toLocaleTimeString();

            showToast('Dashboard refreshed', 'success');
        } catch (err) {
            showToast('Network error — check server connectivity', 'error');
            console.error(err);
        } finally {
            btn.disabled = false;
            btn.innerHTML = '<span>🔐</span> Authenticate';
        }
    }

    // ── Render Sessions ──
    function renderSessionList(count) {
        const el = document.getElementById('sessionList');
        if (count === 0) {
            el.innerHTML = `
                <div class="empty-state">
                    <div class="empty-state-icon">💤</div>
                    <div class="empty-state-text">No active WebSocket sessions</div>
                    <div class="empty-state-sub">Clients connect and authenticate via the WS relay.</div>
                </div>`;
            return;
        }
        // Display count pill since we don't have individual session names from stats
        el.innerHTML = `
            <div style="display:flex;align-items:center;justify-content:space-between;padding:14px 0;">
                <span style="color:var(--text-2);font-size:13px;">Active WebSocket clients</span>
                <span class="chip chip-green" style="font-size:14px;padding:6px 16px;font-family:'IBM Plex Mono',monospace;font-weight:700;">${count}</span>
            </div>
            <div style="font-size:12px;color:var(--text-3);padding:8px 0;">
                Individual session details are not persisted — only the live count is available from the in-memory routing table.
            </div>`;
    }

    // ── Render Users ──
    function renderUsers(users) {
        const tbody = document.getElementById('userTableBody');
        if (!users || users.length === 0) {
            tbody.innerHTML = `<tr><td colspan="7">
                <div class="empty-state">
                    <div class="empty-state-icon">👻</div>
                    <div class="empty-state-text">No users found</div>
                </div>
            </td></tr>`;
            return;
        }

        tbody.innerHTML = users.map(u => {
            const isLocked = u.locked_until && u.locked_until > (Date.now() / 1000);
            const statusChip = isLocked
                ? `<span class="chip chip-danger"><span class="chip-dot"></span> Locked</span>`
                : `<span class="chip chip-green"><span class="chip-dot"></span> Active</span>`;

            const attemptsColor = u.auth_attempts >= 5
                ? 'color:var(--danger)'
                : u.auth_attempts >= 3
                    ? 'color:var(--amber)'
                    : 'color:var(--text-2)';

            const provChip = u.is_provisioned
                ? `<span class="chip chip-blue">✓ Yes</span>`
                : `<span class="chip chip-neutral">No</span>`;

            return `<tr>
                <td class="user-cell">@${u.username}</td>
                <td>
                    <span class="pubkey-cell td-mono" title="${u.ed25519_pubkey}" onclick="copyToClipboard('${u.ed25519_pubkey}')">
                        ${truncate(u.ed25519_pubkey, 36)}
                    </span>
                </td>
                <td>${statusChip}</td>
                <td><span style="${attemptsColor};font-family:'IBM Plex Mono',monospace;font-size:12px;">${u.auth_attempts}</span></td>
                <td>${provChip}</td>
                <td class="td-muted">${fmtDate(u.created_at)}</td>
                <td style="text-align:right;">
                    <button class="btn-danger" onclick="deleteUser('${u.username}')">🗑 Delete</button>
                </td>
            </tr>`;
        }).join('');
    }

    function filterUsers() {
        const q = document.getElementById('userSearch').value.toLowerCase().trim();
        const filtered = rawUsers.filter(u =>
            u.username.toLowerCase().includes(q) ||
            (u.ed25519_pubkey && u.ed25519_pubkey.toLowerCase().includes(q))
        );
        renderUsers(filtered);
    }

    // ── Render Devices ──
    function renderDevices(devices) {
        const tbody = document.getElementById('deviceTableBody');
        if (!devices || devices.length === 0) {
            tbody.innerHTML = `<tr><td colspan="7">
                <div class="empty-state">
                    <div class="empty-state-icon">📵</div>
                    <div class="empty-state-text">No registered devices</div>
                </div>
            </td></tr>`;
            return;
        }

        tbody.innerHTML = devices.map(d => {
            const typeChip = d.device_type === 'Desktop'
                ? `<span class="chip chip-blue">🖥 Desktop</span>`
                : d.device_type === 'Mobile'
                    ? `<span class="chip chip-purple">📱 Mobile</span>`
                    : `<span class="chip chip-neutral">${d.device_type}</span>`;

            return `<tr>
                <td class="td-mono td-muted">#${d.id}</td>
                <td class="user-cell">@${d.username}</td>
                <td style="font-weight:500;">${d.device_name || '—'}</td>
                <td>${typeChip}</td>
                <td>
                    <span class="pubkey-cell td-mono" title="${d.hardware_hash}" onclick="copyToClipboard('${d.hardware_hash}')">
                        ${truncate(d.hardware_hash, 28)}
                    </span>
                </td>
                <td class="td-muted">${fmtDate(d.registered_at)}</td>
                <td class="td-muted">${fmtRelative(d.last_active)}</td>
            </tr>`;
        }).join('');
    }

    function filterDevices() {
        const q = document.getElementById('deviceSearch').value.toLowerCase().trim();
        const filtered = rawDevices.filter(d =>
            (d.username && d.username.toLowerCase().includes(q)) ||
            (d.device_name && d.device_name.toLowerCase().includes(q)) ||
            (d.hardware_hash && d.hardware_hash.toLowerCase().includes(q))
        );
        renderDevices(filtered);
    }

    // ── Announcements ──
    async function fetchAnnouncements() {
        const secret = getSecret();
        if (!secret) return;
        try {
            const res = await fetch('/api/admin/announcements', { headers: { 'X-Admin-Secret': secret } });
            if (!res.ok) return;
            const list = await res.json();
            const div = document.getElementById('announcementsList');
            if (!list || list.length === 0) {
                div.innerHTML = `<div class="empty-state">
                    <div class="empty-state-icon">📭</div>
                    <div class="empty-state-text">No active announcements</div>
                </div>`;
                return;
            }
            div.innerHTML = list.map(a => `
                <div class="ann-item">
                    <div class="ann-meta">
                        <span class="ann-id">#${a.id} &bull; ${fmtDate(a.created_at)}</span>
                        <button class="btn-danger" style="padding:4px 10px;font-size:11px;" onclick="deleteAnnouncement(${a.id})">🗑 Delete</button>
                    </div>
                    <div class="ann-text">${escapeHtml(a.message)}</div>
                </div>
            `).join('');
            document.getElementById('statAnnouncements').textContent = list.length;
        } catch(e) { console.error(e); }
    }

    async function postAnnouncement() {
        const secret = getSecret();
        if (!secret) { showToast('Authenticate first', 'error'); return; }
        const message = document.getElementById('announcementText').value.trim();
        if (!message) { showToast('Message cannot be empty', 'error'); return; }
        await _postAnnouncement(secret, message);
        document.getElementById('announcementText').value = '';
    }

    async function postAnnouncement2() {
        const secret = getSecret();
        if (!secret) { showToast('Authenticate first', 'error'); return; }
        const message = document.getElementById('announcementText2').value.trim();
        if (!message) { showToast('Message cannot be empty', 'error'); return; }
        await _postAnnouncement(secret, message);
        document.getElementById('announcementText2').value = '';
    }

    async function _postAnnouncement(secret, message) {
        try {
            const res = await fetch('/api/admin/announcements', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json', 'X-Admin-Secret': secret },
                body: JSON.stringify({ message })
            });
            if (res.ok) {
                showToast('Announcement broadcast successfully ✓', 'success');
                await fetchAnnouncements();
            } else {
                showToast('Failed to post announcement', 'error');
            }
        } catch(e) {
            showToast('Network error', 'error');
        }
    }

    async function deleteAnnouncement(id) {
        const secret = getSecret();
        try {
            await fetch(`/api/admin/announcements/${id}`, {
                method: 'DELETE',
                headers: { 'X-Admin-Secret': secret }
            });
            showToast(`Announcement #${id} deleted`, 'info');
            fetchAnnouncements();
        } catch(e) { showToast('Delete failed', 'error'); }
    }

    async function deleteUser(username) {
        if (!confirm(`Permanently delete user '@${username}' and all their data?\n\nThis action cannot be undone.`)) return;
        const secret = getSecret();
        try {
            await fetch(`/api/admin/users/${username}`, {
                method: 'DELETE',
                headers: { 'X-Admin-Secret': secret }
            });
            showToast(`User @${username} deleted`, 'info');
            fetchAdminData();
        } catch(e) { showToast('Delete failed', 'error'); }
    }

    function escapeHtml(str) {
        return str.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;');
    }

    // ── Boot ──
    window.addEventListener('DOMContentLoaded', () => {
        const saved = localStorage.getItem('vexta_admin_secret');
        if (saved) {
            document.getElementById('adminSecret').value = saved;
            fetchAdminData();
        }
    });
</script>
</body>
</html>
"#;
