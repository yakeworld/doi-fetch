# doi-fetch

多源 DOI → PDF 全文下载工具，4 层降级管线，Rust 重写版。

## 安装

```bash
cargo install --path .
# 或从 GitHub Releases 下载预编译二进制
```

## 用法

```bash
# 下载论文 PDF
doi-fetch 10.1016/j.jcrs.2019.04.024

# 指定输出路径
doi-fetch 10.1016/j.jcrs.2019.04.024 -o paper.pdf

# 无代理直连（不走 rproxy 轮换）
doi-fetch 10.1016/j.jcrs.2019.04.024 --no-proxy

# 指定代理池文件
doi-fetch 10.1016/j.jcrs.2019.04.024 --proxy-pool /tmp/proxies.txt
```

## 降级管线

```
Phase 1: bban.top CDN 直连
Phase 2: Sci-Hub frontend (sci-hub.vg iframe 提取)
Phase 3: LibGen 搜索 + 下载
Phase 4: Anna's Archive scidb (MD5 发现辅助)
```

下载后自动验证 PDF 魔数（`%PDF-` 5 字节）。

## 代理支持

- `--no-proxy`：直连下载
- 默认：通过 `rproxy exec` 代理轮换（6 次重试）
- 自定义代理池文件

## License

MIT
