import { useState, useEffect, useRef } from 'react'

const API_URL = 'http://localhost:3001'

function App() {
  const [messages, setMessages] = useState([])
  const [input, setInput] = useState('')
  const [loading, setLoading] = useState(false)
  const messagesEndRef = useRef(null)

  useEffect(() => {
    fetchHistory()
  }, [])

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages])

  const fetchHistory = async () => {
    try {
      const res = await fetch(`${API_URL}/api/history`)
      const data = await res.json()
      setMessages(data.messages || [])
    } catch (e) {
      console.error('Failed to fetch history', e)
    }
  }

  const sendMessage = async () => {
    if (!input.trim() || loading) return
    const userText = input
    setInput('')
    setLoading(true)
    // Optimistically show user message
    setMessages(prev => [...prev, { role: 'user', content: userText }])
    try {
      await fetch(`${API_URL}/api/chat`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ message: userText })
      })
      // Replace all messages with clean data from DB
      await fetchHistory()
    } catch (e) {
      setMessages(prev => [...prev, { role: 'assistant', content: 'Error: could not reach IGRIS backend.' }])
    }
    setLoading(false)
  }

  const handleKey = (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      sendMessage()
    }
  }

  return (
    <div style={styles.root}>
      <aside style={styles.sidebar}>
        <div style={styles.logo}>
          <span style={styles.logoIcon}>⚡</span>
          <span style={styles.logoText}>IGRIS</span>
        </div>
        <div style={styles.sidebarSection}>CHAT</div>
        <div style={styles.sidebarItem} onClick={fetchHistory}>↺ Refresh History</div>
        <div style={styles.sidebarItem} onClick={() => setMessages([])}>+ New Chat</div>
        <div style={styles.sidebarFooter}>
          <div style={styles.sidebarFooterText}>v0.1.0 · Sandbox</div>
        </div>
      </aside>
      <main style={styles.main}>
        <header style={styles.header}>
          <span style={styles.headerTitle}>IGRIS Assistant</span>
          <span style={styles.headerBadge}>● Online</span>
        </header>
        <div style={styles.chatArea}>
          {messages.length === 0 && !loading && (
            <div style={styles.empty}>
              <div style={styles.emptyIcon}>⚡</div>
              <div style={styles.emptyTitle}>IGRIS</div>
              <div style={styles.emptySubtitle}>Intelligent General Runtime & Integrated System</div>
              <div style={styles.emptyHint}>Ask me anything. I can run shell commands, search memory, and more.</div>
            </div>
          )}
          {messages.map((msg, i) => (
            <div key={i} style={msg.role === 'user' ? styles.userBubbleWrap : styles.aiBubbleWrap}>
              {msg.role === 'assistant' && <div style={styles.aiAvatar}>⚡</div>}
              <div style={msg.role === 'user' ? styles.userBubble : styles.aiBubble}>
                <pre style={styles.msgText}>{msg.content}</pre>
              </div>
            </div>
          ))}
          {loading && (
            <div style={styles.aiBubbleWrap}>
              <div style={styles.aiAvatar}>⚡</div>
              <div style={styles.aiBubble}>
                <span style={styles.typing}>▌ Thinking...</span>
              </div>
            </div>
          )}
          <div ref={messagesEndRef} />
        </div>
        <div style={styles.inputArea}>
          <textarea
            style={styles.input}
            value={input}
            onChange={e => setInput(e.target.value)}
            onKeyDown={handleKey}
            placeholder="Message IGRIS... (Enter to send, Shift+Enter for newline)"
            rows={1}
          />
          <button style={loading ? styles.sendBtnDisabled : styles.sendBtn} onClick={sendMessage} disabled={loading}>
            ➤
          </button>
        </div>
      </main>
    </div>
  )
}

const styles = {
  root: { display: 'flex', height: '100vh', background: '#0d0d0d', color: '#e0e0e0', fontFamily: "'Inter', sans-serif", overflow: 'hidden' },
  sidebar: { width: 240, background: '#111', borderRight: '1px solid #222', display: 'flex', flexDirection: 'column', padding: '16px 0' },
  logo: { display: 'flex', alignItems: 'center', gap: 8, padding: '0 20px 20px', borderBottom: '1px solid #222' },
  logoIcon: { fontSize: 24, color: '#00d4ff' },
  logoText: { fontSize: 20, fontWeight: 700, color: '#fff', letterSpacing: 2 },
  sidebarSection: { fontSize: 11, color: '#555', padding: '16px 20px 8px', letterSpacing: 1, textTransform: 'uppercase' },
  sidebarItem: { padding: '10px 20px', cursor: 'pointer', color: '#aaa', fontSize: 14, borderRadius: 6, margin: '2px 8px', transition: 'all 0.2s', ':hover': { background: '#1a1a1a', color: '#fff' } },
  sidebarFooter: { marginTop: 'auto', padding: '16px 20px', borderTop: '1px solid #222' },
  sidebarFooterText: { fontSize: 12, color: '#444' },
  main: { flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' },
  header: { padding: '16px 24px', borderBottom: '1px solid #1e1e1e', display: 'flex', alignItems: 'center', justifyContent: 'space-between', background: '#111' },
  headerTitle: { fontWeight: 600, fontSize: 16, color: '#fff' },
  headerBadge: { fontSize: 12, color: '#00d4ff', background: '#001a22', padding: '4px 10px', borderRadius: 20, border: '1px solid #00d4ff33' },
  chatArea: { flex: 1, overflowY: 'auto', padding: '24px', display: 'flex', flexDirection: 'column', gap: 16 },
  empty: { display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', flex: 1, gap: 12, color: '#555', marginTop: 80 },
  emptyIcon: { fontSize: 48, color: '#00d4ff44' },
  emptyTitle: { fontSize: 28, fontWeight: 700, color: '#333', letterSpacing: 3 },
  emptySubtitle: { fontSize: 13, color: '#444' },
  emptyHint: { fontSize: 12, color: '#333', marginTop: 8 },
  userBubbleWrap: { display: 'flex', justifyContent: 'flex-end' },
  aiBubbleWrap: { display: 'flex', alignItems: 'flex-start', gap: 12 },
  aiAvatar: { width: 32, height: 32, borderRadius: '50%', background: '#001a22', border: '1px solid #00d4ff44', display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: 14, color: '#00d4ff', flexShrink: 0 },
  userBubble: { maxWidth: '70%', background: '#1a1a2e', border: '1px solid #2a2a4a', borderRadius: '18px 18px 4px 18px', padding: '12px 16px' },
  aiBubble: { maxWidth: '80%', background: '#141414', border: '1px solid #1e1e1e', borderRadius: '4px 18px 18px 18px', padding: '12px 16px' },
  msgText: { margin: 0, whiteSpace: 'pre-wrap', wordBreak: 'break-word', fontSize: 14, lineHeight: 1.6, color: '#e0e0e0', fontFamily: 'inherit' },
  typing: { color: '#00d4ff', fontSize: 14, animation: 'pulse 1s infinite' },
  inputArea: { padding: '16px 24px', borderTop: '1px solid #1e1e1e', background: '#111', display: 'flex', gap: 12, alignItems: 'flex-end' },
  input: { flex: 1, background: '#1a1a1a', border: '1px solid #2a2a2a', borderRadius: 12, padding: '14px 16px', color: '#e0e0e0', fontSize: 14, resize: 'none', outline: 'none', fontFamily: 'inherit', lineHeight: 1.5, maxHeight: 200, overflowY: 'auto' },
  sendBtn: { width: 44, height: 44, borderRadius: 10, background: '#00d4ff', border: 'none', color: '#000', fontSize: 18, cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center', fontWeight: 700 },
  sendBtnDisabled: { width: 44, height: 44, borderRadius: 10, background: '#1a1a1a', border: '1px solid #2a2a2a', color: '#444', fontSize: 18, cursor: 'not-allowed', display: 'flex', alignItems: 'center', justifyContent: 'center' },
}

export default App
