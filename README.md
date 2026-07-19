# doi-fetch

DOI 全文 PDF 下载器。四层降级串联：

```
[1] bban.top CDN 直连
[2] Sci-Hub frontend (sci-hub.vg iframe)
[3] LibGen 搜索 + 下载
[4] Anna's Archive scidb (MD5 发现，辅助 LibGen)
```

## 安装

```bash
pip install git+https://github.com/yakeworld/doi-fetch.git

# 或带代理轮换支持
pip install "doi-fetch[proxy] @ git+https://github.com/yakeworld/doi-fetch.git"
```

## 用法

```bash
doi-fetch 10.1016/j.jcrs.2019.04.024
doi-fetch 10.1016/j.jcrs.2019.04.024 -o paper.pdf
doi-fetch 10.1007/s00521-025-11109-5 --no-proxy
doi-fetch 10.1038/s41586-019-1799-6 --timeout 30
```

## 依赖

- `curl-cffi` — Cloudflare TLS 指纹绕过
- `rproxy` (可选) — HTTP 代理池轮换，绕过 IP 限流

## 验证

所有下载的 PDF 自动验证 `%PDF-` magic number（5 字节）。

## License

MIT
