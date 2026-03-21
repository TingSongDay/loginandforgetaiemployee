#!/bin/bash
# Start ZeroClaw with AI-powered WhatsApp replies from NeoHumanWorkStation

set -e

echo "🚀 Starting ZeroClaw with AI Replies..."
echo ""

# Kill any existing processes
pkill -9 -f "zeroclaw gateway" 2>/dev/null || true
pkill -9 -f "zeroclaw station" 2>/dev/null || true
pkill -9 -f "playwright-stealth.js launch" 2>/dev/null || true
pkill -9 -f "Google Chrome for Testing" 2>/dev/null || true
pkill -9 -f "Brave Browser" 2>/dev/null || true
pkill -9 -f "chromedriver" 2>/dev/null || true
sleep 1

# Start ChromeDriver
echo "🔌 Starting ChromeDriver..."
chromedriver --port=9515 > /tmp/chromedriver.log 2>&1 &
sleep 2

# Build from the NeoHumanWorkStation repo
cd "/Users/tingsongdai/Openclaw n8n/NeoHuman/NeoHumanWorkStation"

if [ ! -d node_modules/playwright ]; then
  echo "📦 Installing Playwright dependencies..."
  npm install
fi

# Install dashboard dependencies if needed
if [ ! -d web/node_modules ]; then
  echo "📦 Installing dashboard dependencies..."
  (cd web && npm install)
fi

# Rebuild dashboard assets so Rust embeds the latest UI
echo "🎨 Building dashboard..."
(cd web && npm run build)

# Build Rust binary
echo "📦 Building ZeroClaw..."
cargo build --features browser-native --bin zeroclaw

echo ""
echo "🧠 Starting Gateway (AI)..."
./target/debug/zeroclaw gateway start > /tmp/zeroclaw-gateway.log 2>&1 &
GATEWAY_PID=$!
echo "Gateway PID: $GATEWAY_PID"

# Wait for gateway to start
sleep 8

echo ""
echo "🖥️  Starting Station (WhatsApp + Dashboard browsers)..."
./target/debug/zeroclaw station start > /tmp/zeroclaw-station.log 2>&1 &
STATION_PID=$!
echo "Station PID: $STATION_PID"

sleep 15

echo ""
echo "=============================================="
echo "✅ ZeroClaw with AI Replies is running!"
echo "=============================================="
echo ""
echo "📱 WhatsApp (worker_a): Left panel - Playwright/Chromium"
echo "🖥️  Dashboard (worker_b): Right panel - Brave Browser"
echo "🤖 AI Mode: ENABLED (reply_mode = ai)"
echo ""
echo "🔌 Gateway: http://127.0.0.1:42617"
echo ""
echo "💬 How it works:"
echo "   1. Someone sends WhatsApp message to you"
echo "   2. Station detects message in browser"
echo "   3. Station calls Gateway AI for reply"
echo "   4. AI generates contextual response"
echo "   5. Station types reply in WhatsApp Web"
echo ""
echo "📋 Process IDs:"
echo "   Gateway: $GATEWAY_PID"
echo "   Station: $STATION_PID"
echo ""
echo "📝 Logs:"
echo "   tail -f /tmp/zeroclaw-gateway.log"
echo "   tail -f /tmp/zeroclaw-station.log"
echo ""
echo "Press Ctrl+C to stop all processes"
echo ""

# Wait for Ctrl+C
trap "echo ''; echo '🛑 Stopping...'; pkill -9 -f 'zeroclaw gateway' 2>/dev/null; pkill -9 -f 'zeroclaw station' 2>/dev/null; pkill -9 -f 'playwright-stealth.js launch' 2>/dev/null; exit 0" INT
wait
