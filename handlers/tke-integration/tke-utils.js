// TKE 工具函数
// 提供通用的 TKE 输出处理功能

/**
 * 从 TKE 的 stdout 输出中提取有效的 JSON
 *
 * 背景：TKE 可能在 stdout 中输出混合内容：
 * - 第一行/第一部分是有效的 JSON 响应
 * - 后面可能跟着 WARN/INFO 等日志行
 *
 * 例如：
 * {"success":true,"screenshot":"C:\\path\\to\\file.png"}
 * 2025-10-18T04:19:07.382454Z  WARN tke::utils::adb_manager: 清理临时 ADB 文件失败...
 *
 * 此函数会尝试智能提取有效的 JSON 部分
 *
 * @param {string} stdout - TKE 命令的标准输出
 * @returns {string} - 提取的 JSON 字符串
 * @throws {Error} - 如果无法提取有效的 JSON
 */
function extractJsonFromOutput(stdout) {
  if (!stdout || typeof stdout !== 'string') {
    throw new Error('TKE 输出为空或无效');
  }

  // 1. 先尝试直接解析 trim 后的完整输出（最常见的情况）
  const trimmedOutput = stdout.trim();
  try {
    JSON.parse(trimmedOutput);
    return trimmedOutput;
  } catch (firstError) {
    // 完整输出不是有效 JSON，继续尝试提取
  }

  // 2. 检查是否是多行输出
  const lines = stdout.split('\n');
  if (lines.length === 1) {
    // 单行但解析失败，直接抛出错误
    throw new Error(`TKE 输出不是有效的 JSON: ${trimmedOutput}`);
  }

  // 3. 尝试多行输出的处理策略

  // 策略 A: 尝试提取第一行（最常见的 JSON + 日志的情况）
  const firstLine = lines[0].trim();
  if (firstLine) {
    try {
      JSON.parse(firstLine);
      return firstLine;
    } catch (e) {
      // 第一行不是有效 JSON，继续尝试
    }
  }

  // 策略 B: 尝试找到第一个以 { 或 [ 开头的行（可能 JSON 不在第一行）
  for (const line of lines) {
    const trimmedLine = line.trim();
    if (trimmedLine.startsWith('{') || trimmedLine.startsWith('[')) {
      try {
        JSON.parse(trimmedLine);
        return trimmedLine;
      } catch (e) {
        // 这一行虽然看起来像 JSON 但解析失败，继续找
        continue;
      }
    }
  }

  // 策略 C: 尝试提取所有 JSON 行（处理多行 JSON 的情况）
  // 找到第一个 { 或 [ 开始，到对应的 } 或 ] 结束
  const jsonStartIndex = stdout.search(/[{\[]/);
  if (jsonStartIndex !== -1) {
    let braceCount = 0;
    let bracketCount = 0;
    let inString = false;
    let escapeNext = false;
    let jsonEndIndex = -1;

    for (let i = jsonStartIndex; i < stdout.length; i++) {
      const char = stdout[i];

      // 处理字符串转义
      if (escapeNext) {
        escapeNext = false;
        continue;
      }
      if (char === '\\') {
        escapeNext = true;
        continue;
      }

      // 处理字符串边界
      if (char === '"') {
        inString = !inString;
        continue;
      }

      // 只在非字符串内部计数括号
      if (!inString) {
        if (char === '{') braceCount++;
        if (char === '}') braceCount--;
        if (char === '[') bracketCount++;
        if (char === ']') bracketCount--;

        // 当所有括号都闭合时，找到了 JSON 的结束位置
        if (braceCount === 0 && bracketCount === 0) {
          jsonEndIndex = i + 1;
          break;
        }
      }
    }

    if (jsonEndIndex !== -1) {
      const extractedJson = stdout.substring(jsonStartIndex, jsonEndIndex).trim();
      try {
        JSON.parse(extractedJson);
        return extractedJson;
      } catch (e) {
        // 提取的内容仍然不是有效 JSON
      }
    }
  }

  // 所有策略都失败，抛出错误并显示原始输出
  throw new Error(`TKE 输出不是有效的 JSON: ${trimmedOutput}`);
}

module.exports = {
  extractJsonFromOutput
};
