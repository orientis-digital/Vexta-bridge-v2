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
  ArrowRight,
  Database,
  Server,
  Clock,
  Unlock,
  Wifi,
  Power,
  Layers,
  HardDrive,
  LayoutDashboard,
  PieChart,
  TrendingUp,
  ArrowUpRight,
  Sparkles,
  Eye,
  Edit3,
  Copy,
  Filter,
  AlertTriangle,
  Check,
  Share2
} from 'lucide-react';

// Format bytes into human-readable MB / KB
function formatBytes(bytes) {
  if (!bytes || bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

// Format seconds into digital uptime string (e.g. 2h 15m 4s)
function formatUptime(seconds) {
  if (!seconds || seconds < 0) return '0s';
  const hrs = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;
  if (hrs > 0) return `${hrs}h ${mins}m ${secs}s`;
  if (mins > 0) return `${mins}m ${secs}s`;
  return `${secs}s`;
}

// Category badge generator for announcements
function getAnnouncementBadge(text) {
  if (!text) return <span className="chip chip-green"><Info size={11} /> General Notice</span>;
  const lower = text.toLowerCase();
  if (text.includes('[EMERGENCY]') || lower.includes('🚨') || lower.includes('emergency') || lower.includes('outage')) {
    return <span className="chip chip-red"><AlertTriangle size={11} /> Emergency Outage</span>;
  }
  if (text.includes('[MAINTENANCE]') || lower.includes('🔧') || lower.includes('maintenance') || lower.includes('scheduled')) {
    return <span className="chip chip-amber"><AlertCircle size={11} /> Maintenance</span>;
  }
  if (text.includes('[UPDATE]') || lower.includes('🚀') || lower.includes('upgrade') || lower.includes('feature')) {
    return <span className="chip chip-blue"><Sparkles size={11} /> Feature Update</span>;
  }
  return <span className="chip chip-green"><Info size={11} /> General Notice</span>;
}

// Lightweight Markdown Renderer Component
function MarkdownMessage({ content }) {
  if (!content) return null;

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

        const lines = part.split('\n');
        return (
          <React.Fragment key={index}>
            {lines.map((line, lineIdx) => {
              const trimmed = line.trim();
              if (!trimmed) return <div key={lineIdx} style={{ height: 6 }} />;

              if (trimmed.startsWith('### ')) {
                return <h4 key={lineIdx} className="md-h3">{formatInline(trimmed.slice(4))}</h4>;
              }
              if (trimmed.startsWith('## ')) {
                return <h3 key={lineIdx} className="md-h2">{formatInline(trimmed.slice(3))}</h3>;
              }
              if (trimmed.startsWith('# ')) {
                return <h2 key={lineIdx} className="md-h1">{formatInline(trimmed.slice(2))}</h2>;
              }

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

// Inline formatting helper
function formatInline(text) {
  if (!text) return '';
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

  const [activeTab, setActiveTab] = useState('overview');
  const [searchQuery, setSearchQuery] = useState('');
  const [autoRefresh, setAutoRefresh] = useState(false);
  const [refreshCountdown, setRefreshCountdown] = useState(10);
  const [lastUpdated, setLastUpdated] = useState(null);

  // Expanded Telemetry & Traffic Data States
  const [stats, setStats] = useState({
    active_ws_sessions: 0,
    total_users: 0,
    total_queued_offline_messages: 0,
    total_registered_devices: 0,
    total_announcements: 0,
    database_size_bytes: 0,
    wal_size_bytes: 0,
    provisioned_users: 0,
    locked_users: 0,
    users_with_vault: 0,
    users_with_prekey: 0,
    users_with_offline_msgs: 0,
    total_messages_relayed: 0,
    total_bytes_relayed: 0,
    uptime_seconds: 0,
  });

  const [users, setUsers] = useState([]);
  const [devices, setDevices] = useState([]);
  const [announcements, setAnnouncements] = useState([]);
  const [sessions, setSessions] = useState([]);
  const [offlineSummary, setOfflineSummary] = useState([]);

  // New Admin Feature States: Firewall, Maintenance, Audit, Analytics, DB Health
  const [bannedIps, setBannedIps] = useState([]);
  const [auditLogs, setAuditLogs] = useState([]);
  const [topUsers, setTopUsers] = useState([]);
  const [dbHealth, setDbHealth] = useState(null);
  const [banIpInput, setBanIpInput] = useState('');
  const [banReasonInput, setBanReasonInput] = useState('');

  // Enhanced Announcement Composer States
  const [newAnnouncementText, setNewAnnouncementText] = useState('');
  const [announcementCategory, setAnnouncementCategory] = useState('INFO');
  const [announcementComposerTab, setAnnouncementComposerTab] = useState('edit');
  const [announcementFilter, setAnnouncementFilter] = useState('ALL');

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

      // 5. Active Sessions REST API
      const resSessions = await fetch('/api/admin/sessions', { headers });
      if (resSessions.ok) {
        const sessionData = await resSessions.json();
        setSessions(sessionData.active_sessions || []);
      }

      // 6. Offline Messages Summary REST API
      const resOffline = await fetch('/api/admin/offline-messages/summary', { headers });
      if (resOffline.ok) {
        const offlineData = await resOffline.json();
        setOfflineSummary(offlineData);
      }

      // 7. Banned IPs REST API
      const resBans = await fetch('/api/admin/banned-ips', { headers });
      if (resBans.ok) setBannedIps(await resBans.json());

      // 8. Audit Logs REST API
      const resAudit = await fetch('/api/admin/audit-logs', { headers });
      if (resAudit.ok) setAuditLogs(await resAudit.json());

      // 9. Top User Analytics REST API
      const resTop = await fetch('/api/admin/analytics/top-users', { headers });
      if (resTop.ok) setTopUsers(await resTop.json());

      // 10. Database Health REST API
      const resHealth = await fetch('/api/admin/system/db-health', { headers });
      if (resHealth.ok) setDbHealth(await resHealth.json());

      setLastUpdated(new Date().toLocaleTimeString());
      return true;
    } catch (err) {
      console.error('Failed connecting to bridge backend:', err);
      setLoginError('Unable to connect to Rust backend server on port 8000.');
      return false;
    }
  }, [secretKey]);

  // Action Handlers for IP Banning, Maintenance, and DB Vacuuming
  const handleBanIp = async (e) => {
    e.preventDefault();
    if (!banIpInput.trim()) return;
    try {
      const res = await fetch('/api/admin/banned-ips', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'x-admin-secret': secretKey },
        body: JSON.stringify({ ip: banIpInput.trim(), reason: banReasonInput.trim() || 'Banned by admin' })
      });
      if (res.ok) {
        showToast(`Banned IP ${banIpInput.trim()}`, 'success');
        setBanIpInput('');
        setBanReasonInput('');
        fetchData();
      } else {
        showToast('Failed to ban IP', 'error');
      }
    } catch (err) {
      showToast('Error banning IP', 'error');
    }
  };

  const handleUnbanIp = async (ip) => {
    try {
      const res = await fetch(`/api/admin/banned-ips/${encodeURIComponent(ip)}`, {
        method: 'DELETE',
        headers: { 'x-admin-secret': secretKey }
      });
      if (res.ok) {
        showToast(`Unbanned IP ${ip}`, 'info');
        fetchData();
      }
    } catch (err) {
      showToast('Error unbanning IP', 'error');
    }
  };

  const handleToggleMaintenance = async (enable) => {
    const endpoint = enable ? '/api/admin/maintenance/enable' : '/api/admin/maintenance/disable';
    try {
      const res = await fetch(endpoint, {
        method: 'POST',
        headers: { 'x-admin-secret': secretKey }
      });
      if (res.ok) {
        showToast(enable ? 'Emergency maintenance enabled 🚨' : 'Maintenance disabled. Bridge online 🟢', 'warning');
        fetchData();
      }
    } catch (err) {
      showToast('Error toggling maintenance mode', 'error');
    }
  };

  const handleVacuumDb = async () => {
    try {
      const res = await fetch('/api/admin/system/vacuum', {
        method: 'POST',
        headers: { 'x-admin-secret': secretKey }
      });
      if (res.ok) {
        showToast('SQLite WAL truncated & database vacuumed 🧹', 'success');
        fetchData();
      }
    } catch (err) {
      showToast('Error vacuuming database', 'error');
    }
  };

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

  // Action: Delete user
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

  // Action: Unlock User Account
  const handleUnlockUser = async (username) => {
    try {
      const res = await fetch(`/api/admin/users/${encodeURIComponent(username)}/unlock`, {
        method: 'POST',
        headers: { 'x-admin-secret': secretKey },
      });
      if (res.ok) {
        showToast(`Account @${username} unlocked successfully!`, 'success');
        fetchData();
      } else {
        showToast(`Failed to unlock account @${username}`, 'error');
      }
    } catch (err) {
      showToast('Error unlocking user account', 'error');
    }
  };

  // Action: Disconnect Live WS Session
  const handleDisconnectSession = async (username) => {
    if (!window.confirm(`Force disconnect active WebSocket connection for @${username}?`)) return;
    try {
      const res = await fetch(`/api/admin/sessions/${encodeURIComponent(username)}`, {
        method: 'DELETE',
        headers: { 'x-admin-secret': secretKey },
      });
      if (res.ok) {
        showToast(`WebSocket connection for @${username} terminated`, 'info');
        fetchData();
      } else {
        showToast(`Failed to disconnect @${username}`, 'error');
      }
    } catch (err) {
      showToast('Error terminating WebSocket session', 'error');
    }
  };

  // Action: Revoke Device
  const handleRevokeDevice = async (username, hardware_hash) => {
    if (!window.confirm(`Revoke device registration '${hardware_hash}' for user @${username}?`)) return;
    try {
      const res = await fetch(`/api/admin/devices/${encodeURIComponent(username)}/${encodeURIComponent(hardware_hash)}`, {
        method: 'DELETE',
        headers: { 'x-admin-secret': secretKey },
      });
      if (res.ok) {
        showToast(`Device '${hardware_hash}' revoked from @${username}`, 'success');
        fetchData();
      } else {
        showToast('Failed to revoke device', 'error');
      }
    } catch (err) {
      showToast('Error revoking device', 'error');
    }
  };

  // Action: Purge Stale Offline Messages
  const handlePurgeStaleOfflineMsgs = async (days = 30) => {
    if (!window.confirm(`Purge all queued offline messages older than ${days} days?`)) return;
    try {
      const res = await fetch('/api/admin/offline-messages/purge', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'x-admin-secret': secretKey,
        },
        body: JSON.stringify({ older_than_days: days }),
      });
      if (res.ok) {
        const data = await res.json();
        showToast(`Purged ${data.deleted_count || 0} offline messages older than ${days} days!`, 'success');
        fetchData();
      } else {
        showToast('Failed to purge offline messages', 'error');
      }
    } catch (err) {
      showToast('Error purging offline messages', 'error');
    }
  };

  // Action: Post announcement with category tag prefix
  const handlePostAnnouncement = async (textToPost) => {
    let rawText = textToPost || newAnnouncementText;
    if (!rawText.trim()) {
      showToast('Announcement message cannot be empty', 'error');
      return;
    }

    // Attach category prefix if not already present
    let textWithTag = rawText.trim();
    if (!textWithTag.startsWith('[')) {
      textWithTag = `[${announcementCategory}] ${textWithTag}`;
    }

    try {
      const res = await fetch('/api/admin/announcements', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'x-admin-secret': secretKey,
        },
        body: JSON.stringify({ message: textWithTag }),
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

  // Action: Delete announcement
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

  // Formatting helpers for rich markdown toolbar
  const handleFormatInsert = (prefix, suffix = '') => {
    setNewAnnouncementText((prev) => `${prev}${prefix}${suffix}`);
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
        (d.device_id && d.device_id.toLowerCase().includes(q)) ||
        (d.hardware_hash && d.hardware_hash.toLowerCase().includes(q))
    );
  }, [devices, searchQuery]);

  // Filtered sessions search
  const filteredSessions = useMemo(() => {
    if (!searchQuery.trim()) return sessions;
    const q = searchQuery.toLowerCase();
    return sessions.filter((s) => s.toLowerCase().includes(q));
  }, [sessions, searchQuery]);

  // Filtered announcements
  const filteredAnnouncements = useMemo(() => {
    return announcements.filter((ann) => {
      if (announcementFilter === 'ALL') return true;
      const lower = ann.message.toLowerCase();
      if (announcementFilter === 'EMERGENCY') return ann.message.includes('[EMERGENCY]') || lower.includes('emergency') || lower.includes('outage');
      if (announcementFilter === 'MAINTENANCE') return ann.message.includes('[MAINTENANCE]') || lower.includes('maintenance');
      if (announcementFilter === 'UPDATE') return ann.message.includes('[UPDATE]') || lower.includes('upgrade') || lower.includes('feature');
      if (announcementFilter === 'INFO') return ann.message.includes('[INFO]') || (!ann.message.includes('[EMERGENCY]') && !ann.message.includes('[MAINTENANCE]') && !ann.message.includes('[UPDATE]'));
      return true;
    });
  }, [announcements, announcementFilter]);

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
              <div className="login-sub">Vexta Bridge V2 - v0.0.1</div>
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

  // Calculate percentages for Overview widgets
  const provisionedPercent = stats.total_users ? Math.round((stats.provisioned_users / stats.total_users) * 100) : 0;
  const trafficSpeed = stats.uptime_seconds > 0 ? (stats.total_bytes_relayed / stats.uptime_seconds).toFixed(1) : 0;

  // Render Full Admin Dashboard with Header, Sidebar, and Main Content Area
  return (
    <div className="shell">
      {/* ── 1. Top Header ── */}
      <header className="topbar">
        <div className="brand">
          <div className="brand-icon">
            <Radio size={20} />
          </div>
          <div>
            <div className="brand-name">
              <span>VEXTA</span> BRIDGE V2
            </div>
            <div className="brand-sub">Vexta Bridge V2 - v0.0.1</div>
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

      {/* ── 2. Layout Container (Sidebar + Main Content Area) ── */}
      <div className="layout-container">
        {/* Left Sidebar */}
        <aside className="sidebar">
          <div className="sidebar-menu">
            <div className="sidebar-label">Navigation</div>

            <button
              className={`sidebar-btn ${activeTab === 'overview' ? 'active' : ''}`}
              onClick={() => setActiveTab('overview')}
            >
              <div className="sidebar-btn-content">
                <LayoutDashboard size={16} />
                <span>Overview</span>
              </div>
              <Sparkles size={12} style={{ color: 'var(--accent)' }} />
            </button>

            <button
              className={`sidebar-btn ${activeTab === 'users' ? 'active' : ''}`}
              onClick={() => setActiveTab('users')}
            >
              <div className="sidebar-btn-content">
                <Users size={16} />
                <span>User Accounts</span>
              </div>
              <span className="sidebar-count">{users.length}</span>
            </button>

            <button
              className={`sidebar-btn ${activeTab === 'sessions' ? 'active' : ''}`}
              onClick={() => setActiveTab('sessions')}
            >
              <div className="sidebar-btn-content">
                <Wifi size={16} />
                <span>Live WS Sessions</span>
              </div>
              <span className="sidebar-count">{sessions.length}</span>
            </button>

            <button
              className={`sidebar-btn ${activeTab === 'offline' ? 'active' : ''}`}
              onClick={() => setActiveTab('offline')}
            >
              <div className="sidebar-btn-content">
                <MessageSquare size={16} />
                <span>Offline Queues</span>
              </div>
              <span className="sidebar-count">{offlineSummary.length}</span>
            </button>

            <button
              className={`sidebar-btn ${activeTab === 'devices' ? 'active' : ''}`}
              onClick={() => setActiveTab('devices')}
            >
              <div className="sidebar-btn-content">
                <Smartphone size={16} />
                <span>Devices</span>
              </div>
              <span className="sidebar-count">{devices.length}</span>
            </button>

            <button
              className={`sidebar-btn ${activeTab === 'announcements' ? 'active' : ''}`}
              onClick={() => setActiveTab('announcements')}
            >
              <div className="sidebar-btn-content">
                <Megaphone size={16} />
                <span>Announcements</span>
              </div>
              <span className="sidebar-count">{announcements.length}</span>
            </button>

            <div className="sidebar-label" style={{ marginTop: 12 }}>Security & Management</div>

            <button
              className={`sidebar-btn ${activeTab === 'firewall' ? 'active' : ''}`}
              onClick={() => setActiveTab('firewall')}
            >
              <div className="sidebar-btn-content">
                <Shield size={16} style={{ color: 'var(--danger)' }} />
                <span>IP Firewall</span>
              </div>
              <span className="sidebar-count" style={{ background: 'var(--danger-dim)', color: 'var(--danger)' }}>{bannedIps.length}</span>
            </button>

            <button
              className={`sidebar-btn ${activeTab === 'maintenance' ? 'active' : ''}`}
              onClick={() => setActiveTab('maintenance')}
            >
              <div className="sidebar-btn-content">
                <AlertTriangle size={16} style={{ color: 'var(--amber)' }} />
                <span>Maintenance</span>
              </div>
              {stats.maintenance_mode && <span className="sidebar-count" style={{ background: 'var(--danger)', color: '#fff' }}>ON</span>}
            </button>

            <button
              className={`sidebar-btn ${activeTab === 'audit' ? 'active' : ''}`}
              onClick={() => setActiveTab('audit')}
            >
              <div className="sidebar-btn-content">
                <Clock size={16} />
                <span>Audit Logs</span>
              </div>
              <span className="sidebar-count">{auditLogs.length}</span>
            </button>

            <button
              className={`sidebar-btn ${activeTab === 'analytics' ? 'active' : ''}`}
              onClick={() => setActiveTab('analytics')}
            >
              <div className="sidebar-btn-content">
                <TrendingUp size={16} style={{ color: 'var(--accent)' }} />
                <span>Traffic Analytics</span>
              </div>
            </button>

            <div className="sidebar-label" style={{ marginTop: 12 }}>Telemetry</div>

            <button
              className={`sidebar-btn ${activeTab === 'health' ? 'active' : ''}`}
              onClick={() => setActiveTab('health')}
            >
              <div className="sidebar-btn-content">
                <HardDrive size={16} />
                <span>Storage & Vacuum</span>
              </div>
            </button>
          </div>

          {/* Sidebar Footer Engine Summary */}
          <div className="sidebar-footer">
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 12, color: 'var(--text-2)' }}>
              <Server size={14} style={{ color: 'var(--accent)' }} />
              <strong>Rust Engine Status</strong>
            </div>
            <div style={{ fontSize: 11, color: 'var(--text-3)' }}>
              Uptime: <span style={{ color: 'var(--text-1)', fontFamily: 'IBM Plex Mono, monospace' }}>{formatUptime(stats.uptime_seconds)}</span>
            </div>
            <div style={{ fontSize: 11, color: 'var(--text-3)' }}>
              SQLite Size: <span style={{ color: 'var(--text-1)', fontFamily: 'IBM Plex Mono, monospace' }}>{formatBytes(stats.database_size_bytes)}</span>
            </div>
          </div>
        </aside>

        {/* ── 3. Main Content Area ── */}
        <main className="main-content">
          {/* Telemetry Stat Cards Grid — Rendered ONLY on Overview Page */}
          {activeTab === 'overview' && (
            <div className="stats-grid">
              {/* WS Sessions Card */}
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

              {/* User Accounts Card */}
              <div className="stat-card blue">
                <div className="stat-header">
                  <div className="stat-icon blue">
                    <Users size={20} />
                  </div>
                </div>
                <div className="stat-label">Registered Accounts</div>
                <div className="stat-value blue">{stats.total_users}</div>
                <div className="stat-sub">
                  {stats.provisioned_users || 0} provisioned ({provisionedPercent}%)
                </div>
              </div>

              {/* Queued Offline Messages Card */}
              <div className="stat-card amber">
                <div className="stat-header">
                  <div className="stat-icon amber">
                    <MessageSquare size={20} />
                  </div>
                </div>
                <div className="stat-label">Queued Offline Msgs</div>
                <div className="stat-value amber">{stats.total_queued_offline_messages}</div>
                <div className="stat-sub">Across {stats.users_with_offline_msgs || 0} recipient queues</div>
              </div>

              {/* Traffic Statistics Card */}
              <div className="stat-card green">
                <div className="stat-header">
                  <div className="stat-icon green">
                    <TrendingUp size={20} />
                  </div>
                </div>
                <div className="stat-label">Relayed Traffic</div>
                <div className="stat-value green">{stats.total_messages_relayed || 0}</div>
                <div className="stat-sub">
                  {formatBytes(stats.total_bytes_relayed)} total payload
                </div>
              </div>

              {/* Database & Storage Telemetry Card */}
              <div className="stat-card slate">
                <div className="stat-header">
                  <div className="stat-icon slate">
                    <Database size={20} />
                  </div>
                </div>
                <div className="stat-label">SQLite Database Size</div>
                <div className="stat-value slate">{formatBytes(stats.database_size_bytes)}</div>
                <div className="stat-sub">
                  WAL size: {formatBytes(stats.wal_size_bytes)}
                </div>
              </div>

              {/* Registered Devices Card */}
              <div className="stat-card purple">
                <div className="stat-header">
                  <div className="stat-icon purple">
                    <Smartphone size={20} />
                  </div>
                </div>
                <div className="stat-label">Registered Devices</div>
                <div className="stat-value purple">{stats.total_registered_devices}</div>
                <div className="stat-sub">Active device tokens</div>
              </div>
            </div>
          )}

          {/* ── Active Panel Container ── */}
          <div className="panel">
            {/* Panel Header & Controls */}
            <div className="panel-header">
              <div className="panel-title">
                <div className="panel-title-icon">
                  {activeTab === 'overview' && <LayoutDashboard size={16} />}
                  {activeTab === 'users' && <Users size={16} />}
                  {activeTab === 'sessions' && <Wifi size={16} />}
                  {activeTab === 'offline' && <MessageSquare size={16} />}
                  {activeTab === 'devices' && <Smartphone size={16} />}
                  {activeTab === 'announcements' && <Megaphone size={16} />}
                  {activeTab === 'health' && <HardDrive size={16} />}
                </div>
                <span>
                  {activeTab === 'overview' && 'System Overview & Real-Time Relay Telemetry'}
                  {activeTab === 'users' && 'Account Registry & Cryptographic Identity'}
                  {activeTab === 'sessions' && 'Active WebSocket Relay Sessions (In-Memory Routing)'}
                  {activeTab === 'offline' && 'Offline Message Queue Breakdown (Store-and-Forward)'}
                  {activeTab === 'devices' && 'Push Devices & Hardware Registrations'}
                  {activeTab === 'announcements' && 'Broadcast Announcement Center & Interactive Composer'}
                  {activeTab === 'health' && 'SQLite WAL Storage & Server Process Telemetry'}
                </span>
              </div>

              <div className="panel-actions">
                {(activeTab === 'users' || activeTab === 'devices' || activeTab === 'sessions') && (
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

                {activeTab === 'offline' && (
                  <button
                    className="btn-warning"
                    onClick={() => handlePurgeStaleOfflineMsgs(30)}
                  >
                    <Trash2 size={13} /> Purge Stale Msgs (&gt;30 Days)
                  </button>
                )}
              </div>
            </div>

            <div className="panel-body">
              {/* 0. OVERVIEW TAB */}
              {activeTab === 'overview' && (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
                  {/* System Status Banner */}
                  <div
                    style={{
                      background: 'linear-gradient(135deg, rgba(57, 255, 20, 0.08) 0%, rgba(59, 130, 246, 0.08) 100%)',
                      border: '1px solid var(--accent-border)',
                      borderRadius: 'var(--radius-lg)',
                      padding: '24px 28px',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                      flexWrap: 'wrap',
                      gap: 20,
                    }}
                  >
                    <div>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 6 }}>
                        <h2 style={{ fontSize: 20, fontWeight: 700, letterSpacing: '-0.5px' }}>
                          Vexta Bridge Kernel Operational
                        </h2>
                        <span className="chip chip-green">
                          <Activity size={12} /> &lt; 0.2ms Routing
                        </span>
                      </div>
                      <p style={{ color: 'var(--text-2)', fontSize: 13 }}>
                        Zero-trust, blind store-and-forward relay node powered by Rust + Axum 0.7 + SQLite WAL mode.
                      </p>
                    </div>

                    <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
                      <button className="btn-secondary" onClick={() => setActiveTab('sessions')}>
                        <Wifi size={14} /> View {sessions.length} Active Sessions
                      </button>
                      <button className="btn-primary" onClick={() => setActiveTab('announcements')}>
                        <Megaphone size={14} /> Post Announcement
                      </button>
                    </div>
                  </div>

                  {/* Operational Ratios & Traffic Grid */}
                  <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(300px, 1fr))', gap: 20 }}>
                    {/* Traffic & Throughput Widget */}
                    <div className="panel" style={{ background: 'var(--surface-2)' }}>
                      <div className="panel-header">
                        <div className="panel-title">
                          <TrendingUp size={16} /> Relay Traffic & Throughput
                        </div>
                      </div>
                      <div className="panel-body" style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
                        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 13 }}>
                          <span style={{ color: 'var(--text-2)' }}>Messages Relayed</span>
                          <strong style={{ color: 'var(--accent)', fontFamily: 'IBM Plex Mono, monospace' }}>
                            {stats.total_messages_relayed || 0} msgs
                          </strong>
                        </div>
                        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 13 }}>
                          <span style={{ color: 'var(--text-2)' }}>Total Data Transferred</span>
                          <strong style={{ fontFamily: 'IBM Plex Mono, monospace' }}>
                            {formatBytes(stats.total_bytes_relayed)}
                          </strong>
                        </div>
                        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 11, color: 'var(--text-3)' }}>
                          <span>Avg Speed: {trafficSpeed} B/s</span>
                          <span>Server Uptime: {formatUptime(stats.uptime_seconds)}</span>
                        </div>
                      </div>
                    </div>

                    {/* User Provisioning Widget */}
                    <div className="panel" style={{ background: 'var(--surface-2)' }}>
                      <div className="panel-header">
                        <div className="panel-title">
                          <Shield size={16} /> User Provisioning Status
                        </div>
                      </div>
                      <div className="panel-body" style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
                        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 13 }}>
                          <span style={{ color: 'var(--text-2)' }}>Provisioned Roster Ratio</span>
                          <strong>{stats.provisioned_users || 0} / {stats.total_users || 0} ({provisionedPercent}%)</strong>
                        </div>
                        {/* Progress Bar */}
                        <div style={{ height: 8, background: 'var(--surface-3)', borderRadius: 4, overflow: 'hidden' }}>
                          <div
                            style={{
                              width: `${provisionedPercent}%`,
                              height: '100%',
                              background: 'var(--accent)',
                              transition: 'width 0.4s ease',
                            }}
                          />
                        </div>
                        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 11, color: 'var(--text-3)' }}>
                          <span>Vault Backups: {stats.users_with_vault || 0}</span>
                          <span>Pre-key Bundles: {stats.users_with_prekey || 0}</span>
                        </div>
                      </div>
                    </div>

                    {/* Offline Queue Ratio Widget */}
                    <div className="panel" style={{ background: 'var(--surface-2)' }}>
                      <div className="panel-header">
                        <div className="panel-title">
                          <MessageSquare size={16} /> Store-and-Forward Queue Health
                        </div>
                      </div>
                      <div className="panel-body" style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
                        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 13 }}>
                          <span style={{ color: 'var(--text-2)' }}>Pending Offline Ciphertexts</span>
                          <strong style={{ color: 'var(--amber)' }}>{stats.total_queued_offline_messages || 0} msgs</strong>
                        </div>
                        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 11, color: 'var(--text-3)' }}>
                          <span>Active Queued Recipients: {stats.users_with_offline_msgs || 0}</span>
                          <span
                            style={{ color: 'var(--accent)', cursor: 'pointer' }}
                            onClick={() => setActiveTab('offline')}
                          >
                            Inspect Queues &rarr;
                          </span>
                        </div>
                      </div>
                    </div>
                  </div>

                  {/* Active Sessions & Recent Announcements Quick Preview */}
                  <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(360px, 1fr))', gap: 20 }}>
                    {/* Active WS Sessions Preview */}
                    <div className="panel" style={{ background: 'var(--surface-2)' }}>
                      <div className="panel-header">
                        <div className="panel-title">
                          <Wifi size={16} /> Connected Relay Routers ({sessions.length})
                        </div>
                        <button className="btn-ghost" onClick={() => setActiveTab('sessions')} style={{ padding: '4px 8px', fontSize: 11 }}>
                          View All <ArrowUpRight size={12} />
                        </button>
                      </div>
                      <div className="panel-body">
                        {sessions.length === 0 ? (
                          <div style={{ color: 'var(--text-3)', fontSize: 12, textAlign: 'center', padding: 20 }}>
                            No active WebSocket sessions connected right now.
                          </div>
                        ) : (
                          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                            {sessions.slice(0, 5).map((u) => (
                              <div
                                key={u}
                                style={{
                                  display: 'flex',
                                  alignItems: 'center',
                                  justifyContent: 'space-between',
                                  padding: '8px 12px',
                                  background: 'var(--surface-3)',
                                  borderRadius: 'var(--radius-sm)',
                                  border: '1px solid var(--panel-border)',
                                }}
                              >
                                <span style={{ fontFamily: 'IBM Plex Mono, monospace', fontSize: 12, color: 'var(--accent)' }}>
                                  @{u}
                                </span>
                                <button
                                  className="btn-danger"
                                  onClick={() => handleDisconnectSession(u)}
                                  style={{ padding: '2px 8px', fontSize: 11 }}
                                >
                                  Disconnect
                                </button>
                              </div>
                            ))}
                          </div>
                        )}
                      </div>
                    </div>

                    {/* Recent System Announcements Preview */}
                    <div className="panel" style={{ background: 'var(--surface-2)' }}>
                      <div className="panel-header">
                        <div className="panel-title">
                          <Megaphone size={16} /> Broadcast Notices ({announcements.length})
                        </div>
                        <button className="btn-ghost" onClick={() => setActiveTab('announcements')} style={{ padding: '4px 8px', fontSize: 11 }}>
                          Manage <ArrowUpRight size={12} />
                        </button>
                      </div>
                      <div className="panel-body">
                        {announcements.length === 0 ? (
                          <div style={{ color: 'var(--text-3)', fontSize: 12, textAlign: 'center', padding: 20 }}>
                            No system announcements posted yet.
                          </div>
                        ) : (
                          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
                            {announcements.slice(0, 3).map((ann) => (
                              <div
                                key={ann.id}
                                style={{
                                  padding: '10px 12px',
                                  background: 'var(--surface-3)',
                                  borderRadius: 'var(--radius-sm)',
                                  border: '1px solid var(--panel-border)',
                                  fontSize: 12,
                                }}
                              >
                                <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4, color: 'var(--text-3)', fontSize: 11 }}>
                                  {getAnnouncementBadge(ann.message)}
                                  <span>{ann.created_at ? new Date(ann.created_at * 1000).toLocaleTimeString() : ''}</span>
                                </div>
                                <div style={{ color: 'var(--text-1)' }}>
                                  <MarkdownMessage content={ann.message} />
                                </div>
                              </div>
                            ))}
                          </div>
                        )}
                      </div>
                    </div>
                  </div>
                </div>
              )}

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
                          <th>Status Badges</th>
                          <th>Created At</th>
                          <th style={{ textAlign: 'right' }}>Actions</th>
                        </tr>
                      </thead>
                      <tbody>
                        {filteredUsers.map((user) => {
                          const isLocked = (user.locked_until && user.locked_until > Math.floor(Date.now() / 1000)) || user.auth_attempts >= 5;
                          return (
                            <tr key={user.username}>
                              <td className="user-cell">@{user.username}</td>
                              <td>
                                <div
                                  className="pubkey-cell"
                                  title={user.ed25519_pubkey}
                                  onClick={() => {
                                    navigator.clipboard.writeText(user.ed25519_pubkey);
                                    showToast(`Public key for @${user.username} copied!`, 'success');
                                  }}
                                >
                                  <Key size={12} style={{ display: 'inline', marginRight: 4 }} />
                                  {user.ed25519_pubkey || '—'}
                                </div>
                              </td>
                              <td>
                                <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
                                  {user.is_provisioned ? (
                                    <span className="chip chip-green">Provisioned</span>
                                  ) : (
                                    <span className="chip chip-slate">Unprovisioned</span>
                                  )}
                                  {user.encrypted_vault && (
                                    <span className="chip chip-blue">Vault</span>
                                  )}
                                  {user.pre_key && (
                                    <span className="chip chip-purple">Pre-key</span>
                                  )}
                                  {isLocked && (
                                    <span className="chip chip-red">Locked ({user.auth_attempts} attempts)</span>
                                  )}
                                </div>
                              </td>
                              <td className="td-muted">
                                {user.created_at
                                  ? new Date(user.created_at * 1000).toLocaleString()
                                  : '—'}
                              </td>
                              <td style={{ textAlign: 'right' }}>
                                <div style={{ display: 'inline-flex', gap: 6 }}>
                                  {isLocked && (
                                    <button
                                      className="btn-warning"
                                      onClick={() => handleUnlockUser(user.username)}
                                      title="Clear auth attempts and unlock account"
                                    >
                                      <Unlock size={13} /> Unlock
                                    </button>
                                  )}
                                  <button
                                    className="btn-danger"
                                    onClick={() => handleDeleteUser(user.username)}
                                  >
                                    <Trash2 size={13} /> Delete
                                  </button>
                                </div>
                              </td>
                            </tr>
                          );
                        })}
                      </tbody>
                    </table>
                  )}
                </div>
              )}

              {/* 2. LIVE WS SESSIONS TAB */}
              {activeTab === 'sessions' && (
                <div className="table-wrap">
                  {filteredSessions.length === 0 ? (
                    <div className="empty-state">
                      <Wifi className="empty-state-icon" />
                      <div className="empty-state-text">No active WebSocket connections</div>
                      <div className="empty-state-sub">
                        {searchQuery ? 'No active session matching search' : 'Connected clients will appear here in real time'}
                      </div>
                    </div>
                  ) : (
                    <table>
                      <thead>
                        <tr>
                          <th>Username</th>
                          <th>Connection Type</th>
                          <th>Routing Status</th>
                          <th style={{ textAlign: 'right' }}>Actions</th>
                        </tr>
                      </thead>
                      <tbody>
                        {filteredSessions.map((sessionUsername) => (
                          <tr key={sessionUsername}>
                            <td className="user-cell">@{sessionUsername}</td>
                            <td>
                              <span className="chip chip-blue">
                                <Wifi size={11} /> WebSocket (Axum Relay)
                              </span>
                            </td>
                            <td>
                              <span className="chip chip-green">
                                <Activity size={11} /> ACTIVE ROUTER
                              </span>
                            </td>
                            <td style={{ textAlign: 'right' }}>
                              <button
                                className="btn-danger"
                                onClick={() => handleDisconnectSession(sessionUsername)}
                              >
                                <Power size={13} /> Disconnect
                              </button>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  )}
                </div>
              )}

              {/* 3. OFFLINE QUEUES TAB */}
              {activeTab === 'offline' && (
                <div className="table-wrap">
                  {offlineSummary.length === 0 ? (
                    <div className="empty-state">
                      <MessageSquare className="empty-state-icon" />
                      <div className="empty-state-text">No queued offline messages</div>
                      <div className="empty-state-sub">All messages have been delivered to active clients!</div>
                    </div>
                  ) : (
                    <table>
                      <thead>
                        <tr>
                          <th>Recipient User</th>
                          <th>Pending Messages</th>
                          <th>Oldest Message</th>
                          <th>Latest Message</th>
                          <th>Queue Status</th>
                        </tr>
                      </thead>
                      <tbody>
                        {offlineSummary.map((item) => (
                          <tr key={item.recipient}>
                            <td className="user-cell">@{item.recipient}</td>
                            <td className="td-mono" style={{ fontWeight: 600, color: 'var(--amber)' }}>
                              {item.message_count} msgs
                            </td>
                            <td className="td-muted">
                              {item.oldest_timestamp
                                ? new Date(item.oldest_timestamp * 1000).toLocaleString()
                                : '—'}
                            </td>
                            <td className="td-muted">
                              {item.newest_timestamp
                                ? new Date(item.newest_timestamp * 1000).toLocaleString()
                                : '—'}
                            </td>
                            <td>
                              <span className="chip chip-amber">
                                Awaiting Reconnect
                              </span>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  )}
                </div>
              )}

              {/* 4. DEVICES TAB */}
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
                          <th>Hardware Hash / Push Token</th>
                          <th>Device Name</th>
                          <th>Platform</th>
                          <th>Last Active</th>
                          <th style={{ textAlign: 'right' }}>Actions</th>
                        </tr>
                      </thead>
                      <tbody>
                        {filteredDevices.map((device, idx) => (
                          <tr key={device.hardware_hash || device.device_id || idx}>
                            <td className="user-cell">@{device.username}</td>
                            <td className="td-mono">{device.hardware_hash || device.device_id || '—'}</td>
                            <td>{device.device_name || 'Desktop'}</td>
                            <td>
                              <span className="chip chip-blue">
                                <Smartphone size={11} /> {device.device_type || device.platform || 'Desktop'}
                              </span>
                            </td>
                            <td className="td-muted">
                              {device.last_active
                                ? new Date(device.last_active * 1000).toLocaleString()
                                : 'Recently'}
                            </td>
                            <td style={{ textAlign: 'right' }}>
                              <button
                                className="btn-danger"
                                onClick={() => handleRevokeDevice(device.username, device.hardware_hash || device.device_id)}
                              >
                                <Trash2 size={13} /> Revoke
                              </button>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  )}
                </div>
              )}

              {/* 5. ENHANCED ANNOUNCEMENTS TAB (2 EQUAL COLUMNS: COMPOSER ON LEFT, LIST ON RIGHT) */}
              {activeTab === 'announcements' && (
                <div className="overview-grid" style={{ gridTemplateColumns: '1fr 1fr' }}>
                  {/* Left Column: Interactive Composer & Live Markdown Preview */}
                  <div className="panel" style={{ height: 'fit-content', background: 'var(--surface-2)' }}>
                    <div className="panel-header">
                      <div className="panel-title">
                        <PlusCircle size={15} /> Announcement Composer
                      </div>
                      {/* Editor Mode Tabs */}
                      <div style={{ display: 'flex', gap: 4 }}>
                        <button
                          className={`btn-ghost ${announcementComposerTab === 'edit' ? 'active' : ''}`}
                          onClick={() => setAnnouncementComposerTab('edit')}
                          style={{ padding: '4px 10px', fontSize: 11 }}
                        >
                          <Edit3 size={12} /> Edit
                        </button>
                        <button
                          className={`btn-ghost ${announcementComposerTab === 'preview' ? 'active' : ''}`}
                          onClick={() => setAnnouncementComposerTab('preview')}
                          style={{ padding: '4px 10px', fontSize: 11 }}
                        >
                          <Eye size={12} /> Live Preview
                        </button>
                      </div>
                    </div>

                    <div className="panel-body announcement-form">
                      {/* 1. Severity / Category Selector */}
                      <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                        <label style={{ fontSize: 12, color: 'var(--text-3)', fontWeight: 600 }}>
                          Announcement Category / Severity:
                        </label>
                        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 6 }}>
                          <button
                            type="button"
                            className={`tpl-chip ${announcementCategory === 'INFO' ? 'active' : ''}`}
                            onClick={() => setAnnouncementCategory('INFO')}
                            style={{
                              background: announcementCategory === 'INFO' ? 'var(--accent-dim)' : 'var(--surface-3)',
                              color: announcementCategory === 'INFO' ? 'var(--accent)' : 'var(--text-2)',
                              borderColor: announcementCategory === 'INFO' ? 'var(--accent-border)' : 'var(--panel-border)',
                            }}
                          >
                            ℹ️ General Info
                          </button>
                          <button
                            type="button"
                            className={`tpl-chip ${announcementCategory === 'MAINTENANCE' ? 'active' : ''}`}
                            onClick={() => setAnnouncementCategory('MAINTENANCE')}
                            style={{
                              background: announcementCategory === 'MAINTENANCE' ? 'var(--amber-dim)' : 'var(--surface-3)',
                              color: announcementCategory === 'MAINTENANCE' ? 'var(--amber)' : 'var(--text-2)',
                              borderColor: announcementCategory === 'MAINTENANCE' ? 'rgba(245,158,11,0.3)' : 'var(--panel-border)',
                            }}
                          >
                            🔧 Maintenance
                          </button>
                          <button
                            type="button"
                            className={`tpl-chip ${announcementCategory === 'UPDATE' ? 'active' : ''}`}
                            onClick={() => setAnnouncementCategory('UPDATE')}
                            style={{
                              background: announcementCategory === 'UPDATE' ? 'var(--blue-dim)' : 'var(--surface-3)',
                              color: announcementCategory === 'UPDATE' ? 'var(--blue)' : 'var(--text-2)',
                              borderColor: announcementCategory === 'UPDATE' ? 'rgba(59,130,246,0.3)' : 'var(--panel-border)',
                            }}
                          >
                            🚀 Core Upgrade
                          </button>
                          <button
                            type="button"
                            className={`tpl-chip ${announcementCategory === 'EMERGENCY' ? 'active' : ''}`}
                            onClick={() => setAnnouncementCategory('EMERGENCY')}
                            style={{
                              background: announcementCategory === 'EMERGENCY' ? 'var(--danger-dim)' : 'var(--surface-3)',
                              color: announcementCategory === 'EMERGENCY' ? '#fca5a5' : 'var(--text-2)',
                              borderColor: announcementCategory === 'EMERGENCY' ? 'rgba(239,68,68,0.4)' : 'var(--panel-border)',
                            }}
                          >
                            🚨 Outage Alert
                          </button>
                        </div>
                      </div>

                      {/* 2. Formatting Toolbar */}
                      {announcementComposerTab === 'edit' && (
                        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                            <label style={{ fontSize: 12, color: 'var(--text-3)', fontWeight: 600 }}>
                              Markdown Formatting Helper:
                            </label>
                            <span style={{ fontSize: 11, color: 'var(--text-3)' }}>
                              {newAnnouncementText.length} chars
                            </span>
                          </div>

                          <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
                            <button
                              type="button"
                              className="tpl-chip"
                              onClick={() => handleFormatInsert('**', '**')}
                              title="Bold Text"
                            >
                              <strong>B</strong>
                            </button>
                            <button
                              type="button"
                              className="tpl-chip"
                              onClick={() => handleFormatInsert('\n### ')}
                              title="Header Level 3"
                            >
                              H3
                            </button>
                            <button
                              type="button"
                              className="tpl-chip"
                              onClick={() => handleFormatInsert('\n- ')}
                              title="Bullet Item"
                            >
                              • List
                            </button>
                            <button
                              type="button"
                              className="tpl-chip"
                              onClick={() => handleFormatInsert('```\n', '\n```')}
                              title="Code Block"
                            >
                              &lt;/&gt; Code
                            </button>
                            <button
                              type="button"
                              className="tpl-chip"
                              onClick={() => handleFormatInsert('[title](https://)')}
                              title="Hyperlink"
                            >
                              🔗 Link
                            </button>
                          </div>

                          <textarea
                            placeholder="Type markdown announcement message for connected clients..."
                            value={newAnnouncementText}
                            onChange={(e) => setNewAnnouncementText(e.target.value)}
                            style={{ minHeight: 130 }}
                          />
                        </div>
                      )}

                      {/* 3. Live Markdown Preview Tab */}
                      {announcementComposerTab === 'preview' && (
                        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                          <label style={{ fontSize: 12, color: 'var(--text-3)', fontWeight: 600 }}>
                            Client Markdown Live-Preview:
                          </label>
                          <div
                            style={{
                              background: 'var(--surface-3)',
                              border: '1px dashed var(--accent-border)',
                              borderRadius: 'var(--radius-md)',
                              padding: '16px',
                              minHeight: 140,
                            }}
                          >
                            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                              {getAnnouncementBadge(newAnnouncementText || `[${announcementCategory}]`)}
                              <span style={{ fontSize: 11, color: 'var(--text-3)' }}>Preview Mode</span>
                            </div>
                            {newAnnouncementText.trim() ? (
                              <MarkdownMessage content={newAnnouncementText} />
                            ) : (
                              <div style={{ color: 'var(--text-3)', fontSize: 12, fontStyle: 'italic' }}>
                                Announcement preview will appear here as you type in the editor...
                              </div>
                            )}
                          </div>
                        </div>
                      )}

                      {/* 4. Quick Templates / Presets */}
                      <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                        <label style={{ fontSize: 11, color: 'var(--text-3)', fontWeight: 600 }}>
                          Quick Announcement Presets:
                        </label>
                        <div className="template-chips">
                          <button
                            type="button"
                            className="tpl-chip"
                            onClick={() => {
                              setAnnouncementCategory('UPDATE');
                              setNewAnnouncementText(
                                '🚀 Vexta V2 Relay Update: Core routing performance patch & enhanced group message relaying are now live.'
                              );
                            }}
                          >
                            🚀 Relay Update
                          </button>
                          <button
                            type="button"
                            className="tpl-chip"
                            onClick={() => {
                              setAnnouncementCategory('UPDATE');
                              setNewAnnouncementText(
                                '✨ Feature Release: Multi-device key bundle syncing and encrypted vault backup improvements are active.'
                              );
                            }}
                          >
                            ✨ New Feature
                          </button>
                          <button
                            type="button"
                            className="tpl-chip"
                            onClick={() => {
                              setAnnouncementCategory('UPDATE');
                              setNewAnnouncementText(
                                '🛡️ Security Patch: Essential security patch applied to challenge nonces. All systems nominal.'
                              );
                            }}
                          >
                            🛡️ Security Patch
                          </button>
                          <button
                            type="button"
                            className="tpl-chip"
                            onClick={() => {
                              setAnnouncementCategory('MAINTENANCE');
                              setNewAnnouncementText(
                                '⚠️ Maintenance Window: Scheduled database WAL checkpoint in 30 minutes. Brief reconnects may occur.'
                              );
                            }}
                          >
                            🔧 Maintenance
                          </button>
                          <button
                            type="button"
                            className="tpl-chip"
                            onClick={() => {
                              setAnnouncementCategory('INFO');
                              setNewAnnouncementText(
                                'ℹ️ Nominal Status: All signal relay clusters operating at <0.2ms latency.'
                              );
                            }}
                          >
                            ✅ Nominal Status
                          </button>
                        </div>
                      </div>

                      {/* 5. Publish Button */}
                      <button
                        className="btn-primary"
                        onClick={() => handlePostAnnouncement()}
                        style={{ marginTop: 8 }}
                      >
                        <Megaphone size={15} /> Publish Announcement
                      </button>
                    </div>
                  </div>

                  {/* Right Column: Announcement List with Category Filter */}
                  <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
                    {/* Filter Chips Bar */}
                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 12, flexWrap: 'wrap' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                        <Filter size={14} style={{ color: 'var(--text-3)' }} />
                        <span style={{ fontSize: 12, color: 'var(--text-3)', fontWeight: 600 }}>Category Filter:</span>
                      </div>
                      <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
                        {['ALL', 'EMERGENCY', 'MAINTENANCE', 'UPDATE', 'INFO'].map((cat) => (
                          <button
                            key={cat}
                            className={`tpl-chip ${announcementFilter === cat ? 'active' : ''}`}
                            onClick={() => setAnnouncementFilter(cat)}
                            style={{
                              borderColor: announcementFilter === cat ? 'var(--accent-border)' : 'var(--panel-border)',
                              color: announcementFilter === cat ? 'var(--accent)' : 'var(--text-2)',
                              background: announcementFilter === cat ? 'var(--accent-dim)' : 'var(--surface-3)',
                            }}
                          >
                            {cat}
                          </button>
                        ))}
                      </div>
                    </div>

                    {/* Announcement List Items */}
                    <div className="announcement-list">
                      {filteredAnnouncements.length === 0 ? (
                        <div className="empty-state">
                          <Megaphone className="empty-state-icon" />
                          <div className="empty-state-text">No system announcements match your filter</div>
                          <div className="empty-state-sub">Use the composer on the left to post a new announcement.</div>
                        </div>
                      ) : (
                        filteredAnnouncements.map((ann) => (
                          <div className="ann-item" key={ann.id} style={{ background: 'var(--surface-2)' }}>
                            <div className="ann-meta">
                              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                                {getAnnouncementBadge(ann.message)}
                                <span style={{ fontSize: 11, color: 'var(--text-3)', fontFamily: 'IBM Plex Mono, monospace' }}>
                                  #{ann.id}
                                </span>
                              </div>
                              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                                <span className="ann-id">
                                  {ann.created_at
                                    ? new Date(ann.created_at * 1000).toLocaleString()
                                    : ''}
                                </span>
                                <button
                                  className="btn-ghost"
                                  onClick={() => {
                                    navigator.clipboard.writeText(ann.message);
                                    showToast('Announcement text copied to clipboard!', 'success');
                                  }}
                                  title="Copy raw markdown"
                                  style={{ padding: '4px 8px' }}
                                >
                                  <Copy size={12} />
                                </button>
                                <button
                                  className="btn-danger"
                                  onClick={() => handleDeleteAnnouncement(ann.id)}
                                  title="Delete broadcast announcement"
                                >
                                  <Trash2 size={12} />
                                </button>
                              </div>
                            </div>
                            <div className="ann-text" style={{ paddingTop: 4 }}>
                              <MarkdownMessage content={ann.message} />
                            </div>
                          </div>
                        ))
                      )}
                    </div>
                  </div>
                </div>
              )}

              {/* 7. IP FIREWALL TAB */}
              {activeTab === 'firewall' && (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
                  <form onSubmit={handleBanIp} style={{ display: 'flex', gap: 12, flexWrap: 'wrap', background: 'var(--surface-2)', padding: 16, borderRadius: 'var(--radius-md)', border: '1px solid var(--panel-border)' }}>
                    <input
                      type="text"
                      placeholder="IP Address (e.g. 192.168.1.100)"
                      value={banIpInput}
                      onChange={(e) => setBanIpInput(e.target.value)}
                      style={{ flex: '1 1 200px', padding: '8px 12px', background: 'var(--surface-3)', border: '1px solid var(--panel-border)', borderRadius: 6, color: '#fff' }}
                      required
                    />
                    <input
                      type="text"
                      placeholder="Reason (e.g. Rate limit abuse / DDOS)"
                      value={banReasonInput}
                      onChange={(e) => setBanReasonInput(e.target.value)}
                      style={{ flex: '2 1 300px', padding: '8px 12px', background: 'var(--surface-3)', border: '1px solid var(--panel-border)', borderRadius: 6, color: '#fff' }}
                    />
                    <button type="submit" className="btn-danger" style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                      <Shield size={14} /> Ban IP Address
                    </button>
                  </form>

                  <div className="table-wrap">
                    <table className="data-table">
                      <thead>
                        <tr>
                          <th>IP Address</th>
                          <th>Ban Reason</th>
                          <th>Banned By</th>
                          <th>Banned Timestamp</th>
                          <th>Action</th>
                        </tr>
                      </thead>
                      <tbody>
                        {bannedIps.length === 0 ? (
                          <tr>
                            <td colSpan="5" style={{ textAlign: 'center', padding: 24, color: 'var(--text-3)' }}>
                              🛡️ Firewall Active. No IP addresses are currently banned.
                            </td>
                          </tr>
                        ) : (
                          bannedIps.map((b) => (
                            <tr key={b.ip}>
                              <td style={{ fontFamily: 'IBM Plex Mono, monospace', fontWeight: 600, color: 'var(--danger)' }}>{b.ip}</td>
                              <td>{b.reason}</td>
                              <td>{b.banned_by}</td>
                              <td>{b.created_at ? new Date(b.created_at * 1000).toLocaleString() : '—'}</td>
                              <td>
                                <button className="btn-ghost" onClick={() => handleUnbanIp(b.ip)}>Unban IP</button>
                              </td>
                            </tr>
                          ))
                        )}
                      </tbody>
                    </table>
                  </div>
                </div>
              )}

              {/* 8. EMERGENCY MAINTENANCE TAB */}
              {activeTab === 'maintenance' && (
                <div className="panel" style={{ background: 'var(--surface-2)', padding: 24 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 16, marginBottom: 20 }}>
                    <AlertTriangle size={36} style={{ color: stats.maintenance_mode ? 'var(--danger)' : 'var(--amber)' }} />
                    <div>
                      <h3 style={{ fontSize: 18, margin: 0 }}>Bridge Maintenance Mode Control</h3>
                      <p style={{ margin: '4px 0 0', color: 'var(--text-3)', fontSize: 13 }}>
                        When maintenance mode is ON, incoming WebSocket client connections are gracefully rejected with a 503 Notice.
                      </p>
                    </div>
                  </div>

                  <div style={{ display: 'flex', alignItems: 'center', gap: 16, padding: 16, background: 'var(--surface-3)', borderRadius: 12, border: '1px solid var(--panel-border)' }}>
                    <span style={{ fontSize: 14, fontWeight: 600 }}>Current Mode:</span>
                    <span className={stats.maintenance_mode ? 'chip chip-red' : 'chip chip-green'} style={{ fontSize: 14, padding: '4px 12px' }}>
                      {stats.maintenance_mode ? '🚨 EMERGENCY MAINTENANCE ACTIVE' : '🟢 BRIDGE OPERATIONAL (ONLINE)'}
                    </span>
                  </div>

                  <div style={{ display: 'flex', gap: 16, marginTop: 24 }}>
                    {!stats.maintenance_mode ? (
                      <button className="btn-danger" onClick={() => handleToggleMaintenance(true)} style={{ padding: '10px 20px' }}>
                        🚨 Enable Maintenance Lockdown
                      </button>
                    ) : (
                      <button className="btn-primary" onClick={() => handleToggleMaintenance(false)} style={{ padding: '10px 20px' }}>
                        🟢 Resume Normal Operation
                      </button>
                    )}
                  </div>
                </div>
              )}

              {/* 9. AUDIT LOGS TAB */}
              {activeTab === 'audit' && (
                <div className="table-wrap">
                  <table className="data-table">
                    <thead>
                      <tr>
                        <th>#ID</th>
                        <th>Action</th>
                        <th>Target</th>
                        <th>Details</th>
                        <th>Timestamp</th>
                      </tr>
                    </thead>
                    <tbody>
                      {auditLogs.length === 0 ? (
                        <tr>
                          <td colSpan="5" style={{ textAlign: 'center', padding: 24, color: 'var(--text-3)' }}>
                            📜 No administrative audit logs recorded yet.
                          </td>
                        </tr>
                      ) : (
                        auditLogs.map((log) => (
                          <tr key={log.id}>
                            <td style={{ fontFamily: 'IBM Plex Mono, monospace' }}>#{log.id}</td>
                            <td><span className="chip chip-blue">{log.action}</span></td>
                            <td style={{ fontFamily: 'IBM Plex Mono, monospace' }}>{log.target}</td>
                            <td>{log.details}</td>
                            <td>{log.timestamp ? new Date(log.timestamp * 1000).toLocaleString() : '—'}</td>
                          </tr>
                        ))
                      )}
                    </tbody>
                  </table>
                </div>
              )}

              {/* 10. TRAFFIC ANALYTICS TAB */}
              {activeTab === 'analytics' && (
                <div className="table-wrap">
                  <table className="data-table">
                    <thead>
                      <tr>
                        <th>User Account</th>
                        <th>Messages Relayed</th>
                        <th>Bandwidth Utilized</th>
                        <th>Last Active</th>
                      </tr>
                    </thead>
                    <tbody>
                      {topUsers.length === 0 ? (
                        <tr>
                          <td colSpan="4" style={{ textAlign: 'center', padding: 24, color: 'var(--text-3)' }}>
                            📊 No per-user traffic recorded yet.
                          </td>
                        </tr>
                      ) : (
                        topUsers.map((u) => (
                          <tr key={u.username}>
                            <td style={{ fontWeight: 600, color: 'var(--accent)' }}>@{u.username}</td>
                            <td>{u.message_count} msgs</td>
                            <td style={{ fontFamily: 'IBM Plex Mono, monospace' }}>{formatBytes(u.byte_count)}</td>
                            <td>{u.last_active ? new Date(u.last_active * 1000).toLocaleString() : '—'}</td>
                          </tr>
                        ))
                      )}
                    </tbody>
                  </table>
                </div>
              )}

              {/* 6. STORAGE & HEALTH TAB */}
              {activeTab === 'health' && (
                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))', gap: 20 }}>
                  <div style={{ gridColumn: '1 / -1', display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'var(--surface-2)', padding: 16, borderRadius: 12, border: '1px solid var(--panel-border)' }}>
                    <div>
                      <strong style={{ fontSize: 15 }}>SQLite Database Optimization & Maintenance</strong>
                      <p style={{ margin: '2px 0 0', fontSize: 12, color: 'var(--text-3)' }}>Run WAL checkpoint and VACUUM to reclaim disk space.</p>
                    </div>
                    <button className="btn-primary" onClick={handleVacuumDb} style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                      <RefreshCw size={14} /> Run WAL Vacuum
                    </button>
                  </div>
                  {/* Panel 1: SQLite Storage Engine */}
                  <div className="panel" style={{ background: 'var(--surface-2)' }}>
                    <div className="panel-header">
                      <div className="panel-title">
                        <Database size={16} /> SQLite WAL Storage Engine
                      </div>
                    </div>
                    <div className="panel-body" style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
                      <div style={{ display: 'flex', justifyContent: 'space-between', borderBottom: '1px solid var(--panel-border)', paddingBottom: 8 }}>
                        <span style={{ color: 'var(--text-3)' }}>Database File (`vexta_bridge_v2.db`)</span>
                        <strong style={{ fontFamily: 'IBM Plex Mono, monospace' }}>{formatBytes(stats.database_size_bytes)}</strong>
                      </div>
                      <div style={{ display: 'flex', justifyContent: 'space-between', borderBottom: '1px solid var(--panel-border)', paddingBottom: 8 }}>
                        <span style={{ color: 'var(--text-3)' }}>WAL File (`vexta_bridge_v2.db-wal`)</span>
                        <strong style={{ fontFamily: 'IBM Plex Mono, monospace' }}>{formatBytes(stats.wal_size_bytes)}</strong>
                      </div>
                      <div style={{ display: 'flex', justifyContent: 'space-between', borderBottom: '1px solid var(--panel-border)', paddingBottom: 8 }}>
                        <span style={{ color: 'var(--text-3)' }}>Journaling Mode</span>
                        <span className="chip chip-green">WAL (Write-Ahead Logging)</span>
                      </div>
                      <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                        <span style={{ color: 'var(--text-3)' }}>Foreign Keys</span>
                        <span className="chip chip-blue">ENABLED (CASCADE)</span>
                      </div>
                    </div>
                  </div>

                  {/* Panel 2: Account Provisioning & Vault Ratios */}
                  <div className="panel" style={{ background: 'var(--surface-2)' }}>
                    <div className="panel-header">
                      <div className="panel-title">
                        <Shield size={16} /> Identity & Provisioning Ratios
                      </div>
                    </div>
                    <div className="panel-body" style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
                      <div style={{ display: 'flex', justifyContent: 'space-between', borderBottom: '1px solid var(--panel-border)', paddingBottom: 8 }}>
                        <span style={{ color: 'var(--text-3)' }}>Provisioned User Accounts</span>
                        <strong>{stats.provisioned_users || 0} / {stats.total_users || 0}</strong>
                      </div>
                      <div style={{ display: 'flex', justifyContent: 'space-between', borderBottom: '1px solid var(--panel-border)', paddingBottom: 8 }}>
                        <span style={{ color: 'var(--text-3)' }}>Encrypted Vault Backups</span>
                        <strong>{stats.users_with_vault || 0} users</strong>
                      </div>
                      <div style={{ display: 'flex', justifyContent: 'space-between', borderBottom: '1px solid var(--panel-border)', paddingBottom: 8 }}>
                        <span style={{ color: 'var(--text-3)' }}>Pre-key Cryptographic Bundles</span>
                        <strong>{stats.users_with_prekey || 0} users</strong>
                      </div>
                      <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                        <span style={{ color: 'var(--text-3)' }}>Currently Locked Out</span>
                        <span className={stats.locked_users > 0 ? 'chip chip-red' : 'chip chip-green'}>
                          {stats.locked_users || 0} accounts
                        </span>
                      </div>
                    </div>
                  </div>

                  {/* Panel 3: Server Process Telemetry */}
                  <div className="panel" style={{ background: 'var(--surface-2)' }}>
                    <div className="panel-header">
                      <div className="panel-title">
                        <Server size={16} /> Server Process & Uptime Metrics
                      </div>
                    </div>
                    <div className="panel-body" style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
                      <div style={{ display: 'flex', justifyContent: 'space-between', borderBottom: '1px solid var(--panel-border)', paddingBottom: 8 }}>
                        <span style={{ color: 'var(--text-3)' }}>Rust Server Uptime</span>
                        <strong style={{ fontFamily: 'IBM Plex Mono, monospace', color: 'var(--accent)' }}>
                          {formatUptime(stats.uptime_seconds)}
                        </strong>
                      </div>
                      <div style={{ display: 'flex', justifyContent: 'space-between', borderBottom: '1px solid var(--panel-border)', paddingBottom: 8 }}>
                        <span style={{ color: 'var(--text-3)' }}>In-Memory Routing Table</span>
                        <span>DashMap (Lock-Free)</span>
                      </div>
                      <div style={{ display: 'flex', justifyContent: 'space-between', borderBottom: '1px solid var(--panel-border)', paddingBottom: 8 }}>
                        <span style={{ color: 'var(--text-3)' }}>Cryptographic Authentication</span>
                        <span>Ed25519 Challenge</span>
                      </div>
                      <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                        <span style={{ color: 'var(--text-3)' }}>Binary Framing</span>
                        <span>MessagePack (rmp-serde)</span>
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>
        </main>
      </div>

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
