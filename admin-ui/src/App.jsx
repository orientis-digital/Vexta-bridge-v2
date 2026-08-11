import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { 
  Shield, 
  Activity, 
  Users, 
  Smartphone, 
  Megaphone, 
  MessageSquare, 
  RefreshCw, 
  Lock, 
  LogOut,
  Trash2, 
  Search, 
  PlusCircle, 
  Key, 
  CheckCircle2, 
  AlertCircle, 
  Info,
  Radio,
  KeyRound,
  ArrowRight
} from 'lucide-react';

// Lightweight Markdown Renderer Component
function MarkdownMessage({ content }) {
  if (!content) return null;

  // Process code blocks first
  const parts = content.split(/(```[\s\S]*?```)/g);

  return (
    <div className="markdown-content">
      {parts.map((part, index) => {
        if (part.startsWith('```') && part.endsWith('```')) {
          const codeLines = part.slice(3, -3).trim().split('\n');
          const firstLine = codeLines[0].trim();
          const hasLang = !firstLine.includes(' ') && firstLine.length > 0;
          const codeContent = hasLang ? codeLines.slice(1).join('\n') : codeLines.join('\n');
          return (
            <pre key={index} className="md-code-block">
              <code>{codeContent || firstLine}</code>
            </pre>
          );
        }

        // Process line by line for headers, lists, paragraphs
        const lines = part.split('\n');
        return (
          <React.Fragment key={index}>
            {lines.map((line, lineIdx) => {
              const trimmed = line.trim();
              if (!trimmed) return <div key={lineIdx} style={{ height: 6 }} />;

              // Headers
              if (trimmed.startsWith('### ')) {
                return <h4 key={lineIdx} className="md-h3">{formatInline(trimmed.slice(4))}</h4>;
              }
              if (trimmed.startsWith('## ')) {
                return <h3 key={lineIdx} className="md-h2">{formatInline(trimmed.slice(3))}</h3>;
              }
              if (trimmed.startsWith('# ')) {
                return <h2 key={lineIdx} className="md-h1">{formatInline(trimmed.slice(2))}</h2>;
              }

              // Bullet lists
              if (trimmed.startsWith('- ') || trimmed.startsWith('* ')) {
                return (
                  <div key={lineIdx} className="md-list-item">
                    <span className="md-bullet">•</span>
                    <span>{formatInline(trimmed.slice(2))}</span>
                  </div>
                );
              }

              return <p key={lineIdx} className="md-p">{formatInline(line)}</p>;
            })}
          </React.Fragment>
        );
      })}
    </div>
  );
}

// Inline formatting helper for bold, italic, code, links
function formatInline(text) {
  if (!text) return '';

  // Regex patterns
  const tokenRegex = /(\*\*.*?\*\*|__.*?__|`.*?`|\[.*?\]\(.*?\))/g;
  const parts = text.split(tokenRegex);

  return parts.map((part, i) => {
    if (part.startsWith('**') && part.endsWith('**')) {
      return <strong key={i}>{part.slice(2, -2)}</strong>;
    }
    if (part.startsWith('__') && part.endsWith('__')) {
      return <strong key={i}>{part.slice(2, -2)}</strong>;
    }
    if (part.startsWith('`') && part.endsWith('`')) {
      return <code key={i} className="md-inline-code">{part.slice(1, -1)}</code>;
    }
    if (part.startsWith('[') && part.includes('](') && part.endsWith(')')) {
      const match = part.match(/^\[(.*?)\]\((.*?)\)$/);
      if (match) {
        return (
          <a
            key={i}
            href={match[2]}
            target="_blank"
            rel="noopener noreferrer"
            className="md-link"
          >
            {match[1]}
          </a>
        );
      }
    }
    return part;
  });
}

export default function App() {
  const [secretKey, setSecretKey] = useState(() => localStorage.getItem('vexta_admin_token') || 'vexta_admin_secret_key_2026');
  const [inputToken, setInputToken] = useState(() => localStorage.getItem('vexta_admin_token') || 'vexta_admin_secret_key_2026');
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [isAuthenticating, setIsAuthenticating] = useState(false);
  const [loginError, setLoginError] = useState('');

  const [activeTab, setActiveTab] = useState('users');
  const [searchQuery, setSearchQuery] = useState('');
  const [autoRefresh, setAutoRefresh] = useState(false);
  const [refreshCountdown, setRefreshCountdown] = useState(10);
  const [lastUpdated, setLastUpdated] = useState(null);

  // Telemetry & Data States (Strictly loaded from backend)
  const [stats, setStats] = useState({
    active_ws_sessions: 0,
    total_users: 0,
    total_queued_offline_messages: 0,
    total_registered_devices: 0,
    total_announcements: 0,
  });
  const [users, setUsers] = useState([]);
  const [devices, setDevices] = useState([]);
  const [announcements, setAnnouncements] = useState([]);
  const [newAnnouncementText, setNewAnnouncementText] = useState('');

  // UI Toast State
  const [toast, setToast] = useState(null);

  const showToast = useCallback((msg, type = 'info') => {
    setToast({ msg, type, id: Date.now() });
    setTimeout(() => setToast(null), 4000);
  }, []);

  // Fetch telemetry and backend data using real REST endpoints
  const fetchData = useCallback(async (tokenToUse = secretKey) => {
    if (!tokenToUse) {
      setIsAuthenticated(false);
      return false;
    }
    const headers = { 'x-admin-secret': tokenToUse };

    try {
      // 1. Stats Telemetry API
      const resStats = await fetch('/api/admin/stats', { headers });
      if (resStats.status === 401) {
        setIsAuthenticated(false);
        setLoginError('Invalid admin secret token. Access denied by backend.');
        return false;
      }
      if (!resStats.ok) {
        setLoginError('Backend connection error (HTTP ' + resStats.status + ')');
        return false;
      }

      const statsData = await resStats.json();
      setStats(statsData);
      setIsAuthenticated(true);
      setLoginError('');
      localStorage.setItem('vexta_admin_token', tokenToUse);

      // 2. Users REST API
      const resUsers = await fetch('/api/admin/users', { headers });
      if (resUsers.ok) {
        const usersData = await resUsers.json();
        setUsers(usersData);
      }

      // 3. Devices REST API
      const resDevices = await fetch('/api/admin/devices', { headers });
      if (resDevices.ok) {
        const devicesData = await resDevices.json();
        setDevices(devicesData);
      }

      // 4. Announcements REST API
      const resAnnouncements = await fetch('/api/admin/announcements', { headers });
      if (resAnnouncements.ok) {
        const annData = await resAnnouncements.json();
        setAnnouncements(annData);
      }

      setLastUpdated(new Date().toLocaleTimeString());
      return true;
    } catch (err) {
      console.error('Failed connecting to bridge backend:', err);
      setLoginError('Unable to connect to Rust backend server on port 8000.');
      return false;
    }
  }, [secretKey]);

  // Submit Login Token
  const handleLoginSubmit = async (e) => {
    e.preventDefault();
    if (!inputToken.trim()) {
      setLoginError('Please enter an Admin Secret Token');
      return;
    }
    setIsAuthenticating(true);
    setLoginError('');

    const success = await fetchData(inputToken.trim());
    if (success) {
      setSecretKey(inputToken.trim());
      showToast('Authenticated successfully with Vexta Bridge V2!', 'success');
    }
    setIsAuthenticating(false);
  };

  // Logout action
  const handleLogout = () => {
    localStorage.removeItem('vexta_admin_token');
    setSecretKey('');
    setInputToken('');
    setIsAuthenticated(false);
    showToast('Logged out of admin session', 'info');
  };

  // Initial load check
  useEffect(() => {
    if (secretKey) {
      fetchData(secretKey);
    }
  }, [fetchData, secretKey]);

  // Auto Refresh Interval
  useEffect(() => {
    let intervalId = null;
    let timerId = null;

    if (autoRefresh && isAuthenticated) {
      setRefreshCountdown(10);
      timerId = setInterval(() => {
        setRefreshCountdown((prev) => (prev > 1 ? prev - 1 : 10));
      }, 1000);

      intervalId = setInterval(() => {
        fetchData();
      }, 10000);
    }

    return () => {
      if (intervalId) clearInterval(intervalId);
      if (timerId) clearInterval(timerId);
    };
  }, [autoRefresh, isAuthenticated, fetchData]);

  // Action: Delete user (Real REST call)
  const handleDeleteUser = async (username) => {
    if (!window.confirm(`Are you sure you want to delete user "${username}"?`)) return;
    try {
      const res = await fetch(`/api/admin/users/${encodeURIComponent(username)}`, {
        method: 'DELETE',
        headers: { 'x-admin-secret': secretKey },
      });
      if (res.ok) {
        showToast(`User '${username}' deleted successfully`, 'success');
        fetchData();
      } else {
        showToast(`Failed to delete user '${username}'`, 'error');
      }
    } catch (err) {
      showToast('Error executing user deletion', 'error');
    }
  };

  // Action: Post announcement (Real REST call)
  const handlePostAnnouncement = async (textToPost) => {
    const text = textToPost || newAnnouncementText;
    if (!text.trim()) {
      showToast('Announcement message cannot be empty', 'error');
      return;
    }
    try {
      const res = await fetch('/api/admin/announcements', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'x-admin-secret': secretKey,
        },
        body: JSON.stringify({ message: text.trim() }),
      });
      if (res.ok) {
        showToast('Broadcast announcement published!', 'success');
        setNewAnnouncementText('');
        fetchData();
      } else {
        showToast('Failed to create announcement', 'error');
      }
    } catch (err) {
      showToast('Error posting announcement', 'error');
    }
  };

  // Action: Delete announcement (Real REST call)
  const handleDeleteAnnouncement = async (id) => {
    try {
      const res = await fetch(`/api/admin/announcements/${id}`, {
        method: 'DELETE',
        headers: { 'x-admin-secret': secretKey },
      });
      if (res.ok) {
        showToast(`Announcement #${id} deleted`, 'info');
        fetchData();
      } else {
        showToast('Failed to delete announcement', 'error');
      }
    } catch (err) {
      showToast('Error deleting announcement', 'error');
    }
  };

  // Filtered user search
  const filteredUsers = useMemo(() => {
    if (!searchQuery.trim()) return users;
    const q = searchQuery.toLowerCase();
    return users.filter(
      (u) =>
        u.username.toLowerCase().includes(q) ||
        (u.ed25519_pubkey && u.ed25519_pubkey.toLowerCase().includes(q))
    );
  }, [users, searchQuery]);

  // Filtered device search
  const filteredDevices = useMemo(() => {
    if (!searchQuery.trim()) return devices;
    const q = searchQuery.toLowerCase();
    return devices.filter(
      (d) =>
        (d.username && d.username.toLowerCase().includes(q)) ||
        (d.device_id && d.device_id.toLowerCase().includes(q))
    );
  }, [devices, searchQuery]);

  // Render Dedicated Login Screen if unauthenticated
  if (!isAuthenticated) {
    return (
      <div className="login-overlay">
        <div className="login-card">
          <div className="login-brand">
            <div className="login-icon">
              <Radio size={28} />
            </div>
            <div>
              <div className="login-title">
                <span>VEXTA</span> BRIDGE V2
              </div>
              <div className="login-sub">ADMIN CONSOLE AUTHENTICATION</div>
            </div>
          </div>

          {loginError && (
            <div className="login-error">
              <AlertCircle size={16} style={{ flexShrink: 0 }} />
              <span>{loginError}</span>
            </div>
          )}

          <form className="login-form" onSubmit={handleLoginSubmit}>
            <div className="input-group">
              <label>Admin Secret Token</label>
              <div style={{ position: 'relative' }}>
                <KeyRound
                  size={16}
                  style={{
                    position: 'absolute',
                    left: 12,
                    top: '50%',
                    transform: 'translateY(-50%)',
                    color: 'var(--text-3)',
                  }}
                />
                <input
                  type="password"
                  placeholder="Enter secret token..."
                  value={inputToken}
                  onChange={(e) => setInputToken(e.target.value)}
                  style={{ width: '100%', paddingLeft: 38 }}
                  autoFocus
                />
              </div>
            </div>

            <button
              type="submit"
              className="btn-primary"
              disabled={isAuthenticating}
              style={{ width: '100%', padding: 12 }}
            >
              {isAuthenticating ? (
                <>
                  <RefreshCw size={16} className="animate-spin" /> Verifying Token...
                </>
              ) : (
                <>
                  Sign In to Console <ArrowRight size={16} />
                </>
              )}
            </button>
          </form>

          <div
            style={{
              textAlign: 'center',
              fontSize: 11,
              color: 'var(--text-3)',
              fontFamily: 'IBM Plex Mono, monospace',
            }}
          >
            Vexta V2 High-Performance Signal Relay Bridge
          </div>
        </div>
      </div>
    );
  }

  // Render Full Admin Dashboard when Authenticated
  return (
    <div className="shell">
      {/* ── Top Navigation Bar ── */}
      <header className="topbar">
        <div className="brand">
          <div className="brand-icon">
            <Radio size={20} />
          </div>
          <div>
            <div className="brand-name">
              <span>VEXTA</span> BRIDGE V2
            </div>
            <div className="brand-sub">REACT ADMIN CONSOLE</div>
          </div>
          <div className="status-badge">
            <span className="status-dot"></span> ONLINE
          </div>
        </div>

        <div className="auth-section">
          {lastUpdated && (
            <div className="last-updated">
              <span>⏱</span>
              <span>Updated: {lastUpdated}</span>
            </div>
          )}
          {autoRefresh && (
            <div className="refresh-info">
              Refreshing in {refreshCountdown}s
            </div>
          )}
          <button
            className={`btn-ghost ${autoRefresh ? 'active' : ''}`}
            onClick={() => setAutoRefresh(!autoRefresh)}
          >
            <RefreshCw size={14} className={autoRefresh ? 'animate-spin' : ''} />
            {autoRefresh ? 'Auto (10s)' : 'Auto-Refresh'}
          </button>
          <button className="btn-secondary" onClick={() => fetchData()}>
            <RefreshCw size={14} /> Refresh
          </button>
          <button className="btn-danger" onClick={handleLogout}>
            <LogOut size={14} /> Logout
          </button>
        </div>
      </header>

      {/* ── Main Workspace ── */}
      <main className="main">
        {/* Telemetry Stat Cards */}
        <div className="stats-grid">
          <div className="stat-card green">
            <div className="stat-header">
              <div className="stat-icon green">
                <Activity size={20} />
              </div>
            </div>
            <div className="stat-label">Live WS Sessions</div>
            <div className="stat-value green">{stats.active_ws_sessions}</div>
            <div className="stat-sub">Active connections</div>
          </div>

          <div className="stat-card blue">
            <div className="stat-header">
              <div className="stat-icon blue">
                <Users size={20} />
              </div>
            </div>
            <div className="stat-label">Registered Accounts</div>
            <div className="stat-value blue">{stats.total_users}</div>
            <div className="stat-sub">Total users in DB</div>
          </div>

          <div className="stat-card amber">
            <div className="stat-header">
              <div className="stat-icon amber">
                <MessageSquare size={20} />
              </div>
            </div>
            <div className="stat-label">Queued Offline Msgs</div>
            <div className="stat-value amber">{stats.total_queued_offline_messages}</div>
            <div className="stat-sub">Pending delivery</div>
          </div>

          <div className="stat-card purple">
            <div className="stat-header">
              <div className="stat-icon purple">
                <Smartphone size={20} />
              </div>
            </div>
            <div className="stat-label">Registered Devices</div>
            <div className="stat-value purple">{stats.total_registered_devices}</div>
            <div className="stat-sub">Across all accounts</div>
          </div>

          <div className="stat-card amber">
            <div className="stat-header">
              <div className="stat-icon amber">
                <Megaphone size={20} />
              </div>
            </div>
            <div className="stat-label">Active Broadcasts</div>
            <div className="stat-value amber">{stats.total_announcements}</div>
            <div className="stat-sub">System notices</div>
          </div>
        </div>

        {/* Tab Selection Bar */}
        <div className="tabs">
          <button
            className={`tab-btn ${activeTab === 'users' ? 'active' : ''}`}
            onClick={() => setActiveTab('users')}
          >
            <Users size={15} /> User Accounts
            <span className="tab-count">{users.length}</span>
          </button>
          <button
            className={`tab-btn ${activeTab === 'devices' ? 'active' : ''}`}
            onClick={() => setActiveTab('devices')}
          >
            <Smartphone size={15} /> Registered Devices
            <span className="tab-count">{devices.length}</span>
          </button>
          <button
            className={`tab-btn ${activeTab === 'announcements' ? 'active' : ''}`}
            onClick={() => setActiveTab('announcements')}
          >
            <Megaphone size={15} /> Announcements
            <span className="tab-count">{announcements.length}</span>
          </button>
        </div>

        {/* ── Tab Content Panels ── */}
        <div className="panel">
          {/* Panel Header & Controls */}
          <div className="panel-header">
            <div className="panel-title">
              <div className="panel-title-icon">
                {activeTab === 'users' && <Users size={16} />}
                {activeTab === 'devices' && <Smartphone size={16} />}
                {activeTab === 'announcements' && <Megaphone size={16} />}
              </div>
              <span>
                {activeTab === 'users' && 'Account Registry & Public Keys'}
                {activeTab === 'devices' && 'Push Devices & Token Registrations'}
                {activeTab === 'announcements' && 'System Announcements & Broadcasts'}
              </span>
            </div>

            <div className="panel-actions">
              {(activeTab === 'users' || activeTab === 'devices') && (
                <div className="search-wrap">
                  <Search className="search-icon" size={14} />
                  <input
                    type="text"
                    placeholder="Search username or key..."
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                  />
                </div>
              )}
            </div>
          </div>

          <div className="panel-body">
            {/* 1. USERS TAB */}
            {activeTab === 'users' && (
              <div className="table-wrap">
                {filteredUsers.length === 0 ? (
                  <div className="empty-state">
                    <Users className="empty-state-icon" />
                    <div className="empty-state-text">No user accounts found</div>
                    <div className="empty-state-sub">
                      {searchQuery ? 'Try adjusting your search query' : 'Registered user accounts will appear here'}
                    </div>
                  </div>
                ) : (
                  <table>
                    <thead>
                      <tr>
                        <th>Username</th>
                        <th>Ed25519 Public Key</th>
                        <th>Created At</th>
                        <th style={{ textAlign: 'right' }}>Actions</th>
                      </tr>
                    </thead>
                    <tbody>
                      {filteredUsers.map((user) => (
                        <tr key={user.username}>
                          <td className="user-cell">@{user.username}</td>
                          <td>
                            <div
                              className="pubkey-cell"
                              title={user.ed25519_pubkey}
                              onClick={() => {
                                navigator.clipboard.writeText(user.ed25519_pubkey);
                                showToast(`Public key for @${user.username} copied to clipboard!`, 'success');
                              }}
                            >
                              <Key size={12} style={{ display: 'inline', marginRight: 4 }} />
                              {user.ed25519_pubkey || '—'}
                            </div>
                          </td>
                          <td className="td-muted">
                            {user.created_at
                              ? new Date(user.created_at * 1000).toLocaleString()
                              : '—'}
                          </td>
                          <td style={{ textAlign: 'right' }}>
                            <button
                              className="btn-danger"
                              onClick={() => handleDeleteUser(user.username)}
                            >
                              <Trash2 size={13} /> Delete
                            </button>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </div>
            )}

            {/* 2. DEVICES TAB */}
            {activeTab === 'devices' && (
              <div className="table-wrap">
                {filteredDevices.length === 0 ? (
                  <div className="empty-state">
                    <Smartphone className="empty-state-icon" />
                    <div className="empty-state-text">No registered devices</div>
                    <div className="empty-state-sub">
                      {searchQuery ? 'No device matching your search' : 'User devices will appear here once connected'}
                    </div>
                  </div>
                ) : (
                  <table>
                    <thead>
                      <tr>
                        <th>Owner</th>
                        <th>Device ID / Push Token</th>
                        <th>Platform</th>
                        <th>Last Active</th>
                      </tr>
                    </thead>
                    <tbody>
                      {filteredDevices.map((device, idx) => (
                        <tr key={device.device_id || idx}>
                          <td className="user-cell">@{device.username}</td>
                          <td className="td-mono">{device.device_id || '—'}</td>
                          <td>
                            <span className="chip chip-blue">
                              <Smartphone size={11} /> {device.platform || 'Generic'}
                            </span>
                          </td>
                          <td className="td-muted">
                            {device.last_active
                              ? new Date(device.last_active * 1000).toLocaleString()
                              : 'Recently'}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </div>
            )}

            {/* 3. ANNOUNCEMENTS TAB */}
            {activeTab === 'announcements' && (
              <div className="overview-grid">
                {/* Announcement List */}
                <div className="announcement-list">
                  {announcements.length === 0 ? (
                    <div className="empty-state">
                      <Megaphone className="empty-state-icon" />
                      <div className="empty-state-text">No system announcements posted</div>
                    </div>
                  ) : (
                    announcements.map((ann) => (
                      <div className="ann-item" key={ann.id}>
                        <div className="ann-meta">
                          <span className="chip chip-amber">
                            <Megaphone size={11} /> Notice #{ann.id}
                          </span>
                          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                            <span className="ann-id">
                              {ann.created_at
                                ? new Date(ann.created_at * 1000).toLocaleString()
                                : ''}
                            </span>
                            <button
                              className="btn-danger"
                              onClick={() => handleDeleteAnnouncement(ann.id)}
                            >
                              <Trash2 size={12} />
                            </button>
                          </div>
                        </div>
                        <div className="ann-text">
                          <MarkdownMessage content={ann.message} />
                        </div>
                      </div>
                    ))
                  )}
                </div>

                {/* Create Announcement Form */}
                <div className="panel" style={{ height: 'fit-content', background: 'var(--surface-2)' }}>
                  <div className="panel-header">
                    <div className="panel-title">
                      <PlusCircle size={15} /> Create Announcement
                    </div>
                  </div>
                  <div className="panel-body announcement-form">
                    <label style={{ fontSize: 12, color: 'var(--text-3)' }}>
                      Quick Templates:
                    </label>
                    <div className="template-chips">
                      <button
                        className="tpl-chip"
                        onClick={() =>
                          setNewAnnouncementText(
                            '⚠️ Scheduled maintenance window in 30 minutes. Relays may temporarily reconnect.'
                          )
                        }
                      >
                        🔧 Maintenance
                      </button>
                      <button
                        className="tpl-chip"
                        onClick={() =>
                          setNewAnnouncementText(
                            '🚀 Vexta Bridge V2 core relay upgraded. Zero-knowledge encryption active.'
                          )
                        }
                      >
                        ⚡ Upgrade
                      </button>
                      <button
                        className="tpl-chip"
                        onClick={() =>
                          setNewAnnouncementText(
                            'ℹ️ All system relays operating at nominal latency (<5ms).'
                          )
                        }
                      >
                        ✅ Nominal Status
                      </button>
                    </div>

                    <textarea
                      placeholder="Type broadcast message for all connected clients..."
                      value={newAnnouncementText}
                      onChange={(e) => setNewAnnouncementText(e.target.value)}
                    ></textarea>

                    <button
                      className="btn-primary"
                      onClick={() => handlePostAnnouncement()}
                    >
                      <Megaphone size={15} /> Publish Announcement
                    </button>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      </main>

      {/* ── Toast Notifications ── */}
      {toast && (
        <div className="toast-container">
          <div className={`toast ${toast.type}`}>
            {toast.type === 'success' && <CheckCircle2 size={16} />}
            {toast.type === 'error' && <AlertCircle size={16} />}
            {toast.type === 'info' && <Info size={16} />}
            <span>{toast.msg}</span>
          </div>
        </div>
      )}
    </div>
  );
}
