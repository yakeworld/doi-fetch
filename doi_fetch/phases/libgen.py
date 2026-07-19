"""Phase 3: LibGen search + download"""


def download(doi: str, output: str, get, log) -> bool:
    """LibGen search by DOI -> edition page -> MD5 download"""
    import re
    enc = doi.replace("/", "%2F")
    search_url = f"https://libgen.li/index.php?req={enc}&columns%5B%5D=d"
    try:
        r = get(search_url, impersonate="chrome120", timeout=15)
        ids = re.findall(r"edition\.php\?id=(\d+)", r.text)
        if not ids:
            log("  NO_RESULT")
            return False

        # Get edition page
        r2 = get(f"https://libgen.li/edition.php?id={ids[0]}",
                 impersonate="chrome120", timeout=20, verify=False)
        for href in re.findall(r'href="([^"]*)"', r2.text):
            if "md5=" in href:
                url = "https://libgen.li" + href if href.startswith("/") else href
                r3 = get(url, impersonate="chrome120", timeout=20, verify=False)
                if r3.content[:5] == b"%PDF-":
                    open(output, "wb").write(r3.content)
                    log(f"  ✅ {len(r3.content)} bytes")
                    return True
                # Try alternate mirrors
                md5 = re.search(r"md5=([0-9a-f]{32})", url)
                if md5:
                    for base in ["https://libgen.li/main/",
                                 "https://library.lol/main/"]:
                        try:
                            r4 = get(base + md5.group(1),
                                     impersonate="chrome120", timeout=15,
                                     verify=False)
                            if r4.content[:5] == b"%PDF-":
                                open(output, "wb").write(r4.content)
                                log(f"  ✅ {len(r4.content)} bytes")
                                return True
                        except Exception:
                            pass
        log("  NO_DL")
    except Exception as e:
        log(f"  ERR {str(e)[:60]}")
    return False
