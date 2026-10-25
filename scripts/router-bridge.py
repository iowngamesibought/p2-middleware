"""Terminal bridge on this PC, fed by a scoped redirect on the OpenWrt router.

The PS3 keeps the router as its gateway. The router redirects only PS3 TCP27017 to
the legacy CM addresses onto this PC and drops their UDP27017; all other traffic is
routed normally. Credentials pass only from `authorize` stdout to `serve` stdin;
this script never reads them. The router table is removed when this script exits
or its SSH channel closes.
"""
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
import argparse
import datetime
import ipaddress
import json
import os
import re
import subprocess
import sys
import tempfile
import threading
import webbrowser

ROOT = Path(__file__).resolve().parents[1]
TABLE = 'p2_pc_bridge'
LEGACY = re.findall(r'Ipv4Addr::new\((\d+), (\d+), (\d+), (\d+)\)',
                    (ROOT / 'crates/bridge-core/src/legacy_cm.rs').read_text(encoding='utf-8'))
LEGACY = ['.'.join(parts) for parts in LEGACY]
assert len(LEGACY) == 26

PAGE = b'''<!doctype html><meta charset="utf-8"><title>Steam approval</title>
<style>body{font:18px system-ui;margin:40px auto;max-width:520px;color:#18202b;background:#f5f7fa}img{width:400px;height:400px;background:white}</style>
<h1>Approve the PS3 gateway</h1><p>Steam app &rarr; Steam Guard &rarr; Scan QR code.</p>
<img alt="Steam sign-in QR"><p id="s">Waiting for code...</p>
<script>async function r(){let x=await fetch('/qr.svg?t='+Date.now());let i=document.querySelector('img'),s=document.querySelector('#s');
if(x.ok){i.style.display='block';s.textContent='Code refreshes automatically.';let o=i.src;i.src=URL.createObjectURL(await x.blob());if(o.startsWith('blob:'))URL.revokeObjectURL(o);}
else{i.style.display='none';s.textContent='No active code. Check the terminal.';}}r();setInterval(r,1500)</script>'''


def rules(ps3, pc, router, lan):
    cms = ', '.join(LEGACY)
    return f'''table ip {TABLE} {{
    set cm {{ type ipv4_addr; elements = {{ {cms} }} }}
    chain pre_filter {{
        type filter hook prerouting priority -120; policy accept;
        iifname "{lan}" ip saddr {ps3} ip daddr @cm udp dport 27017 counter drop
    }}
    chain pre_nat {{
        type nat hook prerouting priority -110; policy accept;
        iifname "{lan}" ip saddr {ps3} ip daddr @cm tcp dport 27017 counter dnat to {pc}:27017
    }}
    chain post_nat {{
        type nat hook postrouting priority 90; policy accept;
        oifname "{lan}" ip saddr {ps3} ip daddr {pc} tcp dport 27017 ct status dnat counter snat to {router}
    }}
}}
'''.encode()


def ssh(target, command, **kw):
    return subprocess.Popen(['ssh', '-T', '-o', 'BatchMode=yes', '-o', 'StrictHostKeyChecking=yes',
                             '-o', 'ConnectTimeout=5', '-o', 'ServerAliveInterval=5',
                             '-o', 'ServerAliveCountMax=3', target, command], **kw)


def qr_server(qr, port):
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_GET(self):
            path = self.path.split('?')[0]
            if path == '/':
                data, mime, code = PAGE, 'text/html; charset=utf-8', 200
            elif path == '/qr.svg' and qr.exists():
                data, mime, code = qr.read_bytes(), 'image/svg+xml', 200
            else:
                data, mime, code = b'none', 'text/plain', 404
            self.send_response(code)
            self.send_header('Content-Type', mime)
            self.send_header('Cache-Control', 'no-store')
            self.send_header('Content-Length', str(len(data)))
            self.end_headers()
            self.wfile.write(data)

    server = HTTPServer(('127.0.0.1', port), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


def main():
    default_bridge = Path(os.environ['LOCALAPPDATA']) / 'p2-middleware-target/debug/p2-bridge.exe'
    p = argparse.ArgumentParser()
    p.add_argument('--ps3', required=True)
    p.add_argument('--pc', required=True, help="this PC's LAN address")
    p.add_argument('--router', default='10.0.0.1')
    p.add_argument('--ssh', default='root@10.0.0.1')
    p.add_argument('--lan-if', default='br-lan')
    p.add_argument('--bridge', type=Path, default=default_bridge)
    p.add_argument('--port', type=int, default=18743)
    p.add_argument('--no-crossplay', action='store_true')
    a = p.parse_args()
    for ip in (a.ps3, a.pc, a.router):
        if not ipaddress.IPv4Address(ip).is_private:
            p.error(f'{ip} is not a private address')
    if not re.fullmatch(r'[A-Za-z0-9_-]+', a.lan_if):
        p.error('invalid interface name')

    cms = json.loads(subprocess.run([a.bridge, 'directory'], capture_output=True, check=True).stdout)
    cm = next(c for c in cms if c.split(':')[0] not in LEGACY)
    logs = Path(os.environ['LOCALAPPDATA']) / 'P2Middleware/logs'
    logs.mkdir(parents=True, exist_ok=True)
    log_path = logs / f'router-bridge-{datetime.datetime.now():%Y%m%d-%H%M%S}.jsonl'
    log = open(log_path, 'a', encoding='utf-8', buffering=1)
    qr = Path(tempfile.mkdtemp(prefix='p2-qr-')) / 'steam.svg'
    print(f'Upstream CM {cm}. Log: {log_path}')

    crossplay = not a.no_crossplay
    auth = serve = router = server = None
    try:
        server = qr_server(qr, a.port)
        auth = subprocess.Popen([a.bridge, 'authorize', '--qr', str(qr), cm],
                                stdout=subprocess.PIPE, stderr=log)
        serve = subprocess.Popen([a.bridge, 'serve', '--bind', f'{a.pc}:27017', '--peer', a.router,
                                  '--seconds', '86400', '--sessions', '1000',
                                  '--search-platform', 'any' if crossplay else 'preserve',
                                  '--search-distance', 'worldwide',
                                  '--advertise-platform', 'default' if crossplay else 'preserve'],
                                 stdin=auth.stdout, stderr=subprocess.PIPE, text=True)
        auth.stdout.close()
        webbrowser.open(f'http://127.0.0.1:{a.port}/')
        print('Approve the Steam sign-in in the browser window.')
        for line in serve.stderr:
            log.write(line)
            try:
                event = json.loads(line)
            except ValueError:
                continue
            name = event.get('event')
            if name == 'listening' and router is None:
                qr.unlink(missing_ok=True)
                body = rules(a.ps3, a.pc, a.router, a.lan_if)
                # The router removes its table when this channel reaches EOF or drops.
                router = ssh(a.ssh, f'trap "" HUP PIPE; nft delete table ip {TABLE} 2>/dev/null; '
                                    f'head -c {len(body)} | nft -f - && echo armed; '
                                    f'cat >/dev/null; nft delete table ip {TABLE}',
                             stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=False)
                router.stdin.write(body)
                router.stdin.flush()
                if router.stdout.readline().strip() != b'armed':
                    raise RuntimeError('router rules were not applied')
                print(f'READY. PS3 gateway stays {a.router}. Ctrl+C to stop.')
            elif name in ('ps3_connected', 'session_end', 'official_login_result', 'fatal', 'stopped',
                          'lobby_search_diagnostic', 'lobby_publication_diagnostic',
                          'lobby_search_policy', 'lobby_advertisement_policy'):
                print(line.rstrip())
        return serve.wait()
    except KeyboardInterrupt:
        return 130
    finally:
        for proc in (serve, auth):
            if proc is not None and proc.poll() is None:
                proc.kill()
        if router is not None:
            try:
                router.stdin.close()
                router.wait(timeout=10)
            except (OSError, subprocess.TimeoutExpired):
                router.kill()
        check = subprocess.run(['ssh', '-T', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=5', a.ssh,
                                f'nft delete table ip {TABLE} 2>/dev/null; nft list table ip {TABLE} >/dev/null 2>&1 && echo present || echo absent'],
                               capture_output=True, text=True)
        print('Router table', check.stdout.strip() or 'state unknown')
        if server is not None:
            server.shutdown()
        qr.unlink(missing_ok=True)
        log.close()


if __name__ == '__main__':
    sys.exit(main())
