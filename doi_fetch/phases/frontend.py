"""Phase 2: Sci-Hub frontend (sci-hub.vg iframe extraction)"""

FRONTEND_URL = "https://sci-hub.vg/%s"


def download(doi: str, output: str, get, log) -> bool:
    """Try frontend -> iframe -> CDN download"""
    import re
    url = FRONTEND_URL % doi
    try:
        r = get(url, impersonate="chrome120", timeout=15)
        m = re.search(r'src="([^"]*\.pdf[^"]*)"', r.text)
        if not m:
            log("  NO_IFRAME")
            return False
        pdf_url = m.group(1).split("#")[0]
        if pdf_url.startswith("//"):
            pdf_url = "https:" + pdf_url
        r2 = get(pdf_url, impersonate="chrome120", timeout=20)
        if r2.content[:5] == b"%PDF-":
            open(output, "wb").write(r2.content)
            log(f"  ✅ {len(r2.content)} bytes")
            return True
        log("  NOT_PDF")
    except Exception as e:
        log(f"  ERR {str(e)[:60]}")
    return False
