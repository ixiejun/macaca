#!/usr/bin/env bash
set -euo pipefail

MACACA_USER="${MACACA_USER:-macaca}"
MACACA_DIR="${MACACA_DIR:-/var/lib/macaca}"
MACACA_BIN="${MACACA_BIN:-$(which macaca 2>/dev/null || echo /usr/local/bin/macaca)}"

echo "=== Macaca systemd Installation ==="

# 1. Create user if needed
if ! id -u "$MACACA_USER" >/dev/null 2>&1; then
    echo "Creating user: $MACACA_USER"
    sudo useradd --system --home-dir "$MACACA_DIR" --shell /usr/sbin/nologin "$MACACA_USER"
fi

# 2. Create directories
echo "Creating directories..."
sudo mkdir -p "$MACACA_DIR"
sudo mkdir -p /etc/macaca
sudo chown -R "$MACACA_USER:$MACACA_USER" "$MACACA_DIR"

# 3. Copy default config if not exists
if [ ! -f /etc/macaca/env ]; then
    echo "Creating default environment file..."
    cat <<'ENVEOF' | sudo tee /etc/macaca/env > /dev/null
# Macaca Environment Configuration
RUST_LOG=info
# DASHSCOPE_API_KEY=your-key-here
# OPENAI_API_KEY=your-key-here
ENVEOF
    sudo chmod 600 /etc/macaca/env
fi

# 4. Install service file
echo "Installing systemd service..."
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
sudo cp "$SCRIPT_DIR/macaca.service" /etc/systemd/system/macaca.service

# Update ExecStart path
sudo sed -i "s|/usr/local/bin/macaca|$MACACA_BIN|g" /etc/systemd/system/macaca.service
sudo sed -i "s|/var/lib/macaca|$MACACA_DIR|g" /etc/systemd/system/macaca.service

# 5. Reload and enable
sudo systemctl daemon-reload
sudo systemctl enable macaca

echo ""
echo "=== Installation Complete ==="
echo ""
echo "Commands:"
echo "  sudo systemctl start macaca     # Start the service"
echo "  sudo systemctl status macaca    # Check status"
echo "  sudo systemctl stop macaca      # Stop the service"
echo "  sudo journalctl -u macaca -f    # Follow logs"
echo ""
echo "Configuration:"
echo "  Service: /etc/systemd/system/macaca.service"
echo "  Env:     /etc/macaca/env"
echo "  Data:    $MACACA_DIR"
