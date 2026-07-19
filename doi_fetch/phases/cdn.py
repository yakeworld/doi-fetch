"""Phase 1: bban.top CDN direct download"""

CDN_URL = "https://sci.bban.top/pdf/%s.pdf"


def download(doi: str, output: str, get, log) -> bool:
    """Try CDN direct download"""
    url = CDN_URL % doi.lower()
    try:
        r = get(url, impersonate="chrome120", timeout=15)
        if r.content[:5] == b"%PDF-":
            open(output, "wb").write(r.content)
            log(f"  ✅ {len(r.content)} bytes")
            return True
        log(f"  HTTP {r.status_code} / not PDF")
    except Exception as e:
        log(f"  ERR {str(e)[:60]}")
    return False
