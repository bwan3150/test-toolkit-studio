# ADR-0008: 测试分层与放置——单测就地放,tests/ 只放黑盒 CLI 契约,真机走 e2e 脚本

- **状态**: 生效
- **日期**: 2026-08-12
- **关联**: PITFALLS P-12, AGENTS.md「测试」节

## 背景
用户要求"test 文件夹放单元测试和 e2e"。但 Rust 单测要访问 crate 私有项,
搬进 tests/ 就得把一堆 pub(crate) 开公开——得不偿失。

## 决策
三层,各归各位:
1. **单测 + 无设备集成**(FakeLlm/FakeDriver):`#[cfg(test)]` 就地放 src 内
   (drive_tests/repair_tests/tklib_tests…),`cargo test --no-default-features --lib`,pre-push 强制
2. **黑盒 CLI 契约**:`tests/cli.rs`,spawn 真二进制测 clap 装配/输出协议/两件套报错
   (--copilot 裸旗标那类问题只有这层测得到),`cargo test --no-default-features --test cli`,秒级
3. **真机 e2e**:`tests/e2e/*.sh`,需要设备/配置,CI 跑不了,需要时手动跑

## 理由与代价
不为"目录整齐"违背语言惯例。代价:测试分三处——AGENTS.md 命令表统一列出即可。
