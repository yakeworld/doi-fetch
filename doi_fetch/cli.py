"""doi-fetch: multi-source DOI full-text PDF downloader.

Cascade strategy:
  [1] bban.top CDN direct
  [2] Sci-Hub frontend (sci-hub.vg iframe)
  [3] LibGen search + download
  [4] Anna's Archive scidb (MD5 discovery)
"""

import argparse
import os
import sys
import time
from functools import partial

from doi_fetch.phases import cdn, frontend, libgen, annas

PHASES = [
    ("bban.top CDN", cdn.download),
    ("Sci-Hub frontend", frontend.download),
    ("LibGen", libgen.download),
    ("Anna's Archive", annas.download),
]


def _make_http(proxy_pool=None, retries=6):
    """Create HTTP GET function with optional rproxy wrapper."""
    if proxy_pool and os.path.exists(proxy_pool):
        import subprocess

        def get(url, **kwargs):
            timeout = kwargs.pop("timeout", 15)
            code = (
                "import sys; from curl_cffi import requests as cr; "
                f"r=cr.get({url!r}, impersonate='chrome120', timeout={timeout}); "
                "sys.stdout.buffer.write(r.content)"
            )
            try:
                r = subprocess.run(
                    ["rproxy", "exec", "-i", proxy_pool, "-r", str(retries), "--",
                     sys.executable, "-c", code],
                    capture_output=True, timeout=timeout + 30,
                )
                content = r.stdout
                # Build a response-like object
                class Resp:
                    pass
                resp = Resp()
                resp.content = content
                resp.status_code = 0 if content else 503
                return resp
            except subprocess.TimeoutExpired:
                class Resp:
                    pass
                resp = Resp()
                resp.content = b""
                resp.status_code = 504
                return resp
    else:
        from curl_cffi import requests as cr

        def get(url, **kwargs):
            return cr.get(url, impersonate="chrome120", **kwargs)
    return get


def log(msg):
    print(msg, end="", flush=True)


def main():
    ap = argparse.ArgumentParser(
        description="DOI full-text PDF downloader",
        epilog="Cascade: bban.top CDN → Sci-Hub → LibGen → Anna's Archive",
    )
    ap.add_argument("doi", help="DOI to download (e.g. 10.1016/j.jcrs.2019.04.024)")
    ap.add_argument("-o", "--output", help="Output PDF path (default: {doi}.pdf)")
    ap.add_argument("--no-proxy", action="store_true",
                    help="Direct download without rproxy proxy rotation")
    ap.add_argument("--proxy-pool", default="/tmp/proxy_pool.txt",
                    help="Proxy pool file for rproxy (default: /tmp/proxy_pool.txt)")
    ap.add_argument("--timeout", type=int, default=120,
                    help="Per-phase timeout in seconds")
    args = ap.parse_args()

    doi = args.doi
    out = args.output or doi.replace("/", "_") + ".pdf"
    proxy_pool = None if args.no_proxy else args.proxy_pool

    get = _make_http(proxy_pool=proxy_pool)

    print(f"📄 {doi}")
    print(f"   → {out}")

    # Skip if already exists
    if os.path.exists(out):
        with open(out, "rb") as f:
            if f.read(5) == b"%PDF-":
                print(f"   ✅ 已有: {os.path.getsize(out)} bytes")
                return

    for name, phase_fn in PHASES:
        print(f"   [{name}]", end=" ", flush=True)
        if phase_fn(doi, out, get, log):
            return
        time.sleep(1)

    print("   ❌ 全部失败")


if __name__ == "__main__":
    main()
