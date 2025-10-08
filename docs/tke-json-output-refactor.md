# TKE JSON 输出重构总结

## 重构目标

统一 `toolkit-engine/src` 中所有子模块的 JSON 输出，确保：
1. 所有子模块（除了 `src/runner` 和 `src/adb`）的 JSON 输出必须经过 `src/utils/json_output.rs`
2. 禁止子模块自己构建 `serde_json::json!` 或直接使用 `println!` 输出 JSON
3. 建立统一的 JSON 输出协议和方法

## 重构内容

### 1. 增强 `src/utils/json_output.rs`

新增以下统一协议方法：

#### 核心方法
- `success<T: Serialize>(data: T)` - 输出成功 JSON 并退出（退出码 0）
- `error(message: impl AsRef<str>) -> !` - 输出错误 JSON 并退出（退出码 1）
- `print<T: Serialize>(data: T)` - 输出 JSON 但不退出
- `print_error(message: impl AsRef<str>)` - 输出错误 JSON 但不退出

#### 辅助方法
- `success_with<T: Serialize>(data: T)` - 自动包装 `success: true` 并退出
- `unwrap_or_exit<T, E>(result: Result<T, E>) -> T` - Result 转换，失败则输出错误并退出
- `build_success<T: Serialize>(data: T) -> Value` - 构建成功 JSON 对象（不输出）
- `build_error(message: impl AsRef<str>) -> Value` - 构建错误 JSON 对象（不输出）
- `print_raw(json_str: &str)` - 直接打印 JSON 字符串（不退出）
- `success_raw(json_str: &str) -> !` - 打印 JSON 字符串并成功退出
- `error_raw(json_str: &str) -> !` - 打印 JSON 字符串并失败退出

### 2. 重构各子模块

#### `src/recognizer/image.rs`
- ✅ 替换所有手动构建的 `serde_json::json!` 为 `JsonOutput::print_error()`
- ✅ 替换直接 `println!` 为 `JsonOutput::print_raw()`

#### `src/main.rs`
重构了所有 handler 函数中的 JSON 输出：

- ✅ `handle_controller_commands()` - 所有命令输出使用 `JsonOutput::print()`
- ✅ `handle_fetcher_commands()` - 所有命令输出使用 `JsonOutput::print()` 或 `JsonOutput::print_raw()`
- ✅ `handle_recognizer_commands()` - 保持使用 `JsonOutput::success()` 和 `JsonOutput::error()`
- ✅ `handle_parser_commands()` - 所有命令输出使用 `JsonOutput::print()`
- ✅ `handle_run_commands()` - Content 和 Step 命令使用 `JsonOutput::print()`
- ✅ `handle_ocr_command()` - 使用 `JsonOutput::print()`

### 3. 未修改的模块

以下模块按要求保持不变：
- `src/runner/mod.rs` - 按需求排除
- `src/adb/mod.rs` - 按需求排除
- `src/controller/mod.rs` - 无自定义 JSON 构建
- `src/fetcher/mod.rs` - 无自定义 JSON 构建
- `src/recognizer/mod.rs` - 无自定义 JSON 构建
- `src/recognizer/text.rs` - 无自定义 JSON 构建
- `src/recognizer/xml.rs` - 无自定义 JSON 构建
- `src/parser/mod.rs` - 无自定义 JSON 构建
- `src/ocr/mod.rs` - 无自定义 JSON 构建

## 验证结果

### 构建状态
✅ 构建成功（仅有 9 个警告，均为未使用的导入和字段）

### JSON 输出检查
- ✅ 无残留的 `println!.*serde_json` 模式
- ✅ main.rs 中的 `println!` 仅用于用户友好输出（非 JSON 场景）
- ✅ 所有 JSON 输出统一通过 `JsonOutput` 模块

## 统一协议使用示例

### 成功输出（退出）
```rust
JsonOutput::success(serde_json::json!({
    "success": true,
    "x": 100,
    "y": 200
}));
```

### 错误输出（退出）
```rust
JsonOutput::error("未找到匹配点");
```

### 输出但不退出
```rust
JsonOutput::print(serde_json::json!({
    "devices": devices
}));
```

### 输出错误但不退出
```rust
JsonOutput::print_error("找不到 tke-opencv 模块");
```

### 直接输出 JSON 字符串
```rust
JsonOutput::print_raw(&json_string);
```

## 重构收益

1. **统一性**：所有 JSON 输出格式统一，便于前端解析
2. **可维护性**：JSON 输出逻辑集中管理，修改更方便
3. **一致性**：错误处理和成功输出遵循统一模式
4. **清晰性**：代码意图明确，`JsonOutput::success()` vs `JsonOutput::error()` 一目了然
5. **安全性**：减少手动序列化错误的风险

## 后续建议

1. 清理构建警告（运行 `cargo fix` 清理未使用的导入）
2. 考虑为 `JsonOutput` 添加单元测试
3. 在团队中推广统一协议的使用规范
4. 可以考虑为复杂的 JSON 结构创建专门的构建函数
