跑一个 curated 侦察检查，快速拿到某一面的结论。verb 可选：
headers（安全响应头）/ fingerprint（技术指纹）/ cors（跨域）/ graphql（introspection）/
bundle（在某 URL 的 JS/文本里扫密钥）/ endpoints（探常见敏感路径）/ tls（明文跳转+HSTS）。
参数：verb、url。返回该检查命中的 findings 与探测计数；探测都自动落证据。
适合开局测绘，以及顺藤时快速验证某一类问题。
