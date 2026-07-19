"""Phase 4: Anna's Archive scidb (MD5 discovery only, no download)"""

DOMAINS = ["annas-archive.gd", "annas-archive.gl"]


def download(doi: str, output: str, get, log) -> bool:
    """Discover MD5 via AA scidb (slow_download blocked by DDoS-Guard)"""
    import re
    for dom in DOMAINS:
        try:
            r = get(f"https://{dom}/scidb/{doi}/",
                    impersonate="chrome120", timeout=15)
            md5s = re.findall(r"md5/([0-9a-f]{32})", r.text)
            if md5s:
                log(f"  🔍 MD5={md5s[0]} (use LibGen to download)")
                return False  # MD5 found but can't download from AA
        except Exception:
            pass
    log("  NONE")
    return False
